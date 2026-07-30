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
    /// MIME type for files. Empty -> auto-detect.
    pub content_type: String,
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
}
