use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::time::{Duration, Instant};

use flate2::read::{DeflateDecoder, GzDecoder};
use http::{HeaderName, HeaderValue, Request};

use crate::models::{Auth, HttpRequest, HttpResponse, ResponseSize};

/// Default header values used by HttpClient::new().
pub const DEFAULT_ACCEPT: &str = "*/*";
pub const DEFAULT_ACCEPT_ENCODING: &str = "";
pub const DEFAULT_USER_AGENT: &str = concat!("beep/", env!("CARGO_PKG_VERSION"));

/// Map of default headers (key -> value) set on every HttpClient agent.
pub fn default_headers() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Accept", DEFAULT_ACCEPT),
        ("Accept-Encoding", DEFAULT_ACCEPT_ENCODING),
        ("User-Agent", DEFAULT_USER_AGENT),
    ]
}

pub struct HttpClient {
    agent: ureq::Agent,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            agent: ureq::Agent::config_builder()
                .http_status_as_error(false)
                .accept(DEFAULT_ACCEPT)
                .accept_encoding(DEFAULT_ACCEPT_ENCODING)
                .user_agent(DEFAULT_USER_AGENT)
                .build()
                .into(),
        }
    }

    pub fn with_timeout(timeout_secs: u64) -> Self {
        Self {
            agent: ureq::Agent::config_builder()
                .http_status_as_error(false)
                .timeout_global(Some(Duration::from_secs(timeout_secs)))
                .build()
                .into(),
        }
    }

    pub fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, String> {
        let url = self.build_url(request);
        let method = request.method;
        let headers = request.headers.clone();
        let auth = request.auth.clone();
        let body = request.body.clone();

        let start = Instant::now();

        let http_method = method.to_http_method();

        let mut req_builder = Request::builder().method(&http_method).uri(&url);

        for (key, value) in &headers {
            let name = HeaderName::from_bytes(key.as_bytes())
                .map_err(|e| format!("Invalid header name '{}': {}", key, e))?;
            let val = HeaderValue::from_str(value)
                .map_err(|e| format!("Invalid header value '{}': {}", value, e))?;
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
            Auth::None => {}
        }

        let mut resp = if let Some(ref b) = body {
            // with body request
            req_builder = req_builder.header(
                HeaderName::from_static("content-type"),
                HeaderValue::from_static("application/json"),
            );
            let request = req_builder
                .body(b.as_bytes())
                .map_err(|e| format!("Build request failed: {}", e))?;

            self.agent.run(request)
        } else {
            // no body request
            let request = req_builder
                .body(())
                .map_err(|e| format!("Build request failed: {}", e))?;

            self.agent.run(request)
        }
        .map_err(|e| format!("Request failed: {}", e))?;

        let status: u16 = resp.status().into();
        let resp_headers = extract_headers(resp.headers());

        // Read raw bytes and decompress if needed
        let raw_body = resp
            .body_mut()
            .read_to_vec()
            .map_err(|e| format!("Read response body failed: {}", e))?;

        let content_encoding = resp_headers.get("content-encoding").map(|s| s.as_str());
        let resp_body = match decode_body(&raw_body, content_encoding) {
            Ok(body) => body,
            Err(e) => format!(
                "[Body decode error: {}] (Content-Encoding: {})",
                e,
                content_encoding.unwrap_or("none"),
            ),
        };

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
        if request.query_params.is_empty() {
            request.url.clone()
        } else {
            let params: Vec<String> = request
                .query_params
                .iter()
                .map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v)))
                .collect();
            format!("{}?{}", request.url, params.join("&"))
        }
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Decompress response body based on Content-Encoding header value.
/// Supports gzip, deflate, brotli, and identity (no compression).
fn decode_body(data: &[u8], content_encoding: Option<&str>) -> Result<String, String> {
    let encodings: Vec<&str> = content_encoding
        .map(|s| {
            s.split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    if encodings.is_empty() {
        return String::from_utf8(data.to_vec())
            .map_err(|e| format!("Response body is not valid UTF-8: {}", e));
    }

    let mut buf = data.to_vec();

    for enc in &encodings {
        buf = match *enc {
            "gzip" | "x-gzip" => {
                let mut d = GzDecoder::new(Cursor::new(&buf));
                let mut out = Vec::new();
                d.read_to_end(&mut out)
                    .map_err(|e| format!("gzip decode failed: {}", e))?;
                out
            }
            "deflate" => {
                let mut d = DeflateDecoder::new(Cursor::new(&buf));
                let mut out = Vec::new();
                d.read_to_end(&mut out)
                    .map_err(|e| format!("deflate decode failed: {}", e))?;
                out
            }
            "br" => {
                let mut out = Vec::new();
                brotli::BrotliDecompress(&mut Cursor::new(&buf), &mut out)
                    .map_err(|e| format!("brotli decode failed: {}", e))?;
                out
            }
            "identity" | "" => buf,
            other => {
                return Err(format!("Unsupported Content-Encoding: {}", other));
            }
        };
    }

    String::from_utf8(buf).map_err(|e| format!("Response body is not valid UTF-8: {}", e))
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
