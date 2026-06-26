//! Core types for .http file parsing. Shared between parse, serialize, and edit.

use serde::{Deserialize, Serialize};

/// A file-level variable declared with `@key = value`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileVariable {
    pub key: String,
    pub value: String,
}

/// A parsed header from a request block.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HttpHeaderField {
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// A parsed query parameter.
///
/// `is_inline` tracks placement origin: `true` when parsed from the URL
/// (`?key=value`), `false` when parsed from a multiline `?` / `&` line.
/// Once set to `false` (user disabled -> moved to multiline) it never
/// goes back to `true`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueryField {
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Whether this param lives in the URL (`true`) or multiline (`false`).
    #[serde(default = "default_true")]
    pub is_inline: bool,
}

/// A parsed form field (urlencoded or multipart).
///
/// `is_inline` works the same as QueryField: `true` when parsed from a
/// single-line body, `false` from multiline `&` lines.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FormField {
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// "text" or "file" (multipart only).
    #[serde(default = "default_field_type")]
    pub field_type: String,
    /// MIME type for file uploads. Empty = auto-detect.
    #[serde(default)]
    pub content_type: String,
    /// Whether this field came from an inline single line (`true`) or
    /// multiline `&`-prefixed lines (`false`).
    #[serde(default = "default_true")]
    pub is_inline: bool,
}

fn default_true() -> bool {
    true
}

fn default_field_type() -> String {
    "text".to_string()
}

/// Byte-offset region for a logical section within a request block.
///
/// Both `start` and `end` are **absolute** byte positions in the file.
/// `start == end` means the section is absent.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Region {
    /// Absolute byte offset of the first character of this region.
    pub start: usize,
    /// Absolute byte offset one past the last character of this region.
    /// Splice target: `&raw[start..end]`.
    pub end: usize,
}

impl Region {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }
}

/// A single parsed request block from an .http file.
///
/// Contains both structured data (method, url, headers, etc...) and
/// byte-offset regions for surgical editing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedRequest {
    /// Title after `###` (empty string if none).
    pub title: String,
    /// HTTP method (GET, POST, etc.), uppercased.
    pub method: String,
    /// The request URL (without query string).
    pub url: String,
    /// Parsed headers.
    pub headers: Vec<HttpHeaderField>,
    /// Query parameters parsed from URL and multiline query lines.
    #[serde(default)]
    pub query_params: Vec<QueryField>,
    /// Request body, if any (raw body for JSON/XML/text modes).
    pub body: Option<String>,
    /// Detected body mode hint.
    pub body_mode: Option<String>,
    /// Parsed form-urlencoded fields (when body_mode is form-urlencoded).
    #[serde(default)]
    pub form_urlencoded: Vec<FormField>,
    /// Parsed multipart fields (when body_mode is form-multipart).
    #[serde(default)]
    pub form_multipart: Vec<FormField>,
    /// Pre-request script content, if any.
    pub pre_script: Option<String>,
    /// Post-request script content, if any.
    pub post_script: Option<String>,
    /// HTTP version parsed from request line ("HTTP/1.1", "HTTP/2", etc.).
    #[serde(default)]
    pub http_version: Option<String>,

    // -----------------------------------------------------------------------
    // Region offsets. Absolute byte positions in the source file.
    // Used by the editor for surgical splicing. Ignored by the executor.
    // -----------------------------------------------------------------------
    /// The entire block from `###` to the start of the next `###` (exclusive).
    pub block_region: Region,
    /// The request line: `METHOD URL [HTTP/Version]`.
    pub request_line_region: Region,
    /// Multiline query param lines (the `?` / `&` lines below the request
    /// line). Empty region if all query params are inline in the URL.
    pub query_region: Region,
    /// Header lines from the first header to the end of the last header
    /// (excludes the blank separator between headers and body).
    pub headers_region: Region,
    /// Body content: includes the blank separator line, body content,
    /// post-request script, and response redirect lines.
    /// Starts at the byte immediately after `headers_region.end`
    /// (the `\n` that separates headers from body).
    /// Empty region when there is no body and no post-script / redirect.
    pub body_region: Region,
}

/// Result of parsing an .http file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseHttpFileResult {
    /// File-level @var declarations.
    pub variables: Vec<FileVariable>,
    /// Parsed request blocks.
    pub requests: Vec<ParsedRequest>,
}
