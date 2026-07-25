//! Reqwest middleware that captures request headers, URL, method, sizes, and HTTP version.
//! Body bytes are filled by the caller (execute()).

use async_trait::async_trait;
use reqwest_middleware::{Middleware, Next};

/// Wire-level request data captured by the middleware.
#[derive(Debug, Clone)]
pub(crate) struct CapturedRequest {
    pub url: String,
    pub method: String,
    pub headers: Vec<(String, String)>,
    pub header_bytes: usize,
    pub body_bytes: usize,
    pub body_text: Option<String>,
    pub http_version: String,
}

/// Middleware that captures the outgoing request before it hits the wire.
/// The captured data is stored in `http::Extensions` and retrieved after the response arrives.
pub(crate) struct BeepInspector;

#[async_trait]
impl Middleware for BeepInspector {
    async fn handle(
        &self,
        req: reqwest::Request,
        extensions: &mut http::Extensions,
        next: Next<'_>,
    ) -> reqwest_middleware::Result<reqwest::Response> {
        let url = req.url().to_string();
        let method = req.method().to_string();
        let http_version = format!("{:?}", req.version());

        let headers: Vec<(String, String)> = req
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
            .collect();

        let req_line_bytes = format!("{} {} {}\r\n", method, url, http_version).len();
        let req_header_bytes: usize = headers
            .iter()
            .map(|(k, v)| k.len() + 2 + v.len() + 2)
            .sum::<usize>()
            + req_line_bytes
            + 2;

        extensions.insert(CapturedRequest {
            url,
            method,
            headers,
            header_bytes: req_header_bytes,
            body_bytes: 0,
            body_text: None,
            http_version,
        });

        let mut resp = next.run(req, extensions).await?;

        let actual_version = format!("{:?}", resp.version());
        if let Some(mut captured) = extensions.remove::<CapturedRequest>() {
            captured.http_version = actual_version;
            resp.extensions_mut().insert(captured);
        }

        Ok(resp)
    }
}
