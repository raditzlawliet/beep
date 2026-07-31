//! Unified execution engine.
//!
//! All frontends (GUI, CLI, TUI) call `engine::execute()`
//! which runs the full pipeline:
//! parse -> pre-script -> apply overrides -> compile -> execute -> post-script -> record.

use std::collections::HashMap;

use crate::compiler::compile_with_ctx;
use crate::context::ExecutionContext;
use crate::exec::http::HttpResult;
use crate::exec::scripts::{run_post_script, run_pre_script};
use crate::http_parser::{self, ParsedRequest};

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// What the caller provides to the engine.
pub enum ExecuteInput {
    /// Raw .http file text.
    Raw(String),
    /// Already parsed request (from GUI form or code tab).
    Parsed(ParsedRequest),
}

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------

/// Execute a request through the full pipeline.
pub async fn execute(
    input: ExecuteInput,
    ctx: &mut ExecutionContext,
) -> Result<HttpResult, String> {
    // Step 1: Parse
    let mut parsed = match input {
        ExecuteInput::Raw(ref src) => {
            let file = http_parser::parse(src);
            file.requests
                .first()
                .cloned()
                .ok_or_else(|| "No request found in input".to_string())?
        }
        ExecuteInput::Parsed(p) => p,
    };

    // Step 2: Clear request vars + run pre-request script
    ctx.clear_request_vars();

    let mut script_error: Option<String> = None;

    if let Some(ref pre_code) = parsed.pre_script {
        let req_headers = collect_headers(&parsed.headers);
        let request_body = parsed.body.as_deref();

        let (out, overrides) = run_pre_script(
            pre_code,
            &mut ctx.client_vars,
            &mut ctx.request_vars,
            &parsed.url,
            &parsed.method,
            &req_headers,
            request_body,
        );

        if let Some(err) = out.error {
            script_error = Some(format!("Pre-script error: {err}"));
        }

        // Apply request overrides from script — these do NOT affect the UI form.
        if let Some(url) = overrides.url {
            parsed.url = url;
        }
        if let Some(method) = overrides.method {
            parsed.method = method;
        }
        if let Some(body) = overrides.body {
            parsed.body = Some(body);
        }
        if !overrides.headers.is_empty() {
            // Merge: add new headers, update existing ones with matching key
            for (key, value) in overrides.headers {
                let key_lower = key.to_lowercase();
                if let Some(existing) = parsed
                    .headers
                    .iter_mut()
                    .find(|h| h.key.to_lowercase() == key_lower)
                {
                    existing.value = value;
                } else {
                    parsed.headers.push(crate::http_parser::ParsedHeaderField {
                        key,
                        value,
                        enabled: true,
                        auto: false,
                    });
                }
            }
        }
        if !overrides.deleted_headers.is_empty() {
            parsed.headers.retain(|h| {
                !overrides
                    .deleted_headers
                    .iter()
                    .any(|d| h.key.to_lowercase() == d.to_lowercase())
            });
        }
    }

    // Step 3: Resolve variables + compile
    let executable = compile_with_ctx(&parsed, ctx)?;

    // Step 4: Execute
    let executor = crate::exec::http::HttpExecutor::new();
    let result = executor.execute(&executable).await?;

    // Step 5: Run post-request script
    if let Some(ref post_code) = parsed.post_script {
        let req_headers = collect_headers(&parsed.headers);
        let request_body = parsed.body.as_deref();
        let mut resp_headers = HashMap::new();
        for (k, v) in &result.response.headers {
            resp_headers.insert(k.to_lowercase(), v.clone());
        }

        let resp_size_headers = result.response.size.headers;
        let resp_size_body = result.response.size.body;

        let out = run_post_script(
            post_code,
            &mut ctx.client_vars,
            &mut ctx.request_vars,
            &parsed.url,
            &parsed.method,
            &req_headers,
            request_body,
            result.response.status as u16,
            &result.response.body,
            &resp_headers,
            result.response.elapsed_ms as u64,
            resp_size_headers,
            resp_size_body,
        );

        if let Some(err) = out.error {
            script_error = Some(match script_error {
                Some(prev) => format!("{prev}; Post-script error: {err}"),
                None => format!("Post-script error: {err}"),
            });
        }
    }

    // Step 6: Record to history
    if let Ok(mut history) = ctx.history.lock() {
        history.add_parsed(
            parsed,
            executable,
            Some(result.clone()),
            script_error,
            None,
        );
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn collect_headers(
    headers: &[crate::http_parser::ParsedHeaderField],
) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for h in headers {
        if h.enabled && !h.key.is_empty() {
            map.insert(h.key.to_lowercase(), h.value.clone());
        }
    }
    map
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_input_parsed() {
        let input = ExecuteInput::Parsed(ParsedRequest::default());
        match input {
            ExecuteInput::Parsed(_) => {}
            _ => panic!("expected Parsed"),
        }
    }

    #[test]
    fn test_execute_input_raw() {
        let input = ExecuteInput::Raw("GET https://example.com".to_string());
        match input {
            ExecuteInput::Raw(_) => {}
            _ => panic!("expected Raw"),
        }
    }

    #[test]
    fn test_collect_headers() {
        use crate::http_parser::ParsedHeaderField;
        let headers = vec![
            ParsedHeaderField {
                key: "Content-Type".into(),
                value: "application/json".into(),
                enabled: true,
                auto: false,
            },
            ParsedHeaderField {
                key: "Accept".into(),
                value: "*/*".into(),
                enabled: true,
                auto: true,
            },
            ParsedHeaderField {
                key: "X-Debug".into(),
                value: "true".into(),
                enabled: false,
                auto: false,
            },
        ];
        let map = collect_headers(&headers);
        assert_eq!(map.get("content-type").map(|s| s.as_str()), Some("application/json"));
        assert!(map.get("accept").is_some());
        assert!(map.get("x-debug").is_none());
    }
}
