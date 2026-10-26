//! Execution context shared across the pipeline.
//!
//! Carries variable stores, history handle, and variable resolution.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::dynamic::{EnvGuard, resolve_dynamic_with};
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

    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.vars.remove(key)
    }

    pub fn clear(&mut self) {
        self.vars.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.vars.is_empty()
    }

    /// All entries sorted by key.
    pub fn entries(&self) -> Vec<(String, String)> {
        let mut out: Vec<_> = self
            .vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        out.sort();
        out
    }

    /// Build from `@var` declarations. Later declaration of the same key wins.
    pub fn from_file_vars(vars: &[ParsedFileVariable]) -> Self {
        let mut store = Self::new();
        for v in vars {
            store.set(v.key.clone(), v.value.clone());
        }
        store
    }
}

// ---------------------------------------------------------------------------
// ExecutionContext
// ---------------------------------------------------------------------------

/// Shared state for the entire execution pipeline.
pub struct ExecutionContext {
    /// Script-set request scope (`req.vars`). Cleared per-request.
    pub request_vars: VarStore,
    /// Script-set session scope (`client.vars`). Owner keeps it alive across requests.
    pub client_vars: VarStore,
    /// Static `@var` declared inside the request block.
    pub request_static_vars: VarStore,
    /// File-level `@var` declarations.
    pub file_vars: Vec<ParsedFileVariable>,
    /// `.beep/vars.json` folder cascade (already merged, deepest wins).
    pub folder_vars: VarStore,
    /// Directory of the source .http file, for resolving relative multipart file paths.
    pub source_file_dir: Option<String>,
    /// `$processEnv` permission guard (default allows all).
    pub env: EnvGuard,
    /// Shared history handle.
    pub history: Arc<Mutex<RequestHistory>>,
}

impl ExecutionContext {
    pub fn new(history: Arc<Mutex<RequestHistory>>) -> Self {
        Self {
            request_vars: VarStore::new(),
            client_vars: VarStore::new(),
            request_static_vars: VarStore::new(),
            file_vars: Vec::new(),
            folder_vars: VarStore::new(),
            source_file_dir: None,
            env: EnvGuard::allow_all(),
            history,
        }
    }

    /// Clear request-scoped variables (called before each request).
    pub fn clear_request_vars(&mut self) {
        self.request_vars.clear();
    }

    /// Static scopes below the script scopes, in precedence order:
    /// request `@var` > file `@var` > folder vars.
    pub fn static_scopes(&self) -> Vec<VarStore> {
        vec![
            self.request_static_vars.clone(),
            VarStore::from_file_vars(&self.file_vars),
            self.folder_vars.clone(),
        ]
    }

    /// Resolve `{{variable}}` placeholders in `input`.
    ///
    /// Highest first:
    /// 1. `request_vars`        - req.vars (script)
    /// 2. `client_vars`         - client.vars (script, session)
    /// 3. `request_static_vars` - request `@var`
    /// 4. `file_vars`           - file `@var`
    /// 5. `folder_vars`         - .beep/vars.json cascade
    pub fn resolve(&self, input: &str) -> String {
        if !input.contains("{{") {
            return input.to_string();
        }
        let file = VarStore::from_file_vars(&self.file_vars);
        resolve_template_with(
            input,
            &[
                &self.request_vars,
                &self.client_vars,
                &self.request_static_vars,
                &file,
                &self.folder_vars,
            ],
            Some(&self.env),
        )
    }
}

// ---------------------------------------------------------------------------
// Var sources (editor insight)
// ---------------------------------------------------------------------------

/// Static scope a variable value comes from (script `req.vars` is runtime-only, not listed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VarScope {
    Client,
    Request,
    File,
    Folder,
}

/// One value of a variable in one scope. `origin` = folder vars file path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VarSource {
    pub key: String,
    pub scope: VarScope,
    pub value: String,
    pub origin: Option<String>,
}

/// Every known variable value, highest precedence first (the first entry of a key wins).
pub fn collect_var_sources(
    client: &VarStore,
    request: &[ParsedFileVariable],
    file: &[ParsedFileVariable],
    folder: &[VarSource],
) -> Vec<VarSource> {
    let mut out = Vec::new();
    for (key, value) in client.entries() {
        out.push(VarSource { key, scope: VarScope::Client, value, origin: None });
    }
    for (scope, vars) in [(VarScope::Request, request), (VarScope::File, file)] {
        for (key, value) in VarStore::from_file_vars(vars).entries() {
            out.push(VarSource { key, scope, value, origin: None });
        }
    }
    out.extend(folder.iter().rev().cloned());
    out
}

/// Max nesting depth when a variable value references other variables.
const MAX_DEPTH: usize = 10;

/// Resolve `{{variable}}` placeholders against `scopes` (highest precedence first).
///
/// - Whitespace inside braces is ignored.
/// - `{{$name args}}` are dynamic variables, generated per occurrence.
/// - Values containing `{{...}}` are resolved again (max depth 10).
/// - A variable referencing its own name resolves from the next lower scope
///   (e.g. request `@baseUrl = {{baseUrl}}/v2`).
/// - Unknown variables and cycles are left as-is.
pub fn resolve_template(input: &str, scopes: &[&VarStore]) -> String {
    resolve_template_with(input, scopes, None)
}

/// Like `resolve_template`, with a `$processEnv` guard (`None` allows all).
pub fn resolve_template_with(input: &str, scopes: &[&VarStore], env: Option<&EnvGuard>) -> String {
    let mut stack: Vec<(String, usize)> = Vec::new();
    resolve_inner(input, scopes, env, &mut stack)
}

fn resolve_inner(
    input: &str,
    scopes: &[&VarStore],
    env: Option<&EnvGuard>,
    stack: &mut Vec<(String, usize)>,
) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;

    while let Some(begin) = rest.find("{{") {
        let after = &rest[begin + 2..];
        let Some(end) = after.find("}}") else {
            break;
        };
        out.push_str(&rest[..begin]);
        let placeholder = &rest[begin..begin + 2 + end + 2];
        match lookup(after[..end].trim(), scopes, env, stack) {
            Some(value) => out.push_str(&value),
            None => out.push_str(placeholder),
        }
        rest = &after[end + 2..];
    }

    out.push_str(rest);
    out
}

fn lookup(
    key: &str,
    scopes: &[&VarStore],
    env: Option<&EnvGuard>,
    stack: &mut Vec<(String, usize)>,
) -> Option<String> {
    if key.is_empty() {
        return None;
    }
    if let Some(expr) = key.strip_prefix('$') {
        return resolve_dynamic_with(expr.trim(), env);
    }

    // Key already being resolved: only look below the scope that defined it.
    let from = stack
        .iter()
        .rev()
        .find(|(k, _)| k == key)
        .map_or(0, |(_, idx)| idx + 1);
    let (idx, raw) = scopes
        .iter()
        .enumerate()
        .skip(from)
        .find_map(|(i, s)| s.get(key).map(|v| (i, v)))?;

    if !raw.contains("{{") {
        return Some(raw.to_string());
    }
    if stack.len() >= MAX_DEPTH {
        return None;
    }

    stack.push((key.to_string(), idx));
    let value = resolve_inner(raw, scopes, env, stack);
    stack.pop();
    Some(value)
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

    fn store(pairs: &[(&str, &str)]) -> VarStore {
        let mut s = VarStore::new();
        for (k, v) in pairs {
            s.set(k.to_string(), v.to_string());
        }
        s
    }

    #[test]
    fn test_resolve_all_scopes_precedence() {
        let history = Arc::new(Mutex::new(RequestHistory::new()));
        let mut ctx = ExecutionContext::new(history);
        ctx.folder_vars = store(&[("a", "folder"), ("b", "folder"), ("c", "folder"), ("d", "folder"), ("e", "folder")]);
        ctx.file_vars = vec![
            ParsedFileVariable { key: "a".into(), value: "file".into() },
            ParsedFileVariable { key: "b".into(), value: "file".into() },
            ParsedFileVariable { key: "c".into(), value: "file".into() },
            ParsedFileVariable { key: "d".into(), value: "file".into() },
        ];
        ctx.request_static_vars = store(&[("a", "req-static"), ("b", "req-static"), ("c", "req-static")]);
        ctx.client_vars = store(&[("a", "client"), ("b", "client")]);
        ctx.request_vars = store(&[("a", "req")]);

        assert_eq!(
            ctx.resolve("{{a}} {{b}} {{c}} {{d}} {{e}}"),
            "req client req-static file folder"
        );
    }

    #[test]
    fn test_collect_var_sources_order() {
        let client = store(&[("a", "client")]);
        let req = vec![ParsedFileVariable { key: "a".into(), value: "req".into() }];
        let file = vec![
            ParsedFileVariable { key: "a".into(), value: "file1".into() },
            ParsedFileVariable { key: "a".into(), value: "file2".into() },
        ];
        let folder = vec![
            VarSource { key: "a".into(), scope: VarScope::Folder, value: "root".into(), origin: Some("r".into()) },
            VarSource { key: "a".into(), scope: VarScope::Folder, value: "deep".into(), origin: Some("d".into()) },
        ];
        let got: Vec<String> = collect_var_sources(&client, &req, &file, &folder)
            .into_iter()
            .map(|v| v.value)
            .collect();
        assert_eq!(got, vec!["client", "req", "file2", "deep", "root"]);
    }

    #[test]
    fn test_resolve_env_guard_denies() {
        let s = store(&[("token", "{{$processEnv BEEP_TEST_SECRET}}")]);
        let guard = EnvGuard::allow_only(Default::default());
        assert_eq!(resolve_template_with("x={{token}}", &[&s], Some(&guard)), "x=");
        assert_eq!(guard.denied(), vec!["BEEP_TEST_SECRET"]);
    }

    #[test]
    fn test_resolve_trims_whitespace() {
        let s = store(&[("token", "abc")]);
        assert_eq!(resolve_template("Bearer {{ token }}", &[&s]), "Bearer abc");
    }

    #[test]
    fn test_resolve_nested_across_scopes() {
        let file = store(&[("baseUrl", "{{host}}/api/{{ version }}")]);
        let folder = store(&[("host", "https://x.io"), ("version", "v1")]);
        assert_eq!(
            resolve_template("{{baseUrl}}/users", &[&file, &folder]),
            "https://x.io/api/v1/users"
        );
    }

    #[test]
    fn test_resolve_self_reference_uses_lower_scope() {
        let request = store(&[("baseUrl", "{{baseUrl}}/v2")]);
        let file = store(&[("baseUrl", "https://x.io")]);
        assert_eq!(resolve_template("{{baseUrl}}", &[&request, &file]), "https://x.io/v2");
    }

    #[test]
    fn test_resolve_cycle_left_as_is() {
        let s = store(&[("a", "{{b}}"), ("b", "{{a}}")]);
        assert_eq!(resolve_template("x={{a}}", &[&s]), "x={{a}}");
        let self_ref = store(&[("a", "pre-{{a}}")]);
        assert_eq!(resolve_template("{{a}}", &[&self_ref]), "pre-{{a}}");
    }

    #[test]
    fn test_resolve_depth_limit() {
        let mut s = VarStore::new();
        for i in 0..20 {
            s.set(format!("v{i}"), format!("{{{{v{}}}}}", i + 1));
        }
        s.set("v20".into(), "end".into());
        // Too deep: stops resolving but does not loop or panic.
        let out = resolve_template("{{v0}}", &[&s]);
        assert!(out.contains("{{v"));
        // Shallow chain resolves fully.
        assert_eq!(resolve_template("{{v15}}", &[&s]), "end");
    }

    #[test]
    fn test_resolve_unknown_and_unclosed() {
        let s = store(&[("a", "1")]);
        assert_eq!(resolve_template("{{missing}} {{a}}", &[&s]), "{{missing}} 1");
        assert_eq!(resolve_template("{{a}} {{oops", &[&s]), "1 {{oops");
        assert_eq!(resolve_template("{{}}", &[&s]), "{{}}");
    }

    #[test]
    fn test_resolve_dynamic() {
        let out = resolve_template("{{$guid}}|{{ $randomInt 5 6 }}|{{$unknown}}", &[]);
        let parts: Vec<&str> = out.split('|').collect();
        assert_eq!(parts[0].len(), 36);
        assert_eq!(parts[1], "5");
        assert_eq!(parts[2], "{{$unknown}}");
    }

    #[test]
    fn test_no_replacement() {
        let history = Arc::new(Mutex::new(RequestHistory::new()));
        let ctx = ExecutionContext::new(history);

        let result = ctx.resolve("no variables here");
        assert_eq!(result, "no variables here");
    }
}
