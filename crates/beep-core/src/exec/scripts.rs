//! JavaScript script execution via QuickJS.
//!
//! Runs pre-request and post-request scripts defined in .http files.
//! Scripts execute in a sandboxed QuickJS context following Bruno's API conventions.
//!
//! Pre-script globals:
//! - `client`      - session-level API (vars, test, assert)
//! - `req`         - request CRUD (read + mutate before send)
//! - `console`     - log capture
//!
//! Post-script globals:
//! - Same as pre-script, plus `res` with status/body/headers/time/size/statusText

use crate::context::{VarStore, resolve_template};
use crate::http_parser::ParsedFileVariable;
use rquickjs::CatchResultExt;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Console output captured during script execution.
#[derive(Debug, Clone, Default)]
pub struct ScriptOutput {
    /// Console log messages.
    pub console: Vec<String>,
    /// Error message if the script threw or failed to compile.
    pub error: Option<String>,
}

/// Request mutations made by a pre-request script.
/// Applied after script execution, before compilation.
/// Does NOT affect the original request - only the actual sent request.
#[derive(Debug, Clone, Default)]
pub struct RequestOverrides {
    /// Override the request URL.
    pub url: Option<String>,
    /// Override the HTTP method.
    pub method: Option<String>,
    /// Added/overridden headers (merged with existing).
    pub headers: HashMap<String, String>,
    /// Headers to delete (case-insensitive names).
    pub deleted_headers: Vec<String>,
    /// Override the request body (raw string or serialized JSON).
    pub body: Option<String>,
}

// ---------------------------------------------------------------------------
// Internal state
// ---------------------------------------------------------------------------

struct ScriptContext {
    client_vars: Rc<RefCell<VarStore>>,
    request_vars: Rc<RefCell<VarStore>>,
    request_overrides: Rc<RefCell<RequestOverrides>>,
    file_vars: Vec<ParsedFileVariable>,
    request_url: String,
    request_method: String,
    request_headers: HashMap<String, String>,
    request_body: Option<String>,
    response_status: Option<u16>,
    response_body: Option<String>,
    response_headers: Option<HashMap<String, String>>,
    response_time_ms: Option<u64>,
    response_size_headers: Option<u64>,
    response_size_body: Option<u64>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run a pre-request script.
///
/// Returns the console output and any request overrides the script requested.
/// Mutations to `client_vars` and `request_vars` are applied in-place.
pub fn run_pre_script(
    code: &str,
    client_vars: &mut VarStore,
    request_vars: &mut VarStore,
    file_vars: &[ParsedFileVariable],
    request_url: &str,
    request_method: &str,
    request_headers: &HashMap<String, String>,
    request_body: Option<&str>,
) -> (ScriptOutput, RequestOverrides) {
    let cv = Rc::new(RefCell::new(client_vars.clone()));
    let rv = Rc::new(RefCell::new(request_vars.clone()));
    let ro = Rc::new(RefCell::new(RequestOverrides::default()));

    let sc = Rc::new(ScriptContext {
        client_vars: cv.clone(),
        request_vars: rv.clone(),
        request_overrides: ro.clone(),
        file_vars: file_vars.to_vec(),
        request_url: request_url.to_string(),
        request_method: request_method.to_string(),
        request_headers: request_headers.clone(),
        request_body: request_body.map(|s| s.to_string()),
        response_status: None,
        response_body: None,
        response_headers: None,
        response_time_ms: None,
        response_size_headers: None,
        response_size_body: None,
    });

    let result = run_script(sc, code);

    // Always propagate mutations via independently cloned handles
    *client_vars = cv.borrow().clone();
    *request_vars = rv.borrow().clone();
    (result, ro.borrow().clone())
}

/// Run a post-request script.
pub fn run_post_script(
    code: &str,
    client_vars: &mut VarStore,
    request_vars: &mut VarStore,
    file_vars: &[ParsedFileVariable],
    request_url: &str,
    request_method: &str,
    request_headers: &HashMap<String, String>,
    request_body: Option<&str>,
    response_status: u16,
    response_body: &str,
    response_headers: &HashMap<String, String>,
    response_time_ms: u64,
    response_size_headers: u64,
    response_size_body: u64,
) -> ScriptOutput {
    let cv = Rc::new(RefCell::new(client_vars.clone()));
    let rv = Rc::new(RefCell::new(request_vars.clone()));

    let sc = Rc::new(ScriptContext {
        client_vars: cv.clone(),
        request_vars: rv.clone(),
        request_overrides: Rc::new(RefCell::new(RequestOverrides::default())),
        file_vars: file_vars.to_vec(),
        request_url: request_url.to_string(),
        request_method: request_method.to_string(),
        request_headers: request_headers.clone(),
        request_body: request_body.map(|s| s.to_string()),
        response_status: Some(response_status),
        response_body: Some(response_body.to_string()),
        response_headers: Some(response_headers.clone()),
        response_time_ms: Some(response_time_ms),
        response_size_headers: Some(response_size_headers),
        response_size_body: Some(response_size_body),
    });

    let result = run_script(sc, code);

    *client_vars = cv.borrow().clone();
    *request_vars = rv.borrow().clone();

    result
}

// ---------------------------------------------------------------------------
// Core script runner
// ---------------------------------------------------------------------------

fn run_script(sc: Rc<ScriptContext>, code: &str) -> ScriptOutput {
    let console_logs: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let script_err: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let rt = match rquickjs::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            return ScriptOutput {
                console: vec![],
                error: Some(format!("Failed to create JS runtime: {e}")),
            };
        }
    };

    // Apply runtime limits before creating the context
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    rt.set_memory_limit(64 * 1024 * 1024);
    rt.set_interrupt_handler(Some(Box::new(move || std::time::Instant::now() > deadline)));

    let ctx = match rquickjs::Context::full(&rt) {
        Ok(ctx) => ctx,
        Err(e) => {
            return ScriptOutput {
                console: vec![],
                error: Some(format!("Failed to create JS context: {e}")),
            };
        }
    };

    let setup_result: rquickjs::Result<()> = ctx.with(|c| {
        // --- console ---
        {
            let logs_log = console_logs.clone();
            let obj = rquickjs::Object::new(c.clone())?;
            let log_fn = rquickjs::Function::new(
                c.clone(),
                move |args: rquickjs::function::Rest<rquickjs::Coerced<String>>| {
                    let line: Vec<String> = args.iter().map(|s| s.to_string()).collect();
                    logs_log.borrow_mut().push(line.join(" "));
                },
            )?;
            obj.set("log", log_fn)?;

            let logs_err = console_logs.clone();
            let error_fn = rquickjs::Function::new(
                c.clone(),
                move |args: rquickjs::function::Rest<rquickjs::Coerced<String>>| {
                    let line: Vec<String> = args.iter().map(|s| s.to_string()).collect();
                    logs_err
                        .borrow_mut()
                        .push(format!("[error] {}", line.join(" ")));
                },
            )?;
            obj.set("error", error_fn)?;
            c.globals().set("console", obj)?;
        }

        // --- btoa ---
        {
            let _btoa = rquickjs::Function::new(c.clone(), {
                move |s: String| {
                    use crate::exec::http::base64_encode;
                    base64_encode(s.as_bytes())
                }
            })?;
            c.globals().set("btoa", _btoa)?;
        }

        // --- client ---
        {
            let client = rquickjs::Object::new(c.clone())?;

            let vars_obj = rquickjs::Object::new(c.clone())?;

            let sc_cvg = sc.clone();
            vars_obj.set(
                "get",
                rquickjs::Function::new(c.clone(), move |key: String| -> Option<String> {
                    let vars = sc_cvg.client_vars.borrow();
                    vars.get(&key).map(|s| s.to_string())
                })?,
            )?;

            let sc_cvs = sc.clone();
            vars_obj.set(
                "set",
                rquickjs::Function::new(c.clone(), move |key: String, value: String| {
                    sc_cvs.client_vars.borrow_mut().set(key, value);
                })?,
            )?;

            let sc_cvr = sc.clone();
            vars_obj.set(
                "reset",
                rquickjs::Function::new(c.clone(), move |key: String| {
                    sc_cvr.client_vars.borrow_mut().remove(&key);
                })?,
            )?;

            client.set("vars", vars_obj)?;

            // client.test and client.assert intentionally excluded until implemented

            c.globals().set("client", client)?;
        }

        // --- req ---
        {
            let req = rquickjs::Object::new(c.clone())?;

            // -- req.vars (get/set) --
            let vars_obj = rquickjs::Object::new(c.clone())?;

            let sc_rvg = sc.clone();
            vars_obj.set(
                "get",
                rquickjs::Function::new(c.clone(), move |key: String| -> Option<String> {
                    let vars = sc_rvg.request_vars.borrow();
                    vars.get(&key).map(|s| s.to_string())
                })?,
            )?;

            let sc_rvs = sc.clone();
            vars_obj.set(
                "set",
                rquickjs::Function::new(c.clone(), move |key: String, value: String| {
                    sc_rvs.request_vars.borrow_mut().set(key, value);
                })?,
            )?;

            req.set("vars", vars_obj)?;

            // -- req.getUrl() --
            let sc_gu = sc.clone();
            req.set(
                "getUrl",
                rquickjs::Function::new(c.clone(), move || sc_gu.request_url.clone())?,
            )?;

            let is_pre_script = sc.response_status.is_none();

            // -- req.setUrl(url) --
            if is_pre_script {
                let sc_su = sc.clone();
                req.set(
                    "setUrl",
                    rquickjs::Function::new(c.clone(), move |url: String| {
                        sc_su.request_overrides.borrow_mut().url = Some(url);
                    })?,
                )?;
            }

            // -- req.getMethod() --
            let sc_gm = sc.clone();
            req.set(
                "getMethod",
                rquickjs::Function::new(c.clone(), move || sc_gm.request_method.clone())?,
            )?;

            // -- req.setMethod(method) --
            if is_pre_script {
                let sc_sm = sc.clone();
                req.set(
                    "setMethod",
                    rquickjs::Function::new(c.clone(), move |method: String| {
                        sc_sm.request_overrides.borrow_mut().method = Some(method.to_uppercase());
                    })?,
                )?;
            }

            // -- req.getHeaders() --
            // -- req.getHeader(name) --
            let sc_gh = sc.clone();
            let get_header =
                rquickjs::Function::new(c.clone(), move |name: String| -> Option<String> {
                    let name_lower = name.to_lowercase();
                    // Check overrides first
                    {
                        let overrides = sc_gh.request_overrides.borrow();
                        // Check deleted headers
                        if overrides
                            .deleted_headers
                            .iter()
                            .any(|d| d.to_lowercase() == name_lower)
                        {
                            return None;
                        }
                        // Case-insensitive override lookup
                        for (k, v) in &overrides.headers {
                            if k.to_lowercase() == name_lower {
                                return Some(v.clone());
                            }
                        }
                    }
                    // Fall back to original headers
                    for (k, v) in &sc_gh.request_headers {
                        if k.to_lowercase() == name_lower {
                            return Some(v.clone());
                        }
                    }
                    None
                })?;
            req.set("getHeader", get_header)?;

            // -- req.setHeader(key, value) --
            if is_pre_script {
                let sc_sh = sc.clone();
                req.set(
                    "setHeader",
                    rquickjs::Function::new(c.clone(), move |key: String, value: String| {
                        let key_lower = key.to_lowercase();
                        let mut ov = sc_sh.request_overrides.borrow_mut();
                        ov.headers.insert(key, value);
                        // Cancel any previous delete of this header
                        ov.deleted_headers.retain(|d| d.to_lowercase() != key_lower);
                    })?,
                )?;
            }

            // -- req.deleteHeader(name) --
            if is_pre_script {
                let sc_dh = sc.clone();
                req.set(
                    "deleteHeader",
                    rquickjs::Function::new(c.clone(), move |name: String| {
                        let name_lower = name.to_lowercase();
                        let mut ov = sc_dh.request_overrides.borrow_mut();
                        ov.deleted_headers.push(name);
                        // Remove from headers if previously set
                        ov.headers.retain(|k, _| k.to_lowercase() != name_lower);
                    })?,
                )?;
            }

            // -- req.deleteHeaders(names) --
            if is_pre_script {
                let sc_dhs = sc.clone();
                req.set(
                    "deleteHeaders",
                    rquickjs::Function::new(c.clone(), move |names: rquickjs::Value| {
                        let mut parsed: Vec<String> = Vec::new();
                        if let Some(arr) = names.as_array() {
                            for item in arr.iter::<rquickjs::Value>() {
                                if let Ok(v) = item {
                                    if let Some(s) = v.as_string() {
                                        let s = s.to_string().unwrap_or_default();
                                        for n in s.split(',') {
                                            let trimmed = n.trim().to_string();
                                            if !trimmed.is_empty() {
                                                parsed.push(trimmed);
                                            }
                                        }
                                    }
                                }
                            }
                        } else if let Some(s) = names.as_string() {
                            let s = s.to_string().unwrap_or_default();
                            for n in s.split(',') {
                                let trimmed = n.trim().to_string();
                                if !trimmed.is_empty() {
                                    parsed.push(trimmed);
                                }
                            }
                        }
                        let mut ov = sc_dhs.request_overrides.borrow_mut();
                        for n in parsed {
                            let n_lower = n.to_lowercase();
                            ov.deleted_headers.push(n);
                            // Remove from headers if previously set (match deleteHeader)
                            ov.headers.retain(|k, _| k.to_lowercase() != n_lower);
                        }
                    })?,
                )?;
            }

            // -- req.setBody(body) --
            if is_pre_script {
                let sc_sb = sc.clone();
                req.set(
                    "setBody",
                    rquickjs::Function::new(c.clone(), move |body: String| {
                        sc_sb.request_overrides.borrow_mut().body = Some(body);
                    })?,
                )?;
            }

            // -- Read-only request convenience properties (plain strings, override-aware) --
            req.set("url", sc.request_url.clone())?;
            req.set("method", sc.request_method.clone())?;

            let headers_obj = rquickjs::Object::new(c.clone())?;
            for (k, v) in &sc.request_headers {
                let _ = headers_obj.set(k.as_str(), v.as_str());
            }
            req.set("headers", headers_obj)?;

            if let Some(ref body) = sc.request_body {
                req.set("body", body.as_str())?;
            } else {
                req.set("body", rquickjs::Value::new_null(c.clone()))?;
            }

            c.globals().set("req", req)?;
        }

        // --- res (post-script only) ---
        if sc.response_status.is_some() {
            let res = rquickjs::Object::new(c.clone())?;

            res.set("status", sc.response_status.unwrap() as i32)?;

            // Auto-parse JSON body only when Content-Type is json
            let raw_body = sc.response_body.clone().unwrap_or_default();
            let is_json = sc.response_headers.as_ref().is_some_and(|h| {
                h.iter()
                    .any(|(k, v)| k == "content-type" && v.to_lowercase().contains("json"))
            });
            if is_json {
                let parsed: Result<rquickjs::Value, _> = c.json_parse(raw_body.clone());
                if let Ok(p) = parsed {
                    res.set("body", p)?;
                } else {
                    res.set("body", raw_body.as_str())?;
                }
            } else {
                res.set("body", raw_body.as_str())?;
            }

            let status_text = status_text(sc.response_status.unwrap());
            res.set("statusText", status_text)?;

            let headers_obj = rquickjs::Object::new(c.clone())?;
            if let Some(ref headers) = sc.response_headers {
                for (k, v) in headers {
                    let _ = headers_obj.set(k.as_str(), v.as_str());
                }
            }
            res.set("headers", headers_obj)?;

            res.set("time", sc.response_time_ms.unwrap_or(0) as f64)?;

            let size_obj = rquickjs::Object::new(c.clone())?;
            size_obj.set("headers", sc.response_size_headers.unwrap_or(0) as f64)?;
            size_obj.set("body", sc.response_size_body.unwrap_or(0) as f64)?;
            res.set("size", size_obj)?;

            c.globals().set("res", res)?;
        }

        // --- beep (utility namespace) ---
        {
            let beep = rquickjs::Object::new(c.clone())?;

            let sc_interp = sc.clone();
            beep.set(
                "interpolate",
                rquickjs::Function::new(c.clone(), move |input: String| {
                    let client = sc_interp.client_vars.borrow();
                    let request = sc_interp.request_vars.borrow();
                    resolve_template(&input, &request, &client, &sc_interp.file_vars)
                })?,
            )?;

            c.globals().set("beep", beep)?;
        }

        // --- Execute the script ---
        match c.eval::<rquickjs::Value, _>(code).catch(&c) {
            Ok(_) => {}
            Err(e) => {
                let msg = e.to_string();
                console_logs
                    .borrow_mut()
                    .push(format!("Script error: {msg}"));
                *script_err.borrow_mut() = Some(msg);
            }
        }

        Ok(())
    });

    let logs = console_logs.borrow().clone();
    let error = setup_result
        .err()
        .map(|e| format!("Failed to set up script globals: {e}"))
        .or_else(|| script_err.borrow().clone());
    ScriptOutput {
        console: logs,
        error,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn status_text(code: u16) -> &'static str {
    match code {
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        410 => "Gone",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "",
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pre_script_sets_request_vars() {
        let mut client = VarStore::new();
        let mut request = VarStore::new();
        let code = r#"
            req.vars.set("timestamp", "2024-01-01");
            console.log("pre-script ran");
        "#;
        let (out, _ov) = run_pre_script(
            code,
            &mut client,
            &mut request,
            &[],
            "https://api.example.com/users",
            "GET",
            &HashMap::new(),
            None,
        );
        assert_eq!(request.get("timestamp"), Some("2024-01-01"));
        assert!(out.console.iter().any(|l| l.contains("pre-script ran")));
    }

    #[test]
    fn test_req_get_url_and_method() {
        let mut client = VarStore::new();
        let mut request = VarStore::new();
        let code = r#"
            console.log(req.getUrl());
            console.log(req.getMethod());
        "#;
        let (out, _ov) = run_pre_script(
            code,
            &mut client,
            &mut request,
            &[],
            "https://api.example.com/users",
            "GET",
            &HashMap::new(),
            None,
        );
        assert!(
            out.console
                .iter()
                .any(|l| l.contains("https://api.example.com/users"))
        );
        assert!(out.console.iter().any(|l| l.contains("GET")));
    }

    #[test]
    fn test_req_set_url_and_method() {
        let mut client = VarStore::new();
        let mut request = VarStore::new();
        let code = r#"
            req.setUrl("https://override.example.com/v2");
            req.setMethod("POST");
        "#;
        let (_out, ov) = run_pre_script(
            code,
            &mut client,
            &mut request,
            &[],
            "https://api.example.com/users",
            "GET",
            &HashMap::new(),
            None,
        );
        assert_eq!(ov.url, Some("https://override.example.com/v2".into()));
        assert_eq!(ov.method, Some("POST".into()));
    }

    #[test]
    fn test_req_header_crud() {
        let mut client = VarStore::new();
        let mut request = VarStore::new();
        let mut headers = HashMap::new();
        headers.insert("content-type".into(), "application/json".into());

        let code = r#"
            req.setHeader("Authorization", "Bearer new-token");
            req.setHeader("X-Custom", "custom-value");
            console.log(req.getHeader("Content-Type"));
        "#;
        let (_out, ov) = run_pre_script(
            code,
            &mut client,
            &mut request,
            &[],
            "https://example.com",
            "GET",
            &headers,
            None,
        );
        assert_eq!(
            ov.headers.get("Authorization").map(|s| s.as_str()),
            Some("Bearer new-token")
        );
        assert_eq!(
            ov.headers.get("X-Custom").map(|s| s.as_str()),
            Some("custom-value")
        );
    }

    #[test]
    fn test_req_delete_header() {
        let mut client = VarStore::new();
        let mut request = VarStore::new();
        let mut headers = HashMap::new();
        headers.insert("x-debug".into(), "true".into());

        let code = r#"
            req.deleteHeader("X-Debug");
        "#;
        let (_out, ov) = run_pre_script(
            code,
            &mut client,
            &mut request,
            &[],
            "https://example.com",
            "GET",
            &headers,
            None,
        );
        assert!(
            ov.deleted_headers
                .iter()
                .any(|h| h.to_lowercase() == "x-debug")
        );
    }

    #[test]
    fn test_req_delete_headers() {
        let mut client = VarStore::new();
        let mut request = VarStore::new();
        let mut headers = HashMap::new();
        headers.insert("x-a".into(), "1".into());
        headers.insert("x-b".into(), "2".into());

        let code = r#"
            req.deleteHeaders(["X-A", "X-B"]);
        "#;
        let (_out, ov) = run_pre_script(
            code,
            &mut client,
            &mut request,
            &[],
            "https://example.com",
            "GET",
            &headers,
            None,
        );
        assert_eq!(ov.deleted_headers.len(), 2);
    }

    #[test]
    fn test_req_set_body() {
        let mut client = VarStore::new();
        let mut request = VarStore::new();
        let code = r#"
            req.setBody("{\"override\": true}");
        "#;
        let (_out, ov) = run_pre_script(
            code,
            &mut client,
            &mut request,
            &[],
            "https://example.com",
            "POST",
            &HashMap::new(),
            None,
        );
        assert_eq!(ov.body, Some("{\"override\": true}".into()));
    }

    #[test]
    fn test_res_body_parsed_json() {
        let mut client = VarStore::new();
        let mut request = VarStore::new();
        let mut resp_headers: HashMap<String, String> = HashMap::new();
        resp_headers.insert("content-type".into(), "application/json".into());
        let code = r#"
            client.vars.set("token", res.body.token);
            console.log("status: " + res.status);
        "#;
        let out = run_post_script(
            code,
            &mut client,
            &mut request,
            &[],
            "https://example.com",
            "POST",
            &HashMap::new(),
            None,
            200,
            "{\"token\":\"abc-123\"}",
            &resp_headers,
            150,
            256,
            1024,
        );
        assert_eq!(client.get("token"), Some("abc-123"));
        assert!(out.console.iter().any(|l| l.contains("status: 200")));
    }

    #[test]
    fn test_res_status_text() {
        let mut client = VarStore::new();
        let mut request = VarStore::new();
        let resp_headers: HashMap<String, String> = HashMap::new();
        let code = r#"
            console.log(res.statusText);
        "#;
        let out = run_post_script(
            code,
            &mut client,
            &mut request,
            &[],
            "https://example.com",
            "GET",
            &HashMap::new(),
            None,
            201,
            "{}",
            &resp_headers,
            100,
            512,
            4096,
        );
        assert!(out.console.iter().any(|l| l.contains("Created")));
    }

    #[test]
    fn test_client_vars_persist() {
        let mut client = VarStore::new();
        let mut req1 = VarStore::new();
        run_pre_script(
            r#"client.vars.set("secret", "key-42");"#,
            &mut client,
            &mut req1,
            &[],
            "https://example.com",
            "GET",
            &HashMap::new(),
            None,
        );
        let mut req2 = VarStore::new();
        let resp_headers: HashMap<String, String> = HashMap::new();
        let out = run_post_script(
            r#"console.log("secret: " + client.vars.get("secret"));"#,
            &mut client,
            &mut req2,
            &[],
            "https://example.com",
            "GET",
            &HashMap::new(),
            None,
            200,
            "{}",
            &resp_headers,
            100,
            0,
            0,
        );
        assert!(out.console.iter().any(|l| l.contains("key-42")));
    }

    #[test]
    fn test_console_log_capture() {
        let mut client = VarStore::new();
        let mut request = VarStore::new();
        let (out, _) = run_pre_script(
            r#"console.log("hello"); console.log("world");"#,
            &mut client,
            &mut request,
            &[],
            "https://example.com",
            "GET",
            &HashMap::new(),
            None,
        );
        assert_eq!(out.console.len(), 2);
    }

    #[test]
    fn test_btoa() {
        let mut client = VarStore::new();
        let mut request = VarStore::new();
        let (out, _) = run_pre_script(
            r#"console.log(btoa("hello"));"#,
            &mut client,
            &mut request,
            &[],
            "https://example.com",
            "GET",
            &HashMap::new(),
            None,
        );
        assert!(out.console.iter().any(|l| l == "aGVsbG8="));
    }

    #[test]
    fn test_interpolate_vars() {
        use crate::context::resolve_template;

        let mut client = VarStore::new();
        client.set("host".into(), "api.example.com".into());
        let mut request = VarStore::new();
        request.set("token".into(), "abc123".into());

        let result = resolve_template(
            "https://{{host}}/v1?token={{token}}",
            &request,
            &client,
            &[],
        );
        assert_eq!(result, "https://api.example.com/v1?token=abc123");

        let result2 = resolve_template("Hello {{name}}!", &request, &client, &[]);
        assert_eq!(result2, "Hello {{name}}!");

        let result3 = resolve_template("plain text", &request, &client, &[]);
        assert_eq!(result3, "plain text");
    }

    #[test]
    fn test_beep_interpolate_in_script() {
        let mut client = VarStore::new();
        client.set("env".into(), "staging".into());
        let mut request = VarStore::new();
        let file_vars = vec![ParsedFileVariable {
            key: "region".into(),
            value: "us-east".into(),
        }];

        let (out, _) = run_pre_script(
            r#"
                var url = beep.interpolate("https://{{env}}.example.com/api");
                console.log(url);
                req.vars.set("base", beep.interpolate("/v1/{{env}}/{{region}}"));
            "#,
            &mut client,
            &mut request,
            &file_vars,
            "https://example.com",
            "GET",
            &HashMap::new(),
            None,
        );

        assert!(
            out.console
                .iter()
                .any(|l| l == "https://staging.example.com/api")
        );
        assert_eq!(
            request.get("base"),
            Some("/v1/staging/us-east".to_string()).as_deref()
        );
    }

    #[test]
    fn test_script_throw_captured_in_error() {
        let mut client = VarStore::new();
        let mut request = VarStore::new();
        let (out, _) = run_pre_script(
            r#"throw new Error("boom");"#,
            &mut client,
            &mut request,
            &[],
            "https://example.com",
            "GET",
            &HashMap::new(),
            None,
        );
        assert!(
            out.error.as_deref().is_some_and(|e| e.contains("boom")),
            "expected out.error to contain 'boom', got {:?}",
            out.error
        );
    }
}
