//! Unified execution engine.
//!
//! All frontends (GUI, CLI, TUI) call `engine::execute()`
//! which runs the full pipeline:
//! parse -> resolve -> execute -> record.

use crate::compiler::compile_with_ctx;
use crate::context::ExecutionContext;
use crate::exec::http::HttpResult;
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
///
/// 1. Parse (if raw) -> `ParsedRequest`
/// 2. Resolve variables + compile -> `ExecutableRequest`
/// 3. Execute via HTTP executor -> `HttpResult`
/// 4. Record to history
pub async fn execute(
    input: ExecuteInput,
    ctx: &mut ExecutionContext,
) -> Result<HttpResult, String> {
    // Step 1: Parse
    let parsed = match input {
        ExecuteInput::Raw(ref src) => {
            let file = http_parser::parse(src);
            file.requests
                .first()
                .cloned()
                .ok_or_else(|| "No request found in input".to_string())?
        }
        ExecuteInput::Parsed(p) => p,
    };

    // Step 2: Resolve variables + compile
    ctx.clear_request_vars();
    let executable = compile_with_ctx(&parsed, ctx)?;

    // Step 3: Execute
    let executor = crate::exec::http::HttpExecutor::new();
    let result = executor.execute(&executable).await?;

    // Step 4: Record to history
    if let Ok(mut history) = ctx.history.lock() {
        history.add_parsed(parsed, executable, Some(result.clone()), None, None);
    }

    Ok(result)
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
}
