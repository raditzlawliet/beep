//! Surgical edit operations on .http files.

use super::parse::parse_http_file;
use super::serialize::{
    serialize_body_section, serialize_file_variables, serialize_headers_section,
    serialize_query_section, serialize_request_block, serialize_request_line,
};
use super::types::*;

// --- Newline helpers

/// Detect the line-ending convention used in the file content.
fn detect_newline(content: &str) -> &'static str {
    if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

/// Convert any LF characters in `s` to the requested `target` style.
fn normalize_newlines(s: &str, target: &str) -> String {
    if target == "\n" {
        s.to_string()
    } else {
        s.replace('\n', target)
    }
}

// --- Change detection

#[derive(Default)]
struct ChangedSections {
    /// Title or pre_script changed will triggers full-block fallback.
    pre_or_title: bool,
    /// Method, url, http_version, or inline query_params changed.
    request_line: bool,
    /// Multiline query_params changed (is_inline == false).
    query_region: bool,
    /// Headers vec changed.
    headers: bool,
    /// Body, body_mode, form fields, or post_script changed.
    body: bool,
}

fn detect_changed_sections(old: &ParsedRequest, new: &ParsedRequest) -> ChangedSections {
    let old_inline: Vec<_> = old.query_params.iter().filter(|q| q.is_inline).collect();
    let new_inline: Vec<_> = new.query_params.iter().filter(|q| q.is_inline).collect();
    let old_multiline: Vec<_> = old.query_params.iter().filter(|q| !q.is_inline).collect();
    let new_multiline: Vec<_> = new.query_params.iter().filter(|q| !q.is_inline).collect();

    ChangedSections {
        pre_or_title: old.title != new.title || old.pre_script != new.pre_script,
        request_line: old.method != new.method
            || old.url != new.url
            || old.http_version != new.http_version
            || old_inline != new_inline,
        query_region: old_multiline != new_multiline,
        headers: old.headers != new.headers,
        body: old.body != new.body
            || old.body_mode != new.body_mode
            || old.form_urlencoded != new.form_urlencoded
            || old.form_multipart != new.form_multipart
            || old.post_script != new.post_script,
    }
}

// --- Variable update

/// Replace file-level @var declarations while preserving all other content.
pub fn apply_variable_update(content: &str, variables: &[FileVariable]) -> String {
    // Split at the first request block.
    let nl = detect_newline(content);

    // Split at the first request delimiter respecting the detected line ending.
    let first_delim = if content.starts_with("###") {
        0 // no preamble, variables go before the first request
    } else {
        content.find(&format!("{nl}###")).unwrap_or(content.len())
    };

    let prefix = &content[..first_delim];
    let suffix = &content[first_delim..];

    let mut prelude_lines: Vec<&str> = Vec::new();
    for line in prefix.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('@') {
            prelude_lines.push(line);
        }
    }

    let mut result = String::new();
    for (i, line) in prelude_lines.iter().enumerate() {
        if i > 0 {
            result.push_str(nl);
        }
        result.push_str(line);
    }

    let var_text = normalize_newlines(&serialize_file_variables(variables), nl);
    if !var_text.is_empty() {
        if !result.is_empty() {
            result.push_str(nl);
        }
        result.push_str(&var_text);
        // Separate variables from first request with blank line.
        // suffix may start with "<nl>###" (had preamble) or "###".
        if !suffix.starts_with(nl) {
            result.push_str(nl);
        }
        result.push_str(nl);
    }

    result.push_str(suffix);
    result
}

// --- Surgical request update

/// Apply a surgical update to a single request block.
///
/// Uses region offsets to replace only the sections that actually changed.
/// Comments, formatting, and any unrecognized content in unchanged sections
/// are preserved verbatim.
/// Ugh... so much...
pub fn apply_request_update(content: &str, request_idx: usize, updated: &ParsedRequest) -> String {
    let parse_result = parse_http_file(content);
    if request_idx >= parse_result.requests.len() {
        return content.to_string();
    }

    let old_req = &parse_result.requests[request_idx];
    let changed = detect_changed_sections(old_req, updated);
    let nl = detect_newline(content);

    // Full-block fallback for title/pre-script changes
    if changed.pre_or_title {
        let new_text = normalize_newlines(&serialize_request_block(updated), nl);
        let mut result = String::with_capacity(content.len());
        result.push_str(&content[..old_req.block_region.start]);
        result.push_str(&new_text);
        if old_req.block_region.end < content.len() {
            result.push_str(&content[old_req.block_region.end..]);
        }
        return result;
    }

    // Nothing changed at all, return original content
    if !changed.request_line && !changed.query_region && !changed.headers && !changed.body {
        return content.to_string();
    }

    let mut out = String::with_capacity(content.len());

    // 0. Content before this block
    out.push_str(&content[..old_req.block_region.start]);

    // 1. Pre area (title + everything between title and request line)
    out.push_str(&content[old_req.block_region.start..old_req.request_line_region.start]);

    // 2. Request line
    if changed.request_line {
        out.push_str(&normalize_newlines(&serialize_request_line(updated), nl));
    } else {
        out.push_str(&content[old_req.request_line_region.start..old_req.request_line_region.end]);
    }

    // 3. Query region (multiline params only)
    if changed.query_region {
        let has_inline = updated
            .query_params
            .iter()
            .any(|q| q.is_inline && q.enabled);
        out.push_str(&normalize_newlines(
            &serialize_query_section(&updated.query_params, has_inline),
            nl,
        ));
    } else {
        out.push_str(&content[old_req.query_region.start..old_req.query_region.end]);
    }

    // 4. Headers
    if changed.headers {
        out.push_str(&normalize_newlines(
            &serialize_headers_section(&updated.headers),
            nl,
        ));
    } else {
        out.push_str(&content[old_req.headers_region.start..old_req.headers_region.end]);
    }

    // 5. Body region (blank separator + body + post-script)
    if changed.body {
        out.push_str(&normalize_newlines(
            &serialize_body_section(
                updated.body_mode.as_deref(),
                updated.body.as_deref(),
                &updated.form_urlencoded,
                &updated.form_multipart,
                updated.post_script.as_deref(),
            ),
            nl,
        ));
    } else {
        out.push_str(&content[old_req.body_region.start..old_req.body_region.end]);
    }

    // 6. Trailing content within block (between body end and next ###)
    out.push_str(&content[old_req.body_region.end..old_req.block_region.end]);

    // 7. Rest of file after this block - untouched
    if old_req.block_region.end < content.len() {
        out.push_str(&content[old_req.block_region.end..]);
    }

    out
}

// --- Append

/// Append a new request block to an .http file.
pub fn append_request_block(content: &str, new_request: &ParsedRequest) -> String {
    let nl = detect_newline(content);
    let block_text = normalize_newlines(&serialize_request_block(new_request), nl);
    let trimmed = content.trim_end();

    if trimmed.is_empty() {
        return block_text;
    }

    let separator = if content.ends_with(nl) {
        nl.to_string()
    } else {
        format!("{nl}{nl}")
    };
    format!("{}{}{}", trimmed, separator, block_text)
}
