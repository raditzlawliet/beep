//! Execution context shared across the pipeline.
//!
//! Carries variable stores, history handle, and variable resolution.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::history::RequestHistory;
use crate::http_parser::ParsedFileVariable;

// ---------------------------------------------------------------------------
// VarStore
// ---------------------------------------------------------------------------

/// Simple key-value store with get/set/clear semantics.
#[derive(Debug, Clone, Default)]
pub struct VarStore {
    vars: HashMap<String, String>,
}

impl VarStore {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.vars.get(key).map(|s| s.as_str())
    }

    pub fn set(&mut self, key: String, value: String) {
        self.vars.insert(key, value);
    }

    pub fn clear(&mut self) {
        self.vars.clear();
    }
}

// ---------------------------------------------------------------------------
// ExecutionContext
// ---------------------------------------------------------------------------

/// Shared state for the entire execution pipeline.
pub struct ExecutionContext {
    /// Session-scoped variables (client.vars in spec). TODO: full cascade.
    pub client_vars: VarStore,
    /// Request-scoped variables (request.vars in spec). Cleared per-request.
    pub request_vars: VarStore,
    /// File-level @var declarations.
    pub file_vars: Vec<ParsedFileVariable>,
    /// Shared history handle.
    pub history: Arc<Mutex<RequestHistory>>,
}

impl ExecutionContext {
    pub fn new(history: Arc<Mutex<RequestHistory>>) -> Self {
        Self {
            client_vars: VarStore::new(),
            request_vars: VarStore::new(),
            file_vars: Vec::new(),
            history,
        }
    }

    /// Clear request-scoped variables (called before each request).
    pub fn clear_request_vars(&mut self) {
        self.request_vars.clear();
    }

    /// Resolve `{{variable}}` placeholders in `input`.
    ///
    /// Highest first:
    /// 1. `request_vars`   - request scope
    /// 2. `client_vars`    - session scope
    /// 3. `file_vars`      - @var declarations
    pub fn resolve(&self, input: &str) -> String {
        resolve_template(
            input,
            &self.request_vars,
            &self.client_vars,
            &self.file_vars,
        )
    }
}

/// Resolve `{{variable}}` placeholders using the cascade:
/// request_vars > client_vars > file_vars. Unknown variables left as-is.
pub fn resolve_template(
    input: &str,
    request_vars: &VarStore,
    client_vars: &VarStore,
    file_vars: &[ParsedFileVariable],
) -> String {
    let mut out = input.to_string();

    let mut start = 0;
    while let Some(begin) = out[start..].find("{{") {
        let abs_begin = start + begin;
        if let Some(end) = out[abs_begin + 2..].find("}}") {
            let abs_end = abs_begin + 2 + end;
            let key = &out[abs_begin + 2..abs_end];

            let replacement = request_vars
                .get(key)
                .or_else(|| client_vars.get(key))
                .or_else(|| {
                    file_vars
                        .iter()
                        .find(|v| v.key == key)
                        .map(|v| v.value.as_str())
                })
                .map(|s| s.to_string());

            if let Some(repl) = replacement {
                out.replace_range(abs_begin..abs_end + 2, &repl);
                start = abs_begin + repl.len();
            } else {
                start = abs_end + 2;
            }
        } else {
            break;
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_precedence() {
        let history = Arc::new(Mutex::new(RequestHistory::new()));
        let mut ctx = ExecutionContext::new(history);

        ctx.file_vars.push(ParsedFileVariable {
            key: "base".into(),
            value: "file-level".into(),
        });
        ctx.client_vars.set("base".into(), "client-level".into());
        ctx.request_vars.set("base".into(), "request-level".into());

        let result = ctx.resolve("{{base}}");
        assert_eq!(result, "request-level");
    }

    #[test]
    fn test_resolve_fallback() {
        let history = Arc::new(Mutex::new(RequestHistory::new()));
        let mut ctx = ExecutionContext::new(history);

        ctx.file_vars.push(ParsedFileVariable {
            key: "host".into(),
            value: "api.example.com".into(),
        });

        let result = ctx.resolve("https://{{host}}/users");
        assert_eq!(result, "https://api.example.com/users");
    }

    #[test]
    fn test_no_replacement() {
        let history = Arc::new(Mutex::new(RequestHistory::new()));
        let ctx = ExecutionContext::new(history);

        let result = ctx.resolve("no variables here");
        assert_eq!(result, "no variables here");
    }
}
