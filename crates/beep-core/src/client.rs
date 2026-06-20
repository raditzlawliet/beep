use std::collections::HashMap;
use std::time::{Duration, Instant};

use http::{HeaderName, HeaderValue};
use reqwest::Client;

use crate::models::{Auth, HttpRequest, HttpResponse, ResponseSize};

/// Default header values used by HttpClient::new().
pub const DEFAULT_ACCEPT: &str = "*/*";
pub const DEFAULT_ACCEPT_ENCODING: &str = "gzip, deflate, br";
pub const DEFAULT_USER_AGENT: &str = concat!("beep/", env!("CARGO_PKG_VERSION"));

/// Maximum file size for multipart uploads (100 MB).
pub const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;

/// Map of default headers (key -> value) set on every HttpClient agent.
pub fn default_headers() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Accept", DEFAULT_ACCEPT),
        ("Accept-Encoding", DEFAULT_ACCEPT_ENCODING),
        ("User-Agent", DEFAULT_USER_AGENT),
    ]
}

#[derive(Clone)]
pub struct HttpClient {
    client: Client, // Auto
    client_http1: Client,
    client_http2: Client,
}

impl HttpClient {
    fn base_builder() -> reqwest::ClientBuilder {
        Client::builder()
            .user_agent(DEFAULT_USER_AGENT)
            .gzip(true)
            .brotli(true)
            .deflate(true)
            .zstd(true)
    }

    pub fn new() -> Self {
        Self {
            client: Self::base_builder()
                .build()
                .expect("Failed to build reqwest client"),
            client_http1: Self::base_builder()
                .http1_only()
                .build()
                .expect("Failed to build reqwest http1 client"),
            client_http2: Self::base_builder()
                .http2_prior_knowledge()
                .build()
                .expect("Failed to build reqwest http2 client"),
        }
    }

    pub fn with_timeout(timeout_secs: u64) -> Self {
        Self {
            client: Self::base_builder()
                .timeout(Duration::from_secs(timeout_secs))
                .build()
                .expect("Failed to build reqwest client"),
            client_http1: Self::base_builder()
                .timeout(Duration::from_secs(timeout_secs))
                .http1_only()
                .build()
                .expect("Failed to build reqwest http1 client"),
            client_http2: Self::base_builder()
                .timeout(Duration::from_secs(timeout_secs))
                .http2_prior_knowledge()
                .build()
                .expect("Failed to build reqwest http2 client"),
        }
    }

    pub async fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, String> {
        let url = self.build_url(request);
        let method = request.method;
        let headers = request.headers.clone();
        let auth = request.auth.clone();

        let start = Instant::now();

        // Infer body_mode if not explicitly set (CLI may not set it).
        // body_mode encodes both mode and type: "raw/json", "form-urlencoded", etc.
        let body_mode = request.body_mode.as_deref().unwrap_or_else(|| {
            if !request.form_multipart.is_empty() {
                "form-multipart"
            } else if !request.form_urlencoded.is_empty() {
                "form-urlencoded"
            } else if request.raw_body.is_some() || request.body.is_some() {
                "raw/text"
            } else {
                "none"
            }
        });

        let http_method = method.to_http_method();

        // Route through the appropriate client based on HTTP version.
        let version_client = match request.http_version {
            crate::models::HttpVersion::Http1 => &self.client_http1,
            crate::models::HttpVersion::Http2 => &self.client_http2,
            _ => &self.client,
        };
        let mut req_builder = version_client.request(http_method, &url);

        // Auto headers first on builder
        for field in headers
            .iter()
            .filter(|h| h.enabled && !h.key.is_empty() && h.auto)
        {
            let name = HeaderName::from_bytes(field.key.as_bytes())
                .map_err(|e| format!("Invalid header name '{}': {}", field.key, e))?;
            let val = HeaderValue::from_str(&field.value)
                .map_err(|e| format!("Invalid header value '{}': {}", field.value, e))?;
            req_builder = req_builder.header(name, val);
        }

        match &auth {
            Auth::Basic { username, password } => {
                let encoded = base64_encode(format!("{}:{}", username, password).as_bytes());
                req_builder = req_builder.header(
                    HeaderName::from_static("authorization"),
                    HeaderValue::from_str(&format!("Basic {}", encoded)).unwrap(),
                );
            }
            Auth::Bearer { token } => {
                req_builder = req_builder.header(
                    HeaderName::from_static("authorization"),
                    HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
                );
            }
            Auth::ApiKey { key, value, add_to } if add_to == "header" => {
                let name = HeaderName::from_bytes(key.as_bytes())
                    .unwrap_or(HeaderName::from_static("x-api-key"));
                let val = HeaderValue::from_str(value).unwrap_or(HeaderValue::from_static(""));
                req_builder = req_builder.header(name, val);
            }
            Auth::ApiKey { .. } => {} // query param handled in build_url
            Auth::None => {}
        }

        // Body
        match body_mode {
            "form-urlencoded" => {
                let encoded = build_url_encoded_body(&request.form_urlencoded);
                req_builder = req_builder
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(encoded);
            }
            "form-multipart" => {
                let (mp_req, mp_body) = build_multipart_body(&request.form_multipart)
                    .map_err(|e| format!("Multipart build failed: {}", e))?;
                if let Some(ct) = mp_req.headers().get("content-type") {
                    if let Ok(v) = ct.to_str() {
                        req_builder = req_builder.header("content-type", v);
                    }
                }
                req_builder = req_builder.body(mp_body);
            }
            _ => {
                let content_type: Option<&str> = match body_mode {
                    "raw/json" => Some("application/json"),
                    "raw/xml" => Some("application/xml"),
                    "raw/html" => Some("text/html"),
                    "raw/text" => Some("text/plain"),
                    _ => None,
                };

                let raw = request.raw_body.as_ref().or(request.body.as_ref());
                if let Some(ref b) = raw {
                    if let Some(ct) = content_type {
                        req_builder = req_builder.header("content-type", ct);
                    }
                    req_builder = req_builder.body(b.to_string());
                }
            }
        }

        // User headers (overwrite auto headers for same key)
        for field in headers
            .iter()
            .filter(|h| h.enabled && !h.key.is_empty() && !h.auto)
        {
            let name = HeaderName::from_bytes(field.key.as_bytes())
                .map_err(|e| format!("Invalid header name '{}': {}", field.key, e))?;
            let val = HeaderValue::from_str(&field.value)
                .map_err(|e| format!("Invalid header value '{}': {}", field.value, e))?;
            req_builder = req_builder.header(name, val);
        }

        let resp = req_builder
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let status = resp.status().as_u16();
        let resp_headers = extract_headers(resp.headers());

        // Read decoded body (reqwest auto-decompresses gzip/deflate/brotli)
        let raw_body = resp
            .bytes()
            .await
            .map_err(|e| format!("Read response body failed: {}", e))?;
        let resp_body = String::from_utf8(raw_body.to_vec())
            .map_err(|e| format!("Response body is not valid UTF-8: {}", e))?;

        // Compute sizes
        let response_headers_size: u64 = resp_headers
            .iter()
            .map(|(k, v)| (k.len() + v.len() + 4) as u64)
            .sum();
        let response_body_size = resp_body.len() as u64;

        let elapsed_ms = start.elapsed().as_millis() as u64;

        Ok(HttpResponse {
            status,
            headers: resp_headers,
            body: resp_body,
            elapsed_ms,
            size: ResponseSize {
                response_body: response_body_size,
                response_headers: response_headers_size,
            },
        })
    }

    fn build_url(&self, request: &HttpRequest) -> String {
        let mut params: Vec<String> = request
            .query_params
            .iter()
            .filter(|q| q.enabled && !q.key.is_empty())
            .map(|q| format!("{}={}", urlencode(&q.key), urlencode(&q.value)))
            .collect();

        // Append API key as query param if add_to is "query"
        if let Auth::ApiKey { key, value, add_to } = &request.auth {
            if add_to == "query" {
                params.push(format!("{}={}", urlencode(key), urlencode(value)));
            }
        }

        if params.is_empty() {
            request.url.clone()
        } else {
            format!("{}?{}", request.url, params.join("&"))
        }
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
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

/// Build application/x-www-form-urlencoded body from form fields.
fn build_url_encoded_body(form_data: &[crate::models::FormField]) -> String {
    form_data
        .iter()
        .filter(|f| f.enabled && !f.key.is_empty())
        .map(|f| format!("{}={}", urlencode(&f.key), urlencode(&f.value)))
        .collect::<Vec<_>>()
        .join("&")
}

/// Build multipart/form-data body from form fields.
/// Returns (request_with_headers, body_bytes).
fn build_multipart_body(
    form_data: &[crate::models::FormField],
) -> Result<(http::Request<()>, Vec<u8>), String> {
    let boundary = format!(
        "----BeepFormBoundary{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );

    let mut body = Vec::new();

    for field in form_data.iter().filter(|f| f.enabled && !f.key.is_empty()) {
        let is_file = field.field_type == "file";
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
            let metadata = std::fs::metadata(file_path)
                .map_err(|e| format!("Cannot read file '{}': {}", file_path, e))?;
            let file_size = metadata.len();
            if file_size > MAX_FILE_SIZE {
                return Err(format!(
                    "File '{}' exceeds max size ({} MB)",
                    file_path,
                    MAX_FILE_SIZE / (1024 * 1024)
                ));
            }
            let file_data = std::fs::read(file_path)
                .map_err(|e| format!("Failed to read file '{}': {}", file_path, e))?;

            let ct = if field.content_type.is_empty() {
                "application/octet-stream"
            } else {
                &field.content_type
            };
            body.extend_from_slice(b"Content-Type: ");
            body.extend_from_slice(ct.as_bytes());
            body.extend_from_slice(b"\r\n");
            body.extend_from_slice(b"\r\n");
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

/// Escape special characters for a quoted-string in an HTTP header value
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
