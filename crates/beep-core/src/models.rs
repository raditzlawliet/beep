//! Data models for HTTP requests and responses

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HTTP version to use for the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub enum HttpVersion {
    #[default]
    Auto,
    Http1,
    Http2,
}

/// HTTP methods supported by Beep
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl HttpMethod {
    pub fn as_str(&self) -> &str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        }
    }

    pub fn to_http_method(&self) -> http::Method {
        match self {
            HttpMethod::Get => http::Method::GET,
            HttpMethod::Post => http::Method::POST,
            HttpMethod::Put => http::Method::PUT,
            HttpMethod::Delete => http::Method::DELETE,
            HttpMethod::Patch => http::Method::PATCH,
            HttpMethod::Head => http::Method::HEAD,
            HttpMethod::Options => http::Method::OPTIONS,
        }
    }
}

impl Default for HttpMethod {
    fn default() -> Self {
        HttpMethod::Get
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Authentication types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Auth {
    None,
    Basic {
        username: String,
        password: String,
    },
    Bearer {
        token: String,
    },
    ApiKey {
        key: String,
        value: String,
        add_to: String,
    },
}

impl Default for Auth {
    fn default() -> Self {
        Auth::None
    }
}

/// A single form data field (key-value pair).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    pub key: String,
    pub value: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// "text" or "file" (multipart only; urlencoded ignores).
    #[serde(default = "default_field_type")]
    pub field_type: String,
    /// MIME type for file uploads. Empty = auto-detect.
    #[serde(default)]
    pub content_type: String,
}

/// A single header field with auto-generate flag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderField {
    pub key: String,
    pub value: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub auto: bool,
}

/// A single query parameter field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryField {
    pub key: String,
    pub value: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

fn default_field_type() -> String {
    "text".to_string()
}

/// HTTP Request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequest {
    pub url: String,
    pub method: HttpMethod,
    #[serde(default)]
    pub http_version: HttpVersion,
    #[serde(default)]
    pub headers: Vec<HeaderField>,
    #[serde(default)]
    pub query_params: Vec<QueryField>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub auth: Auth,

    // GUI helper
    #[serde(default)]
    pub body_mode: Option<String>, // none, raw/json, raw/xml, raw/html, raw/text, form-urlencoded, form-multipart
    /// Draft: raw body content
    #[serde(default)]
    pub raw_body: Option<String>,
    /// Draft: URL-encoded form fields
    #[serde(default)]
    pub form_urlencoded: Vec<FormField>,
    /// Draft: multipart form fields
    #[serde(default)]
    pub form_multipart: Vec<FormField>,
}

impl HttpRequest {
    pub fn new(url: String, method: HttpMethod) -> Self {
        Self {
            url,
            method,
            headers: Vec::new(),
            query_params: Vec::new(),
            body: None,
            auth: Auth::None,
            body_mode: None,
            raw_body: None,
            form_urlencoded: Vec::new(),
            form_multipart: Vec::new(),
            http_version: HttpVersion::default(),
        }
    }

    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.push(HeaderField {
            key,
            value,
            enabled: true,
            auto: false,
        });
        self
    }

    pub fn with_body(mut self, body: String) -> Self {
        self.body = Some(body);
        self
    }

    pub fn with_query(mut self, key: String, value: String) -> Self {
        self.query_params.push(QueryField {
            key,
            value,
            enabled: true,
        });
        self
    }

    pub fn with_auth(mut self, auth: Auth) -> Self {
        self.auth = auth;
        self
    }
}

/// Size breakdown for the response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseSize {
    /// Size of the response body in bytes
    pub response_body: u64,
    /// Size of the response headers in bytes
    pub response_headers: u64,
}

impl ResponseSize {
    /// Total response size (headers + body)
    pub fn response_total(&self) -> u64 {
        self.response_headers + self.response_body
    }
}

/// HTTP Response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub elapsed_ms: u64,
    pub size: ResponseSize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_method_display() {
        assert_eq!(HttpMethod::default().to_string(), "GET");
        assert_eq!(HttpMethod::Get.as_str(), "GET");
        assert_eq!(HttpMethod::Post.to_string(), "POST");
        assert_eq!(HttpMethod::Put.to_string(), "PUT");
        assert_eq!(HttpMethod::Delete.to_string(), "DELETE");
        assert_eq!(HttpMethod::Patch.to_string(), "PATCH");
        assert_eq!(HttpMethod::Head.to_string(), "HEAD");
        assert_eq!(HttpMethod::Options.to_string(), "OPTIONS");
    }

    #[test]
    fn test_http_request_builder() {
        let req = HttpRequest::new("https://api.example.com".to_string(), HttpMethod::Get)
            .with_header("User-Agent".to_string(), "Beep".to_string())
            .with_query("key".to_string(), "value".to_string());

        assert_eq!(req.url, "https://api.example.com");
        assert_eq!(req.method, HttpMethod::Get);
        assert_eq!(req.headers[0].key, "User-Agent");
        assert_eq!(req.headers[0].value, "Beep");
        assert_eq!(req.query_params[0].key, "key");
        assert_eq!(req.query_params[0].value, "value");
    }
}
