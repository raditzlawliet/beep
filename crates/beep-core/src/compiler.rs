//! Compiler: `...Parsed` + variables = `Executable...`.
//!
//! This is a pure transformation; no I/O, no network, no side effects.
//! Variable interpolation happens here.
use crate::context::ExecutionContext;
use crate::executable::{ExecutableRequest, FormFieldType, ResolvedBody, ResolvedFormField};
use crate::http_parser::{ParsedFileVariable, ParsedRequest, effective_body_kind};
use crate::types::{HeaderField, HttpMethod, HttpVersion, QueryField};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compile a `ParsedRequest` into an `ExecutableRequest`
/// by resolving `{{variable}}` references and selecting the concrete body encoding.
pub fn compile(
    parsed: &ParsedRequest,
    variables: &[ParsedFileVariable],
) -> Result<ExecutableRequest, String> {
    let method = parse_method(&parsed.method)?;
    let http_version = parse_version(parsed.http_version.as_deref());
    let url = resolve(&parsed.url, variables);
    let headers = compile_headers(&parsed.headers, variables);
    let query_params = compile_query_params(&parsed.query_params, variables);
    let body = compile_body(parsed, variables);
    let multipart_boundary = parsed
        .multipart_boundary
        .as_ref()
        .map(|b| resolve(b, variables));

    Ok(ExecutableRequest {
        url,
        method,
        http_version,
        headers,
        query_params,
        body,
        multipart_boundary,
    })
}

/// Compile using `ExecutionContext` for variable resolution with full cascade
/// (request.vars > client.vars > file_vars).
pub fn compile_with_ctx(
    parsed: &ParsedRequest,
    ctx: &ExecutionContext,
) -> Result<ExecutableRequest, String> {
    let method = parse_method(&parsed.method)?;
    let http_version = parse_version(parsed.http_version.as_deref());
    let url = ctx.resolve(&parsed.url);
    let headers = compile_headers_ctx(&parsed.headers, ctx);
    let query_params = compile_query_params_ctx(&parsed.query_params, ctx);
    let body = compile_body_ctx(parsed, ctx);
    let multipart_boundary = parsed.multipart_boundary.as_ref().map(|b| ctx.resolve(b));

    Ok(ExecutableRequest {
        url,
        method,
        http_version,
        headers,
        query_params,
        body,
        multipart_boundary,
    })
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

fn parse_method(raw: &str) -> Result<HttpMethod, String> {
    let upper = raw.to_uppercase();
    match upper.as_str() {
        "GET" => Ok(HttpMethod::Get),
        "POST" => Ok(HttpMethod::Post),
        "PUT" => Ok(HttpMethod::Put),
        "DELETE" => Ok(HttpMethod::Delete),
        "PATCH" => Ok(HttpMethod::Patch),
        "HEAD" => Ok(HttpMethod::Head),
        "OPTIONS" => Ok(HttpMethod::Options),
        _ => Ok(HttpMethod::Other(raw.to_string())),
    }
}

fn parse_version(raw: Option<&str>) -> HttpVersion {
    match raw {
        Some("HTTP/1.1") => HttpVersion::Http1,
        Some("HTTP/2") | Some("HTTP/2.0") => HttpVersion::Http2,
        _ => HttpVersion::Auto,
    }
}

// ---------------------------------------------------------------------------
// Variable resolution
// ---------------------------------------------------------------------------

/// Replace every `{{key}}` in `input` with the matching variable value.
/// Single-pass: replaced values are **not** re-scanned.
fn resolve(input: &str, vars: &[ParsedFileVariable]) -> String {
    let mut out = input.to_string();
    for v in vars {
        out = out.replace(&format!("{{{{{}}}}}", v.key), &v.value);
    }
    out
}

// ---------------------------------------------------------------------------
// Sub-compilers
// ---------------------------------------------------------------------------

fn compile_headers(
    raw: &[crate::http_parser::ParsedHeaderField],
    vars: &[ParsedFileVariable],
) -> Vec<HeaderField> {
    raw.iter()
        .filter(|h| !h.key.is_empty() && (h.enabled || h.auto))
        .map(|h| HeaderField {
            key: resolve(&h.key, vars),
            value: resolve(&h.value, vars),
            enabled: h.enabled,
            auto: h.auto,
        })
        .collect()
}

fn compile_query_params(
    raw: &[crate::http_parser::ParsedQueryField],
    vars: &[ParsedFileVariable],
) -> Vec<QueryField> {
    raw.iter()
        .filter(|q| q.enabled && !q.key.is_empty())
        .map(|q| QueryField {
            key: resolve(&q.key, vars),
            value: resolve(&q.value, vars),
            enabled: true,
        })
        .collect()
}

fn compile_body(parsed: &ParsedRequest, vars: &[ParsedFileVariable]) -> ResolvedBody {
    let mode = effective_body_kind(
        &parsed.headers,
        parsed.body_directive.as_deref(),
        parsed.body.as_deref(),
    );

    match mode {
        "none" => ResolvedBody::None,

        "form-urlencoded" => {
            let fields: Vec<ResolvedFormField> = parsed
                .form_urlencoded
                .iter()
                .filter(|f| f.enabled)
                .map(|f| ResolvedFormField {
                    key: resolve(&f.key, vars),
                    value: resolve(&f.value, vars),
                    field_type: FormFieldType::Text,
                    content_type: None,
                })
                .collect();
            if fields.is_empty() {
                ResolvedBody::None
            } else {
                ResolvedBody::FormUrlEncoded(fields)
            }
        }

        "form-multipart" => {
            let fields: Vec<ResolvedFormField> = parsed
                .form_multipart
                .iter()
                .filter(|f| f.enabled)
                .map(|f| ResolvedFormField {
                    key: resolve(&f.key, vars),
                    value: resolve(&f.value, vars),
                    field_type: if f.field_type == "file" {
                        FormFieldType::File
                    } else {
                        FormFieldType::Text
                    },
                    content_type: f.content_type.as_ref().map(|ct| resolve(ct, vars)),
                })
                .collect();
            if fields.is_empty() {
                ResolvedBody::None
            } else {
                ResolvedBody::FormMultipart(fields)
            }
        }

        _ => {
            // Raw kinds send the source body unchanged.
            if let Some(ref body) = parsed.body {
                let ct = match mode {
                    "raw/json" => "application/json",
                    "raw/xml" => "application/xml",
                    "raw/html" => "text/html",
                    "raw/text" => "text/plain",
                    _ => "text/plain",
                };
                ResolvedBody::Raw {
                    content: resolve(body, vars),
                    content_type: ct.to_string(),
                }
            } else {
                ResolvedBody::None
            }
        }
    }
}

fn compile_headers_ctx(
    raw: &[crate::http_parser::ParsedHeaderField],
    ctx: &ExecutionContext,
) -> Vec<HeaderField> {
    raw.iter()
        .filter(|h| !h.key.is_empty() && (h.enabled || h.auto))
        .map(|h| HeaderField {
            key: ctx.resolve(&h.key),
            value: ctx.resolve(&h.value),
            enabled: h.enabled,
            auto: h.auto,
        })
        .collect()
}

fn compile_query_params_ctx(
    raw: &[crate::http_parser::ParsedQueryField],
    ctx: &ExecutionContext,
) -> Vec<QueryField> {
    raw.iter()
        .filter(|q| q.enabled && !q.key.is_empty())
        .map(|q| QueryField {
            key: ctx.resolve(&q.key),
            value: ctx.resolve(&q.value),
            enabled: true,
        })
        .collect()
}

fn compile_body_ctx(parsed: &ParsedRequest, ctx: &ExecutionContext) -> ResolvedBody {
    let mode = effective_body_kind(
        &parsed.headers,
        parsed.body_directive.as_deref(),
        parsed.body.as_deref(),
    );

    match mode {
        "none" => ResolvedBody::None,

        "form-urlencoded" => {
            let fields: Vec<ResolvedFormField> = parsed
                .form_urlencoded
                .iter()
                .filter(|f| f.enabled)
                .map(|f| ResolvedFormField {
                    key: ctx.resolve(&f.key),
                    value: ctx.resolve(&f.value),
                    field_type: FormFieldType::Text,
                    content_type: None,
                })
                .collect();
            if fields.is_empty() {
                ResolvedBody::None
            } else {
                ResolvedBody::FormUrlEncoded(fields)
            }
        }

        "form-multipart" => {
            let fields: Vec<ResolvedFormField> = parsed
                .form_multipart
                .iter()
                .filter(|f| f.enabled)
                .map(|f| ResolvedFormField {
                    key: ctx.resolve(&f.key),
                    value: ctx.resolve(&f.value),
                    field_type: if f.field_type == "file" {
                        FormFieldType::File
                    } else {
                        FormFieldType::Text
                    },
                    content_type: f.content_type.as_ref().map(|ct| ctx.resolve(ct)),
                })
                .collect();
            if fields.is_empty() {
                ResolvedBody::None
            } else {
                ResolvedBody::FormMultipart(fields)
            }
        }

        _ => {
            if let Some(ref body) = parsed.body {
                let ct = match mode {
                    "raw/json" => "application/json",
                    "raw/xml" => "application/xml",
                    "raw/html" => "text/html",
                    "raw/text" => "text/plain",
                    _ => "text/plain",
                };
                ResolvedBody::Raw {
                    content: ctx.resolve(body),
                    content_type: ct.to_string(),
                }
            } else {
                ResolvedBody::None
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> Vec<ParsedFileVariable> {
        pairs
            .iter()
            .map(|(k, v)| ParsedFileVariable {
                key: k.to_string(),
                value: v.to_string(),
            })
            .collect()
    }

    // -- helper to build a bare ParsedRequest with Default body --
    fn minimal(method: &str, url: &str) -> ParsedRequest {
        ParsedRequest {
            method: method.to_string(),
            url: url.to_string(),
            ..Default::default()
        }
    }

    // -----------------------------------------------------------------------
    // Variable resolution
    // -----------------------------------------------------------------------

    #[test]
    fn url_substitution() {
        let parsed = minimal("GET", "{{host}}/{{version}}/users");
        let exe = compile(
            &parsed,
            &vars(&[("host", "https://example.com"), ("version", "v2")]),
        )
        .unwrap();
        assert_eq!(exe.url, "https://example.com/v2/users");
    }

    #[test]
    fn header_value_substitution() {
        use crate::http_parser::ParsedHeaderField;
        let parsed = ParsedRequest {
            headers: vec![ParsedHeaderField {
                key: "Authorization".into(),
                value: "Bearer {{token}}".into(),
                enabled: true,
                auto: false,
            }],
            ..minimal("GET", "https://example.com")
        };
        let exe = compile(&parsed, &vars(&[("token", "abc123")])).unwrap();
        assert_eq!(exe.headers[0].value, "Bearer abc123");
    }

    #[test]
    fn header_key_substitution() {
        use crate::http_parser::ParsedHeaderField;
        let parsed = ParsedRequest {
            headers: vec![ParsedHeaderField {
                key: "X-{{env}}-Token".into(),
                value: "val".into(),
                enabled: true,
                auto: false,
            }],
            ..minimal("GET", "https://example.com")
        };
        let exe = compile(&parsed, &vars(&[("env", "prod")])).unwrap();
        assert_eq!(exe.headers[0].key, "X-prod-Token");
    }

    #[test]
    fn query_param_substitution() {
        use crate::http_parser::ParsedQueryField as PQ;
        let parsed = ParsedRequest {
            query_params: vec![PQ {
                key: "filter".into(),
                value: "{{status}}".into(),
                enabled: true,
                is_inline: true,
            }],
            ..minimal("GET", "https://example.com")
        };
        let exe = compile(&parsed, &vars(&[("status", "active")])).unwrap();
        assert_eq!(exe.query_params[0].value, "active");
    }

    #[test]
    fn body_raw_json_substitution() {
        let parsed = ParsedRequest {
            body: Some("{\"name\": \"{{user}}\"}".into()),
            body_directive: Some("raw/json".into()),
            ..minimal("POST", "https://example.com")
        };
        let exe = compile(&parsed, &vars(&[("user", "alice")])).unwrap();
        match exe.body {
            ResolvedBody::Raw {
                content,
                content_type,
            } => {
                assert_eq!(content, "{\"name\": \"alice\"}");
                assert_eq!(content_type, "application/json");
            }
            _ => panic!("expected Raw body"),
        }
    }

    #[test]
    fn body_raw_xml_substitution() {
        let parsed = ParsedRequest {
            body: Some("<id>{{id}}</id>".into()),
            body_directive: Some("raw/xml".into()),
            ..minimal("POST", "https://example.com")
        };
        let exe = compile(&parsed, &vars(&[("id", "42")])).unwrap();
        match exe.body {
            ResolvedBody::Raw {
                content,
                content_type,
            } => {
                assert_eq!(content, "<id>42</id>");
                assert_eq!(content_type, "application/xml");
            }
            _ => panic!("expected Raw body"),
        }
    }

    #[test]
    fn body_raw_html_substitution() {
        let parsed = ParsedRequest {
            body: Some("<h1>{{title}}</h1>".into()),
            body_directive: Some("raw/html".into()),
            ..minimal("POST", "https://example.com")
        };
        let exe = compile(&parsed, &vars(&[("title", "Hello")])).unwrap();
        match exe.body {
            ResolvedBody::Raw {
                content,
                content_type,
            } => {
                assert_eq!(content, "<h1>Hello</h1>");
                assert_eq!(content_type, "text/html");
            }
            _ => panic!("expected Raw body"),
        }
    }

    #[test]
    fn body_raw_text_substitution() {
        let parsed = ParsedRequest {
            body: Some("Hello {{name}}".into()),
            body_directive: Some("raw/text".into()),
            ..minimal("POST", "https://example.com")
        };
        let exe = compile(&parsed, &vars(&[("name", "world")])).unwrap();
        match exe.body {
            ResolvedBody::Raw {
                content,
                content_type,
            } => {
                assert_eq!(content, "Hello world");
                assert_eq!(content_type, "text/plain");
            }
            _ => panic!("expected Raw body"),
        }
    }

    #[test]
    fn body_none_when_no_body() {
        let exe = compile(&minimal("GET", "https://example.com"), &[]).unwrap();
        match exe.body {
            ResolvedBody::None => {}
            _ => panic!("expected None body"),
        }
    }

    #[test]
    fn form_urlencoded_substitution() {
        use crate::http_parser::ParsedFormField as PF;
        let parsed = ParsedRequest {
            body_directive: Some("form-urlencoded".into()),
            form_urlencoded: vec![
                PF {
                    key: "user".into(),
                    value: "{{name}}".into(),
                    enabled: true,
                    field_type: "text".into(),
                    content_type: None,
                    is_inline: true,
                },
                PF {
                    key: "role".into(),
                    value: "{{role}}".into(),
                    enabled: true,
                    field_type: "text".into(),
                    content_type: None,
                    is_inline: true,
                },
            ],
            ..minimal("POST", "https://example.com")
        };
        let exe = compile(&parsed, &vars(&[("name", "bob"), ("role", "admin")])).unwrap();
        match exe.body {
            ResolvedBody::FormUrlEncoded(ref fields) => {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].key, "user");
                assert_eq!(fields[0].value, "bob");
                assert_eq!(fields[1].key, "role");
                assert_eq!(fields[1].value, "admin");
            }
            _ => panic!("expected form-urlencoded body"),
        }
    }

    #[test]
    fn form_multipart_substitution() {
        use crate::http_parser::ParsedFormField as PF;
        let parsed = ParsedRequest {
            body_directive: Some("form-multipart".into()),
            form_multipart: vec![PF {
                key: "file".into(),
                value: "{{path}}".into(),
                enabled: true,
                field_type: "file".into(),
                content_type: Some("{{mime}}".into()),
                is_inline: true,
            }],
            ..minimal("POST", "https://example.com")
        };
        let exe = compile(
            &parsed,
            &vars(&[("path", "/tmp/photo.png"), ("mime", "image/png")]),
        )
        .unwrap();
        match exe.body {
            ResolvedBody::FormMultipart(ref fields) => {
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].key, "file");
                assert_eq!(fields[0].value, "/tmp/photo.png");
                assert_eq!(fields[0].content_type, Some("image/png".into()));
            }
            _ => panic!("expected form-multipart body"),
        }
    }

    #[test]
    fn form_multipart_field_type_text() {
        use crate::http_parser::ParsedFormField as PF;
        let parsed = ParsedRequest {
            body_directive: Some("form-multipart".into()),
            form_multipart: vec![PF {
                key: "desc".into(),
                value: "hello".into(),
                enabled: true,
                field_type: "text".into(),
                content_type: None,
                is_inline: true,
            }],
            ..minimal("POST", "https://example.com")
        };
        let exe = compile(&parsed, &[]).unwrap();
        match exe.body {
            ResolvedBody::FormMultipart(ref fields) => {
                assert_eq!(fields[0].field_type, FormFieldType::Text);
            }
            _ => panic!("expected form-multipart body"),
        }
    }

    #[test]
    fn multipart_boundary_variable_substitution() {
        use crate::http_parser::ParsedFormField as PF;
        let parsed = ParsedRequest {
            body_directive: Some("form-multipart".into()),
            multipart_boundary: Some("{{b}}".into()),
            form_multipart: vec![PF {
                key: "name".into(),
                value: "alice".into(),
                enabled: true,
                field_type: "text".into(),
                content_type: None,
                is_inline: true,
            }],
            ..minimal("POST", "https://example.com")
        };
        let exe = compile(&parsed, &vars(&[("b", "myboundary42")])).unwrap();
        assert_eq!(exe.multipart_boundary, Some("myboundary42".into()));
    }

    #[test]
    fn multipart_boundary_auto_when_none() {
        let parsed = ParsedRequest {
            body_directive: Some("form-multipart".into()),
            multipart_boundary: None,
            ..minimal("POST", "https://example.com")
        };
        let exe = compile(&parsed, &[]).unwrap();
        assert_eq!(exe.multipart_boundary, None);
    }

    #[test]
    fn multipart_boundary_explicit() {
        let parsed = ParsedRequest {
            body_directive: Some("form-multipart".into()),
            multipart_boundary: Some("xx".into()),
            ..minimal("POST", "https://example.com")
        };
        let exe = compile(&parsed, &[]).unwrap();
        assert_eq!(exe.multipart_boundary, Some("xx".into()));
    }

    // -----------------------------------------------------------------------
    // Filtering
    // -----------------------------------------------------------------------

    #[test]
    fn disabled_headers_omitted() {
        use crate::http_parser::ParsedHeaderField;
        let parsed = ParsedRequest {
            headers: vec![
                ParsedHeaderField {
                    key: "X-Include".into(),
                    value: "yes".into(),
                    enabled: true,
                    auto: false,
                },
                ParsedHeaderField {
                    key: "X-Skip".into(),
                    value: "no".into(),
                    enabled: false,
                    auto: false,
                },
            ],
            ..minimal("GET", "https://example.com")
        };
        let exe = compile(&parsed, &[]).unwrap();
        assert_eq!(exe.headers.len(), 1);
        assert_eq!(exe.headers[0].key, "X-Include");
    }

    #[test]
    fn disabled_query_params_omitted() {
        use crate::http_parser::ParsedQueryField as PQ;
        let parsed = ParsedRequest {
            query_params: vec![
                PQ {
                    key: "a".into(),
                    value: "1".into(),
                    enabled: true,
                    is_inline: true,
                },
                PQ {
                    key: "b".into(),
                    value: "2".into(),
                    enabled: false,
                    is_inline: true,
                },
            ],
            ..minimal("GET", "https://example.com")
        };
        let exe = compile(&parsed, &[]).unwrap();
        assert_eq!(exe.query_params.len(), 1);
        assert_eq!(exe.query_params[0].key, "a");
    }

    #[test]
    fn disabled_form_fields_omitted() {
        use crate::http_parser::ParsedFormField as PF;
        let parsed = ParsedRequest {
            body_directive: Some("form-urlencoded".into()),
            form_urlencoded: vec![
                PF {
                    key: "x".into(),
                    value: "1".into(),
                    enabled: true,
                    field_type: "text".into(),
                    content_type: None,
                    is_inline: true,
                },
                PF {
                    key: "y".into(),
                    value: "2".into(),
                    enabled: false,
                    field_type: "text".into(),
                    content_type: None,
                    is_inline: true,
                },
            ],
            ..minimal("POST", "https://example.com")
        };
        let exe = compile(&parsed, &[]).unwrap();
        match exe.body {
            ResolvedBody::FormUrlEncoded(ref fields) => {
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].key, "x");
            }
            _ => panic!("expected form-urlencoded body"),
        }
    }

    #[test]
    fn empty_form_fields_becomes_none() {
        let parsed = ParsedRequest {
            body_directive: Some("form-urlencoded".into()),
            form_urlencoded: vec![],
            ..minimal("POST", "https://example.com")
        };
        let exe = compile(&parsed, &[]).unwrap();
        match exe.body {
            ResolvedBody::None => {}
            _ => panic!("expected None body"),
        }
    }

    // -----------------------------------------------------------------------
    // HTTP version
    // -----------------------------------------------------------------------

    #[test]
    fn http_version_1_1() {
        let parsed = ParsedRequest {
            http_version: Some("HTTP/1.1".into()),
            ..minimal("GET", "https://example.com")
        };
        let exe = compile(&parsed, &[]).unwrap();
        assert_eq!(exe.http_version, HttpVersion::Http1);
    }

    #[test]
    fn http_version_2() {
        let parsed = ParsedRequest {
            http_version: Some("HTTP/2".into()),
            ..minimal("GET", "https://example.com")
        };
        let exe = compile(&parsed, &[]).unwrap();
        assert_eq!(exe.http_version, HttpVersion::Http2);
    }

    #[test]
    fn http_version_auto() {
        let parsed = ParsedRequest {
            http_version: None,
            ..minimal("GET", "https://example.com")
        };
        let exe = compile(&parsed, &[]).unwrap();
        assert_eq!(exe.http_version, HttpVersion::Auto);
    }

    // -----------------------------------------------------------------------
    // Unknown variable left as-is
    // -----------------------------------------------------------------------

    #[test]
    fn unknown_variable_preserved() {
        let parsed = minimal("GET", "https://example.com/{{missing}}");
        let exe = compile(&parsed, &vars(&[("host", "ignored")])).unwrap();
        assert_eq!(exe.url, "https://example.com/{{missing}}");
    }

    // -----------------------------------------------------------------------
    // Method parsing
    // -----------------------------------------------------------------------

    #[test]
    fn parse_method_case_insensitive() {
        let parsed = minimal("post", "https://example.com");
        let exe = compile(&parsed, &[]).unwrap();
        assert_eq!(exe.method, HttpMethod::Post);
    }

    #[test]
    fn parse_method_unknown() {
        let parsed = minimal("CONNECT", "https://example.com");
        let exe = compile(&parsed, &[]).unwrap();
        assert!(matches!(exe.method, HttpMethod::Other(ref m) if m == "CONNECT"));
    }
}
