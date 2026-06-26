//! Serializers for .http file sections and full blocks.

use super::types::*;

/// Serialize file-level @var lines.
pub fn serialize_file_variables(variables: &[FileVariable]) -> String {
    variables
        .iter()
        .map(|v| format!("@{} = {}", v.key, v.value))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Serialize the request line: `METHOD URL[?inline_params] [HTTP/Version]\n`.
/// Empty-value params use `key` (no `=`).
pub fn serialize_request_line(req: &ParsedRequest) -> String {
    let inline: Vec<_> = req
        .query_params
        .iter()
        .filter(|q| q.enabled && q.is_inline)
        .collect();

    let url_with_query = if inline.is_empty() {
        req.url.clone()
    } else {
        let qs: Vec<String> = inline
            .iter()
            .map(|q| {
                if q.value.is_empty() {
                    q.key.clone()
                } else {
                    format!("{}={}", q.key, q.value)
                }
            })
            .collect();
        format!("{}?{}", req.url, qs.join("&"))
    };

    if let Some(ref ver) = req.http_version {
        format!("{} {} {}\n", req.method, url_with_query, ver)
    } else {
        format!("{} {}\n", req.method, url_with_query)
    }
}

/// Serialize multiline query param lines.
/// `has_inline`, when true (URL already has `?inline`), all multiline
/// params use `&`. When false, first enabled multiline uses `?`.
pub fn serialize_query_section(params: &[QueryField], has_inline: bool) -> String {
    let multiline: Vec<_> = params.iter().filter(|q| !q.is_inline).collect();
    if multiline.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    let mut first_enabled = true;
    for q in &multiline {
        let prefix = if q.enabled { "" } else { "//- " };
        let sep = if q.enabled {
            if has_inline || !first_enabled {
                "&"
            } else {
                first_enabled = false;
                "?"
            }
        } else {
            "&"
        };
        if q.value.is_empty() {
            out.push_str(&format!("{}{}{}\n", prefix, sep, q.key));
        } else {
            out.push_str(&format!("{}{}{}={}\n", prefix, sep, q.key, q.value));
        }
    }
    out
}

/// Serialize header lines. Each line ends with `\n`.
/// Returns empty string if no headers.
pub fn serialize_headers_section(headers: &[HttpHeaderField]) -> String {
    if headers.is_empty() {
        return String::new();
    }
    headers
        .iter()
        .map(|h| {
            let prefix = if h.enabled { "" } else { "//- " };
            format!("{}{}: {}\n", prefix, h.key, h.value)
        })
        .collect::<Vec<_>>()
        .concat()
}

/// Serialize body content for a given mode.
///
/// Returns empty string when there is no body and no post-script.
pub fn serialize_body_section(
    body_mode: Option<&str>,
    body: Option<&str>,
    form_urlencoded: &[FormField],
    form_multipart: &[FormField],
    post_script: Option<&str>,
) -> String {
    let mut out = String::new();

    match body_mode {
        Some("form-urlencoded") if !form_urlencoded.is_empty() => {
            // Mixing inline & multiline causing too much chaos... need to consider for later
            let all_inline = form_urlencoded.iter().all(|f| f.is_inline && f.enabled);
            if all_inline {
                // Single-line: key1=val1&key2=val2
                let qs: Vec<String> = form_urlencoded
                    .iter()
                    .map(|f| format!("{}={}", f.key, f.value))
                    .collect();
                out.push_str(&format!("{}\n", qs.join("&")));
            } else {
                // Multiline: one &key=value per line, disabled get //- &
                for f in form_urlencoded {
                    let prefix = if f.enabled { "" } else { "//- " };
                    out.push_str(&format!("{}&{}={}\n", prefix, f.key, f.value));
                }
            }
        }
        Some("form-multipart") if !form_multipart.is_empty() => {
            let boundary = "boundary";
            for f in form_multipart {
                let p = if f.enabled { "" } else { "//- " };
                out.push_str(&format!("{}--{}\n", p, boundary));
                if f.field_type == "file" {
                    out.push_str(&format!(
                        "{}Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\n",
                        p, f.key, f.value
                    ));
                    if !f.content_type.is_empty() {
                        out.push_str(&format!("{}Content-Type: {}\n", p, f.content_type));
                    }
                    out.push_str(&format!("{}< ./{}\n", p, f.value));
                } else {
                    out.push_str(&format!(
                        "{}Content-Disposition: form-data; name=\"{}\"\n",
                        p, f.key
                    ));
                    out.push_str(&format!("{}\n", p));
                    out.push_str(&format!("{}{}\n", p, f.value));
                }
            }
            out.push_str(&format!("--{}--\n", boundary));
        }
        _ => {
            if let Some(b) = body {
                if !b.is_empty() {
                    if b.ends_with('\n') {
                        out.push_str(b);
                    } else {
                        out.push_str(&format!("{}\n", b));
                    }
                }
            }
        }
    }

    // Post-request script, separated from body by empty line when body is present
    if let Some(post) = post_script {
        if !post.is_empty() {
            if !out.is_empty() && !out.ends_with("\n\n") {
                out.push('\n');
            }
            if post.contains('\n') {
                out.push_str("> {%\n");
                for line in post.lines() {
                    out.push_str(line);
                    out.push('\n');
                }
                out.push_str("%}\n");
            } else {
                out.push_str(&format!("> {{%\n{}\n%}}\n", post));
            }
        }
    }

    if out.is_empty() {
        String::new()
    } else {
        // Prepend blank separator between headers and body
        format!("\n{}", out)
    }
}

/// Full block serializer, used for appending new requests or full-block fallback edits.
pub fn serialize_request_block(req: &ParsedRequest) -> String {
    let mut out = String::new();

    // ### Title
    if req.title.is_empty() {
        out.push_str("###\n");
    } else {
        out.push_str(&format!("### {}\n", req.title));
    }

    // Pre-request script
    if let Some(ref pre) = req.pre_script {
        if pre.contains('\n') {
            out.push_str("< {%\n");
            for line in pre.lines() {
                out.push_str(line);
                out.push('\n');
            }
            out.push_str("%}\n");
        } else {
            out.push_str(&format!("< {{%\n{}\n%}}\n", pre));
        }
    }

    // Request line
    out.push_str(&serialize_request_line(req));

    // Multiline query params
    let multiline: Vec<_> = req.query_params.iter().filter(|q| !q.is_inline).collect();
    if !multiline.is_empty() {
        let has_inline = req.query_params.iter().any(|q| q.is_inline && q.enabled);
        out.push_str(&serialize_query_section(&req.query_params, has_inline));
    }

    // Headers
    out.push_str(&serialize_headers_section(&req.headers));

    // Body + post-script
    let body_text = serialize_body_section(
        req.body_mode.as_deref(),
        req.body.as_deref(),
        &req.form_urlencoded,
        &req.form_multipart,
        req.post_script.as_deref(),
    );
    if !body_text.is_empty() {
        out.push_str(&body_text);
    }

    out
}
