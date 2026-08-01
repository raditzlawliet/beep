//! Executable request types; fully resolved, ready for the network.

use serde::{Deserialize, Serialize};

use crate::types::{HeaderField, HttpMethod, HttpVersion, QueryField};

// ---------------------------------------------------------------------------
// Body
// ---------------------------------------------------------------------------

/// Concrete body encoding with no ambiguity.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ResolvedBody {
    /// No body.
    #[default]
    None,
    /// Raw body with explicit content-type.
    Raw {
        content: String,
        content_type: String,
    },
    /// application/x-www-form-urlencoded fields.
    FormUrlEncoded(Vec<ResolvedFormField>),
    /// multipart/form-data fields (text or file).
    FormMultipart(Vec<ResolvedFormField>),
}

/// A resolved form field.  Variables already interpolated.
/// Only `enabled` fields survive into `ResolvedBody`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedFormField {
    pub key: String,
    pub value: String,
    pub field_type: FormFieldType,
    /// Content-Type for this part. None = not set (no header).
    /// Some("") = auto (executor uses application/octet-stream for files).
    /// Some("image/png") = explicit.
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormFieldType {
    Text,
    File,
}

// ---------------------------------------------------------------------------
// Executable
// ---------------------------------------------------------------------------

/// A fully resolved HTTP request.
///
/// Every `{{variable}}` has been replaced, all ambiguities are gone.
/// This is the single input type for the executor.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutableRequest {
    pub url: String,
    pub method: HttpMethod,
    #[serde(default)]
    pub http_version: HttpVersion,
    #[serde(default)]
    pub headers: Vec<HeaderField>,
    #[serde(default)]
    pub query_params: Vec<QueryField>,
    #[serde(default)]
    pub body: ResolvedBody,
    /// Custom multipart boundary. None = auto-generate.
    #[serde(default)]
    pub multipart_boundary: Option<String>,
    /// Directory of the .http file that produced this request.
    /// Used to resolve relative file paths in multipart file fields.
    #[serde(default)]
    pub source_file_dir: Option<String>,
}
