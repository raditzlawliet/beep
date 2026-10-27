//! Compiler: `...Parsed` + variables = `Executable...`.
//!
//! This is a pure transformation; no I/O, no network, no side effects.
//! Variable interpolation happens here.
use crate::context::{ExecutionContext, VarStore, resolve_template, resolve_template_with};
use crate::dynamic::{EnvGuard, scan_process_env};
use crate::executable::{ExecutableRequest, FormFieldType, ResolvedBody, ResolvedFormField};
use crate::http_parser::{ParsedFileVariable, ParsedRequest, effective_body_kind};
use crate::types::{HeaderField, HttpMethod, HttpVersion, QueryField};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compile a `ParsedRequest` into an `ExecutableRequest` using a single variable scope.
/// Same resolution rules as `compile_with_ctx` (nested, trimmed, `$dynamic`).
pub fn compile(
    parsed: &ParsedRequest,
    variables: &[ParsedFileVariable],
) -> Result<ExecutableRequest, String> {
    let store = VarStore::from_file_vars(variables);
    compile_impl(parsed, &|s| resolve_template(s, &[&store]), None)
}

/// Compile using `ExecutionContext` for variable resolution with the full cascade
/// (req.vars > client.vars > request @var > file @var > folder vars).
pub fn compile_with_ctx(
    parsed: &ParsedRequest,
    ctx: &ExecutionContext,
) -> Result<ExecutableRequest, String> {
    // Build the file scope once instead of per resolved string.
    let file = VarStore::from_file_vars(&ctx.file_vars);
    let scopes = [
        &ctx.request_vars,
        &ctx.client_vars,
        &ctx.request_static_vars,
        &file,
        &ctx.folder_vars,
    ];
    compile_impl(
        parsed,
        &|s| resolve_template_with(s, &scopes, Some(&ctx.env)),
        ctx.source_file_dir.clone(),
    )
}

/// `$processEnv` names a send of `parsed` would read, sorted.
///
/// Resolves every template before scripts run (request `@var` from `parsed`, no `req.vars`)
/// and scans the script text. Values set by scripts at runtime are not seen here; those
/// still hit `ctx.env` at send time.
pub fn find_process_env(parsed: &ParsedRequest, ctx: &ExecutionContext) -> Vec<String> {
    let probe = EnvGuard::allow_all();
    let request = VarStore::from_file_vars(&parsed.variables);
    let file = VarStore::from_file_vars(&ctx.file_vars);
    let scopes = [&ctx.client_vars, &request, &file, &ctx.folder_vars];
    // Only names are needed; method errors cannot hide env use, so the result is ignored.
    let _ = compile_impl(parsed, &|s| resolve_template_with(s, &scopes, Some(&probe)), None);

    let mut names = probe.used();
    for script in [&parsed.pre_script, &parsed.post_script].into_iter().flatten() {
        names.extend(scan_process_env(script));
    }
    names.sort();
    names.dedup();
    names
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

type Resolve<'a> = &'a dyn Fn(&str) -> String;

fn compile_impl(
    parsed: &ParsedRequest,
    resolve: Resolve,
    source_file_dir: Option<String>,
) -> Result<ExecutableRequest, String> {
    let method = parse_method(&parsed.method)?;
    let http_version = parse_version(parsed.http_version.as_deref());
    let url = resolve(&parsed.url);
    let headers = compile_headers(&parsed.headers, resolve);
    let query_params = compile_query_params(&parsed.query_params, resolve);
    let body = compile_body(parsed, resolve);
    let multipart_boundary = parsed.multipart_boundary.as_deref().map(resolve);

    Ok(ExecutableRequest {
        url,
        method,
        http_version,
        headers,
        query_params,
        body,
        multipart_boundary,
        source_file_dir,
    })
}

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
// Sub-compilers
// ---------------------------------------------------------------------------

fn compile_headers(
    raw: &[crate::http_parser::ParsedHeaderField],
    resolve: Resolve,
) -> Vec<HeaderField> {
    raw.iter()
        .filter(|h| !h.key.is_empty() && (h.enabled || h.auto))
        .map(|h| HeaderField {
            key: resolve(&h.key),
            value: resolve(&h.value),
            enabled: h.enabled,
            auto: h.auto,
        })
        .collect()
}

fn compile_query_params(
    raw: &[crate::http_parser::ParsedQueryField],
    resolve: Resolve,
) -> Vec<QueryField> {
    raw.iter()
        .filter(|q| q.enabled && !q.key.is_empty())
        .map(|q| QueryField {
            key: resolve(&q.key),
            value: resolve(&q.value),
            enabled: true,
        })
        .collect()
}

fn compile_body(parsed: &ParsedRequest, resolve: Resolve) -> ResolvedBody {
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
                    key: resolve(&f.key),
                    value: resolve(&f.value),
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
                    key: resolve(&f.key),
                    value: resolve(&f.value),
                    field_type: if f.field_type == "file" {
                        FormFieldType::File
                    } else {
                        FormFieldType::Text
                    },
                    content_type: f.content_type.as_deref().map(resolve),
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
                    content: resolve(body),
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

    #[test]
    fn compile_uses_scoped_resolver() {
        let parsed = minimal("GET", "{{ baseUrl }}/users/{{$randomInt 7 8}}");
        let exe = compile(
            &parsed,
            &vars(&[("baseUrl", "{{host}}/v1"), ("host", "https://x.io")]),
        )
        .unwrap();
        assert_eq!(exe.url, "https://x.io/v1/users/7");
    }

    #[test]
    fn compile_with_ctx_all_scopes() {
        use crate::history::RequestHistory;
        use std::sync::{Arc, Mutex};

        let mut ctx = ExecutionContext::new(Arc::new(Mutex::new(RequestHistory::new())));
        ctx.folder_vars.set("host".into(), "https://folder.io".into());
        ctx.file_vars = vars(&[("baseUrl", "{{host}}/api"), ("id", "file")]);
        ctx.request_static_vars.set("baseUrl".into(), "{{baseUrl}}/v2".into());
        ctx.client_vars.set("id".into(), "client".into());
        ctx.source_file_dir = Some("/tmp".into());

        let exe = compile_with_ctx(&minimal("GET", "{{baseUrl}}/{{id}}"), &ctx).unwrap();
        assert_eq!(exe.url, "https://folder.io/api/v2/client");
        assert_eq!(exe.source_file_dir.as_deref(), Some("/tmp"));
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

    #[test]
    fn test_find_process_env_nested_and_script() {
        use crate::history::RequestHistory;
        use std::sync::{Arc, Mutex};

        let mut ctx = ExecutionContext::new(Arc::new(Mutex::new(RequestHistory::new())));
        ctx.file_vars = vars(&[("token", "{{$processEnv API_TOKEN}}"), ("unused", "{{$processEnv NOPE}}")]);
        let mut parsed = minimal("GET", "https://x.io/{{token}}");
        parsed.pre_script = Some(r#"beep.interpolate("{{$processEnv HOME}}")"#.into());
        assert_eq!(find_process_env(&parsed, &ctx), vec!["API_TOKEN", "HOME"]);
    }
}
