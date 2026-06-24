//! Core library
//!
//! Provides Wrapper HTTP client functionality, request/response handling,
//! request history management, and .http file parsing.

pub mod client;
pub mod history;
pub mod http_parser;
pub mod models;

// Re-exports
pub use client::HttpClient;
pub use history::{HistoryEntrySummary, RequestHistory};
pub use http_parser::{
    FileVariable, FormField, HttpHeaderField, ParseHttpFileResult, ParsedRequest, QueryField,
    append_request_block, apply_request_update, apply_variable_update, parse_http_file,
    serialize_file_variables, serialize_request_block,
};
pub use models::{HttpMethod, HttpRequest, HttpResponse, ResponseSize};
