//! Core library
//!
//! HTTP client, request/response handling, history, .http file parsing,
//! and the unified execution pipeline (parse -> compile -> execute -> record).

pub mod compiler;
pub mod context;
pub mod engine;
pub mod exec;
pub mod executable;
pub mod history;
pub mod http_parser;
pub mod types;

// Re-exports
pub use compiler::{compile, compile_with_ctx};
pub use context::ExecutionContext;
pub use engine::{ExecuteInput, execute};
pub use exec::http::{
    BodyEncoding, HttpExecutor, HttpResponse, HttpResult, SentRequest, Size, default_headers,
};
pub use exec::scripts::{RequestOverrides, ScriptOutput, run_post_script, run_pre_script};
pub use executable::{ExecutableRequest, FormFieldType, ResolvedBody, ResolvedFormField};
pub use history::{HistoryEntry, HistoryEntrySummary, RequestHistory};
pub use http_parser::{
    ParsedFileVariable, ParsedFormField, ParsedHeaderField, ParsedHttpFile, ParsedQueryField,
    ParsedRequest, Region, append_request_block, apply_request_update, apply_variable_update,
    parse, serialize_file_variables, serialize_request_block,
};
pub use types::{Auth, HeaderField, HttpMethod, HttpVersion, QueryField};
