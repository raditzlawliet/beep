//! Core library
//!
//! Provides Wrapper HTTP client functionality, request/response handling,
//! and request history management.

pub mod client;
pub mod history;
pub mod models;

// Re-exports alias
pub use client::HttpClient;
pub use history::RequestHistory;
pub use models::{HttpMethod, HttpRequest, HttpResponse, ResponseSize};
