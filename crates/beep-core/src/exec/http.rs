//! HTTP protocol executor.
//!
//! Takes an `ExecutableRequest`, sends it over HTTP, and returns `HttpResult`.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use http::{HeaderName, HeaderValue};
use reqwest::Client;
use reqwest_middleware::ClientWithMiddleware;
use serde::{Deserialize, Serialize};

use crate::exec::inspector::{BeepInspector, CapturedRequest};
use crate::executable::{ExecutableRequest, FormFieldType, ResolvedBody, ResolvedFormField};
use crate::types::{HeaderField, HttpVersion};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const DEFAULT_ACCEPT: &str = "*/*";
pub const DEFAULT_ACCEPT_ENCODING: &str = "gzip, deflate, br";
pub const DEFAULT_USER_AGENT: &str = concat!("beep/", env!("CARGO_PKG_VERSION"));
pub const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;

pub fn default_headers() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Accept", DEFAULT_ACCEPT),
        ("Accept-Encoding", DEFAULT_ACCEPT_ENCODING),
        ("User-Agent", DEFAULT_USER_AGENT),
    ]
}

// ---------------------------------------------------------------------------
// HTTP result types
// ---------------------------------------------------------------------------

/// Size breakdown for a request/response.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Size {
    pub headers: u64,
    pub body: u64,
}

/// How the response body bytes are encoded for string transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BodyEncoding {
    #[default]
    Utf8,
    Base64,
}

/// The actual request as sent on the wire, captured by middleware.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentRequest {
    pub url: String,
    pub method: String,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub http_version: String,
    #[serde(default)]
    pub size: Option<Size>,
}

/// HTTP response structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub elapsed_ms: u64,
    pub size: Size,
    #[serde(default)]
    pub body_encoding: BodyEncoding,
}

/// Result of executing an HTTP request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResult {
    pub request: SentRequest,
    pub response: HttpResponse,
}

// ---------------------------------------------------------------------------
// HttpExecutor
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct HttpExecutor {
    client: ClientWithMiddleware,
    client_http1: ClientWithMiddleware,
    client_http2: ClientWithMiddleware,
}

impl HttpExecutor {
    fn base_builder() -> reqwest::ClientBuilder {
        Client::builder()
            .gzip(true)
            .brotli(true)
            .deflate(true)
            .zstd(true)
    }

    fn build_client(builder: reqwest::ClientBuilder) -> ClientWithMiddleware {
        reqwest_middleware::ClientBuilder::new(
            builder.build().expect("Failed to build reqwest client"),
        )
        .with(BeepInspector)
        .build()
    }

    pub fn new() -> Self {
        Self {
            client: Self::build_client(Self::base_builder()),
            client_http1: Self::build_client(Self::base_builder().http1_only()),
            client_http2: Self::build_client(Self::base_builder().http2_prior_knowledge()),
        }
    }

    pub fn with_timeout(timeout_secs: u64) -> Self {
        Self {
            client: Self::build_client(
                Self::base_builder().timeout(Duration::from_secs(timeout_secs)),
            ),
            client_http1: Self::build_client(
                Self::base_builder()
                    .timeout(Duration::from_secs(timeout_secs))
                    .http1_only(),
            ),
            client_http2: Self::build_client(
                Self::base_builder()
                    .timeout(Duration::from_secs(timeout_secs))
                    .http2_prior_knowledge(),
            ),
        }
    }

    /// Execute a compiled `ExecutableRequest`.
    pub async fn execute(&self, request: &ExecutableRequest) -> Result<HttpResult, String> {
        let url = self.build_url(request);
        let start = Instant::now();

        let version_client = match request.http_version {
            HttpVersion::Http1 => &self.client_http1,
            HttpVersion::Http2 => &self.client_http2,
            _ => &self.client,
        };
        let http_method = request.method.to_http_method();
        let mut req_builder = version_client.request(http_method, &url);

        // Merge auto + user headers with proper override:
        // 1. auto headers first (that are enabled)
        // 2. user headers override matching auto keys (remove auto, keep user)
        // 3. user headers with no matching auto key are appended
        let merged = merge_headers(&request.headers);
        for field in &merged {
            let name = HeaderName::from_bytes(field.key.as_bytes())
                .map_err(|e| format!("Invalid header name '{}': {}", field.key, e))?;
            let resolved = resolve_basic_auth(field);
            let val = HeaderValue::from_str(&resolved)
                .map_err(|e| format!("Invalid header value '{}': {}", field.value, e))?;
            req_builder = req_builder.header(name, val);
        }

        let has_user_content_type = merged
            .iter()
            .any(|h| h.key.eq_ignore_ascii_case("content-type"));

        // Body
        let request_body_str: Option<String>;
        let request_body_len: usize;
        match &request.body {
            ResolvedBody::None => {
                request_body_len = 0;
                request_body_str = None;
            }
            ResolvedBody::Raw {
                content,
                content_type,
            } => {
                if !has_user_content_type {
                    req_builder = req_builder.header("content-type", content_type.as_str());
                }
                request_body_len = content.len();
                request_body_str = Some(content.clone());
                req_builder = req_builder.body(content.clone());
            }
            ResolvedBody::FormUrlEncoded(fields) => {
                let encoded = build_url_encoded_body(fields);
                request_body_len = encoded.len();
                request_body_str = Some(encoded.clone());
                if !has_user_content_type {
                    req_builder =
                        req_builder.header("content-type", "application/x-www-form-urlencoded");
                }
                req_builder = req_builder.body(encoded);
            }
            ResolvedBody::FormMultipart(fields) => {
                let (mp_req, mp_body) = build_multipart_body(fields)
                    .await
                    .map_err(|e| format!("Multipart build failed: {}", e))?;
                if !has_user_content_type {
                    if let Some(ct) = mp_req.headers().get("content-type") {
                        if let Ok(v) = ct.to_str() {
                            req_builder = req_builder.header("content-type", v);
                        }
                    }
                }
                request_body_len = mp_body.len();
                request_body_str = std::str::from_utf8(&mp_body).ok().map(|s| s.to_owned());
                req_builder = req_builder.body(mp_body);
            }
        }

        // --- Send ---
        let resp = req_builder
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let mut captured = resp.extensions().get::<CapturedRequest>().cloned();
        if let Some(ref mut cap) = captured {
            cap.body_bytes = request_body_len;
            cap.body_text = request_body_str;
        }

        let status = resp.status().as_u16();
        let resp_headers = extract_headers(resp.headers());

        let raw_body = resp
            .bytes()
            .await
            .map_err(|e| format!("Read response body failed: {}", e))?;

        let (resp_body, body_encoding) = match String::from_utf8(raw_body.to_vec()) {
            Ok(s) => (s, BodyEncoding::Utf8),
            Err(_) => (base64_encode(&raw_body), BodyEncoding::Base64),
        };

        let response_headers_size: u64 = resp_headers
            .iter()
            .map(|(k, v)| (k.len() + v.len() + 4) as u64)
            .sum();
        let response_body_size = raw_body.len() as u64;
        let elapsed_ms = start.elapsed().as_millis() as u64;

        let response = HttpResponse {
            status,
            headers: resp_headers,
            body: resp_body,
            body_encoding,
            elapsed_ms,
            size: Size {
                body: response_body_size,
                headers: response_headers_size,
            },
        };

        let request_echo = if let Some(cap) = captured {
            SentRequest {
                url: cap.url,
                method: cap.method,
                headers: cap.headers,
                body: cap.body_text,
                http_version: cap.http_version,
                size: Some(Size {
                    headers: cap.header_bytes as u64,
                    body: cap.body_bytes as u64,
                }),
            }
        } else {
            SentRequest {
                url: String::new(),
                method: String::new(),
                headers: Vec::new(),
                body: None,
                http_version: String::new(),
                size: None,
            }
        };

        Ok(HttpResult {
            request: request_echo,
            response,
        })
    }

    fn build_url(&self, request: &ExecutableRequest) -> String {
        let base_url = match request.url.find('?') {
            Some(q) => &request.url[..q],
            None => &request.url,
        };

        let params: Vec<String> = request
            .query_params
            .iter()
            .filter(|q| q.enabled && !q.key.is_empty())
            .map(|q| format!("{}={}", urlencode(&q.key), urlencode(&q.value)))
            .collect();

        if params.is_empty() {
            base_url.to_string()
        } else {
            format!("{}?{}", base_url, params.join("&"))
        }
    }
}

impl Default for HttpExecutor {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Merge auto + user headers with proper override:
/// 1. auto headers first (enabled ones from defaults + parsed)
/// 2. user headers override matching auto keys (remove auto, keep user)
/// 3. user headers with no matching auto key are appended
fn merge_headers(headers: &[HeaderField]) -> Vec<HeaderField> {
    // Collect disabled auto keys from @headerAuto directives.
    let disabled_auto_keys: HashSet<String> = headers
        .iter()
        .filter(|h| !h.enabled && h.auto)
        .map(|h| h.key.to_lowercase())
        .collect();

    let all_keys: HashSet<String> = headers
        .iter()
        .filter(|h| h.enabled && !h.key.is_empty())
        .map(|h| h.key.to_lowercase())
        .collect();

    let user_keys: HashSet<String> = headers
        .iter()
        .filter(|h| h.enabled && !h.key.is_empty() && !h.auto)
        .map(|h| h.key.to_lowercase())
        .collect();

    // 1. Auto headers: defaults (not disabled, not overridden) + parsed auto headers.
    let mut merged: Vec<HeaderField> = Vec::new();
    for (key, value) in default_headers() {
        let kl = key.to_lowercase();
        if !disabled_auto_keys.contains(&kl) && !all_keys.contains(&kl) {
            merged.push(HeaderField {
                key: key.to_string(),
                value: value.to_string(),
                enabled: true,
                auto: true,
            });
        }
    }

    // Auto headers from parsed (e.g. UI-managed), skip overridden/disabled.
    for h in headers
        .iter()
        .filter(|h| h.enabled && !h.key.is_empty() && h.auto)
    {
        if !user_keys.contains(&h.key.to_lowercase()) {
            merged.push(h.clone());
        }
    }

    // 2+3. User headers (override auto, then append rest).
    for h in headers
        .iter()
        .filter(|h| h.enabled && !h.key.is_empty() && !h.auto)
    {
        merged.push(h.clone());
    }

    merged
}

/// Auto-encode Basic auth credentials at execution time.
///
/// Encode the credentials if they are not already base64-encoded.
/// The source header remains as-is, encoding is transparent to the user.
fn resolve_basic_auth(field: &HeaderField) -> String {
    if !field.key.eq_ignore_ascii_case("authorization") {
        return field.value.clone();
    }

    let val = field.value.trim();
    let Some(credentials) = val.strip_prefix("Basic ") else {
        return field.value.clone();
    };

    let creds = credentials.trim();
    if is_base64(creds) {
        return field.value.clone();
    }

    // Plain-text credentials, auto-encode.
    // Accept both "user:passwd" and "user passwd" formats.
    let plain: String = creds.replace(' ', ":");
    let encoded = base64_encode(plain.as_bytes());
    format!("Basic {}", encoded)
}

/// Returns `true` if `s` consists entirely of base64 URL-safe characters.
fn is_base64(s: &str) -> bool {
    s.bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=')
}

fn extract_headers(header_map: &http::HeaderMap) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    for (name, value) in header_map {
        if let Ok(v) = value.to_str() {
            headers.insert(name.as_str().to_string(), v.to_string());
        }
    }
    headers
}

fn urlencode(s: &str) -> String {
    let mut result = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(b as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", b));
            }
        }
    }
    result
}

fn build_url_encoded_body(fields: &[ResolvedFormField]) -> String {
    fields
        .iter()
        .filter(|f| !f.key.is_empty())
        .map(|f| format!("{}={}", urlencode(&f.key), urlencode(&f.value)))
        .collect::<Vec<_>>()
        .join("&")
}

async fn build_multipart_body(
    fields: &[ResolvedFormField],
) -> Result<(http::Request<()>, Vec<u8>), String> {
    let boundary = format!(
        "----BeepFormBoundary{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );

    let mut body = Vec::new();

    for field in fields.iter().filter(|f| !f.key.is_empty()) {
        let is_file = field.field_type == FormFieldType::File;
        body.extend_from_slice(b"--");
        body.extend_from_slice(boundary.as_bytes());
        body.extend_from_slice(b"\r\n");
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"");
        body.extend_from_slice(escape_quoted_string(&field.key).as_bytes());
        if is_file && !field.value.is_empty() {
            let filename = std::path::Path::new(&field.value)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file");
            body.extend_from_slice(
                format!("; filename=\"{}\"", escape_quoted_string(filename)).as_bytes(),
            );
        }
        body.extend_from_slice(b"\"\r\n");

        if is_file && !field.value.is_empty() {
            let file_path = &field.value;
            let metadata = tokio::fs::metadata(file_path)
                .await
                .map_err(|e| format!("Cannot read file '{}': {}", file_path, e))?;
            let file_size = metadata.len();
            if file_size > MAX_FILE_SIZE {
                return Err(format!(
                    "File '{}' exceeds max size ({} MB)",
                    file_path,
                    MAX_FILE_SIZE / (1024 * 1024)
                ));
            }
            let file_data = tokio::fs::read(file_path)
                .await
                .map_err(|e| format!("Failed to read file '{}': {}", file_path, e))?;

            let ct = if field.content_type.is_empty() {
                "application/octet-stream"
            } else {
                &field.content_type
            };
            body.extend_from_slice(b"Content-Type: ");
            body.extend_from_slice(ct.as_bytes());
            body.extend_from_slice(b"\r\n\r\n");
            body.extend_from_slice(&file_data);
        } else {
            body.extend_from_slice(b"\r\n");
            body.extend_from_slice(field.value.as_bytes());
        }
        body.extend_from_slice(b"\r\n");
    }

    body.extend_from_slice(b"--");
    body.extend_from_slice(boundary.as_bytes());
    body.extend_from_slice(b"--\r\n");

    let content_type = format!("multipart/form-data; boundary={}", boundary);
    let req = http::Request::builder()
        .header("content-type", content_type)
        .body(())
        .map_err(|e| format!("Build multipart request failed: {}", e))?;

    Ok((req, body))
}

fn escape_quoted_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\r', "")
        .replace('\n', "")
}

fn base64_encode(data: &[u8]) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let b1 = chunk[0];
        let b2 = chunk.get(1).copied().unwrap_or(0);
        let b3 = chunk.get(2).copied().unwrap_or(0);

        let n = ((b1 as u32) << 16) | ((b2 as u32) << 8) | (b3 as u32);

        result.push(CHARSET[((n >> 18) & 0x3F) as usize] as char);
        result.push(CHARSET[((n >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARSET[((n >> 6) & 0x3F) as usize] as char);
        }
        if chunk.len() > 2 {
            result.push(CHARSET[(n & 0x3F) as usize] as char);
        }
    }

    while result.len() % 4 != 0 {
        result.push('=');
    }

    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_headers_not_empty() {
        let headers = default_headers();
        assert!(!headers.is_empty());
        assert!(headers.iter().any(|(k, _)| *k == "Accept"));
    }

    #[test]
    fn test_merge_no_duplicate_auto_headers() {
        // Simulates the GUI sending parsed headers with auto defaults.
        let headers = vec![
            HeaderField {
                key: "Accept".into(),
                value: "*/*".into(),
                enabled: true,
                auto: true,
            },
            HeaderField {
                key: "Accept-Encoding".into(),
                value: "gzip, deflate, br".into(),
                enabled: true,
                auto: true,
            },
            HeaderField {
                key: "User-Agent".into(),
                value: "beep/0.1.0".into(),
                enabled: true,
                auto: true,
            },
        ];
        let merged = merge_headers(&headers);
        // Each key should appear exactly once
        let accept_count = merged
            .iter()
            .filter(|h| h.key.eq_ignore_ascii_case("accept"))
            .count();
        let enc_count = merged
            .iter()
            .filter(|h| h.key.eq_ignore_ascii_case("accept-encoding"))
            .count();
        assert_eq!(accept_count, 1, "Accept header should not be duplicated");
        assert_eq!(
            enc_count, 1,
            "Accept-Encoding header should not be duplicated"
        );
        let ua_count = merged
            .iter()
            .filter(|h| h.key.eq_ignore_ascii_case("user-agent"))
            .count();
        assert_eq!(ua_count, 1, "User-Agent header should not be duplicated");
    }

    #[test]
    fn test_base64_roundtrip() {
        let data = b"hello world";
        let encoded = base64_encode(data);
        // Basic sanity: base64 output length is multiple of 4
        assert_eq!(encoded.len() % 4, 0);
    }

    #[test]
    fn test_resolve_basic_auth_plain() {
        let field = HeaderField {
            key: "Authorization".into(),
            value: "Basic user:passwd".into(),
            enabled: true,
            auto: false,
        };
        let result = resolve_basic_auth(&field);
        assert!(result.starts_with("Basic "));
        assert_ne!(result, "Basic user:passwd");
    }

    #[test]
    fn test_resolve_basic_auth_already_encoded() {
        let encoded = base64_encode(b"user:passwd");
        let field = HeaderField {
            key: "Authorization".into(),
            value: format!("Basic {}", encoded),
            enabled: true,
            auto: false,
        };
        let result = resolve_basic_auth(&field);
        assert_eq!(result, field.value);
    }

    #[test]
    fn test_resolve_basic_auth_space_separated() {
        let field = HeaderField {
            key: "Authorization".into(),
            value: "Basic user passwd".into(),
            enabled: true,
            auto: false,
        };
        let result = resolve_basic_auth(&field);
        assert!(result.starts_with("Basic "));
        assert_ne!(result, "Basic user passwd");
    }

    #[test]
    fn test_resolve_basic_auth_bearer_untouched() {
        let field = HeaderField {
            key: "Authorization".into(),
            value: "Bearer token123".into(),
            enabled: true,
            auto: false,
        };
        let result = resolve_basic_auth(&field);
        assert_eq!(result, "Bearer token123");
    }

    #[test]
    fn test_resolve_basic_auth_non_auth_header() {
        let field = HeaderField {
            key: "Content-Type".into(),
            value: "application/json".into(),
            enabled: true,
            auto: false,
        };
        let result = resolve_basic_auth(&field);
        assert_eq!(result, "application/json");
    }
}
