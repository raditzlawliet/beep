//! Parser and serializer for .http / .rest files.
//!
//! Parses the Beep HTTP file format (based on JetBrains HTTP spec) into
//! structured data, and provides surgical update functions that replace
//! only the changed sections while preserving the rest of the file.

use serde::{Deserialize, Serialize};

/// A file-level variable declared with `@key = value`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileVariable {
    pub key: String,
    pub value: String,
}

/// A parsed header from a request block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpHeaderField {
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// A parsed query parameter from URL or multiline query lines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryField {
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// A parsed form field (urlencoded or multipart).
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

fn default_field_type() -> String {
    "text".to_string()
}

/// A single parsed request block from an .http file.
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
    /// Character offset where this request block starts in the original content.
    pub offset_start: usize,
    /// Character offset where this request block ends in the original content.
    pub offset_end: usize,
    /// Pre-request script content, if any.
    pub pre_script: Option<String>,
    /// Post-request script content, if any.
    pub post_script: Option<String>,
}

/// Result of parsing an .http file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseHttpFileResult {
    /// File-level @var declarations.
    pub variables: Vec<FileVariable>,
    /// Parsed request blocks.
    pub requests: Vec<ParsedRequest>,
}

/// Parse an .http file into structured data.
pub fn parse_http_file(content: &str) -> ParseHttpFileResult {
    let mut variables = Vec::new();
    let mut requests = Vec::new();

    // Parse file-level variables: lines before the first ### that start with @
    let preamble_end = content.find("\n###");
    let preamble = if let Some(pos) = preamble_end {
        &content[..pos]
    } else if content.starts_with("###") {
        // no file-level content
        ""
    } else {
        content
    };

    // parsing file-level variables
    for line in preamble.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('@') {
            let rest = &trimmed[1..];
            if let Some(eq_pos) = rest.find('=') {
                let key = rest[..eq_pos].trim().to_string();
                let value_raw = rest[eq_pos + 1..].trim();
                let value = if let Some(comment_pos) = value_raw.find("//") {
                    value_raw[..comment_pos].trim().to_string()
                } else {
                    value_raw.to_string()
                };
                variables.push(FileVariable { key, value });
            }
        }
    }

    // Parse request blocks separated by ###
    let mut search_start: usize = 0;
    let bytes = content.as_bytes();

    loop {
        let delim_pos = find_delim(bytes, search_start);
        if delim_pos.is_none() {
            break;
        }
        let block_start = delim_pos.unwrap();

        let block_end = find_delim(bytes, block_start + 3).unwrap_or(content.len());

        if block_end > block_start {
            let request = parse_request_block(&content[block_start..block_end], block_start);
            requests.push(request);
        }

        search_start = block_end;
        if block_end >= content.len() {
            break;
        }
    }

    ParseHttpFileResult {
        variables,
        requests,
    }
}

/// Find the next `###` at the start of a line.
fn find_delim(bytes: &[u8], start: usize) -> Option<usize> {
    if start >= bytes.len() {
        return None;
    }
    if start + 3 <= bytes.len() && &bytes[start..start + 3] == b"###" {
        return Some(start);
    }
    let pattern = b"\n###";
    bytes[start..]
        .windows(4)
        .position(|w| w == pattern)
        .map(|p| start + p + 1)
}

/// Parse a single request block (from ### to next ### or EOF).
fn parse_request_block(block: &str, offset: usize) -> ParsedRequest {
    let lines: Vec<&str> = block.lines().collect();

    // Extract title: text after ### on the first line
    let first_line = lines.first().map(|l| l.trim()).unwrap_or("");
    let title = if first_line.starts_with("###") {
        first_line[3..].trim().to_string()
    } else {
        String::new()
    };

    // Skip line 0 (the ### delimiter/title), start scanning from line 1
    let mut i = 1;

    // Find pre-request script
    let mut pre_script: Option<String> = None;
    let mut request_line_idx: Option<usize> = None;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.starts_with('<') {
            let script_content = extract_script_block(&lines, &mut i, trimmed);
            if pre_script.is_none() {
                pre_script = script_content;
            }
            i += 1;
        } else if trimmed.is_empty() || trimmed.starts_with("//") {
            i += 1;
        } else {
            request_line_idx = Some(i);
            break;
        }
    }

    // Parse request line: METHOD URL [HTTP/Version]
    let mut method = String::new();
    let mut url = String::new();

    if let Some(rl_idx) = request_line_idx {
        let rl = lines[rl_idx].trim();
        let parts: Vec<&str> = rl.splitn(3, ' ').collect();
        if parts.len() >= 2 {
            method = parts[0].to_uppercase();
            url = parts[1].to_string();
        }
        i = rl_idx + 1;
    }

    // Parse multiline query params (lines starting with ? or & after request line)
    let mut multiline_query: Vec<(String, String, bool)> = Vec::new();
    while i < lines.len() {
        let trimmed = lines[i].trim();
        let (content, disabled) = strip_disable_marker(trimmed);
        if content.starts_with('?') {
            let kv = &content[1..];
            for pair in kv.split('&') {
                if let Some((k, v)) = parse_kv_pair(pair) {
                    multiline_query.push((k.to_string(), v.to_string(), disabled));
                }
            }
            i += 1;
        } else if content.starts_with('&') {
            let kv = &content[1..];
            for pair in kv.split('&') {
                if let Some((k, v)) = parse_kv_pair(pair) {
                    multiline_query.push((k.to_string(), v.to_string(), disabled));
                }
            }
            i += 1;
        } else {
            break;
        }
    }

    // Parse query string from URL
    let (clean_url, url_query) = split_url_query(&url);
    let mut query_params: Vec<QueryField> = Vec::new();

    // URL query params first
    for (k, v) in url_query {
        query_params.push(QueryField {
            key: k,
            value: v,
            enabled: true,
        });
    }

    // Multiline query params override/add (with disabled flag)
    for (k, v, disabled) in multiline_query {
        if let Some(existing) = query_params.iter_mut().find(|q| q.key == k) {
            existing.value = v;
            existing.enabled = !disabled;
        } else {
            query_params.push(QueryField {
                key: k,
                value: v,
                enabled: !disabled,
            });
        }
    }

    // Parse headers key:value
    let mut headers = Vec::new();
    let mut body_start: Option<usize> = None;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            // empty line; end of header
            body_start = Some(i + 1);
            break;
        }

        let (content, disabled) = strip_disable_marker(trimmed);
        if let Some(colon) = content.find(':') {
            let key = content[..colon].trim().to_string();
            let value = content[colon + 1..].trim().to_string();
            headers.push(HttpHeaderField {
                key,
                value,
                enabled: !disabled,
            });
        }
        i += 1;
    }

    // Parse body and post-request sections
    let mut body_lines: Vec<&str> = Vec::new();
    let mut post_script: Option<String> = None;

    if let Some(bs) = body_start {
        let mut j = bs;
        while j < lines.len() {
            let trimmed = lines[j].trim();
            if trimmed.starts_with(">>") {
                j += 1;
                continue;
            }
            if trimmed.starts_with("> {") || trimmed.starts_with("> ./") {
                let script_content = extract_script_block(&lines, &mut j, trimmed);
                post_script = script_content;
                j += 1;
                continue;
            }
            body_lines.push(lines[j]);
            j += 1;
        }
    }

    let body = if body_lines.is_empty() {
        None
    } else {
        Some(body_lines.join("\n"))
    };

    // Detect body mode and parse structured body fields
    let body_mode = detect_body_mode(&headers, body.as_deref());
    let (form_urlencoded, form_multipart) =
        parse_body_fields(body.as_deref(), body_mode.as_deref());

    let offset_end = if block.is_empty() {
        offset
    } else {
        offset + block.len()
    };
    let offset_end = if offset_end > offset && block.ends_with('\n') {
        offset_end - 1
    } else {
        offset_end
    };

    ParsedRequest {
        title,
        method,
        url: clean_url,
        headers,
        query_params,
        body,
        body_mode,
        form_urlencoded,
        form_multipart,
        offset_start: offset,
        offset_end,
        pre_script,
        post_script,
    }
}

/// Split URL into base URL and query parameters.
fn split_url_query(url: &str) -> (String, Vec<(String, String)>) {
    if let Some(q_pos) = url.find('?') {
        let base = url[..q_pos].to_string();
        let query = &url[q_pos + 1..];
        let params = parse_query_string(query);
        (base, params)
    } else {
        (url.to_string(), Vec::new())
    }
}

/// Parse a query string like "page=1&limit=20&sort=name" into key-value pairs.
fn parse_query_string(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter_map(|pair| {
            let (k, v) = parse_kv_pair(pair)?;
            Some((k.to_string(), v.to_string()))
        })
        .collect()
}

/// Parse a single key=value pair. Returns None if empty.
fn parse_kv_pair(pair: &str) -> Option<(&str, &str)> {
    let pair = pair.trim();
    if pair.is_empty() {
        return None;
    }
    if let Some(eq) = pair.find('=') {
        Some((pair[..eq].trim(), pair[eq + 1..].trim()))
    } else {
        Some((pair, ""))
    }
}

/// Strip the `//-` disable marker from a trimmed line.
/// Returns (content_after_marker, is_disabled).
/// Handles both `//-Key: Val` and `//- Key: Val` (the space is optional).
fn strip_disable_marker(trimmed: &str) -> (&str, bool) {
    if let Some(rest) = trimmed.strip_prefix("//-") {
        let stripped = rest.strip_prefix(' ').unwrap_or(rest);
        (stripped, true)
    } else {
        (trimmed, false)
    }
}

/// Parse body into structured fields based on body mode.
fn parse_body_fields(
    body: Option<&str>,
    body_mode: Option<&str>,
) -> (Vec<FormField>, Vec<FormField>) {
    let body = match body {
        Some(b) if !b.is_empty() => b,
        _ => return (Vec::new(), Vec::new()),
    };

    match body_mode {
        Some("form-urlencoded") => (parse_urlencoded_body(body), Vec::new()),
        Some("form-multipart") => (Vec::new(), parse_multipart_body(body)),
        _ => (Vec::new(), Vec::new()),
    }
}

/// Parse form-urlencoded body (single-line or multiline).
/// Multiline: "key=value\n&key2=value2\n//-&disabled=xxx"
fn parse_urlencoded_body(body: &str) -> Vec<FormField> {
    let mut fields = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let (content, disabled) = strip_disable_marker(trimmed);
        // Strip leading ? or & from multiline format
        let cleaned = content.strip_prefix('?').unwrap_or(content);
        let cleaned = cleaned.strip_prefix('&').unwrap_or(cleaned);
        for pair in cleaned.split('&') {
            if let Some((k, v)) = parse_kv_pair(pair) {
                fields.push(FormField {
                    key: k.to_string(),
                    value: v.to_string(),
                    enabled: !disabled,
                    field_type: "text".to_string(),
                    content_type: String::new(),
                });
            }
        }
    }
    fields
}

/// Parse multipart/form-data body into structured fields.
fn parse_multipart_body(body: &str) -> Vec<FormField> {
    // Find boundary from the first line, stripping //- disable marker
    let first_line = body.lines().next().unwrap_or("").trim();
    let (first_content, _first_disabled) = strip_disable_marker(first_line);
    if !first_content.starts_with("--") {
        return Vec::new();
    }
    let boundary = &first_content[2..]; // strip leading --

    // Normalize: replace //---boundary with --boundary so split works on disabled sections
    let normalized = body.replace(&format!("//---{}", boundary), &format!("--{}", boundary));

    let mut fields = Vec::new();
    let parts: Vec<&str> = normalized.split(&format!("--{}", boundary)).collect();

    for part in parts.iter().skip(1) {
        let part = part.trim();
        if part.is_empty() || part == "--" {
            continue;
        }

        // Detect disabled field: first non-empty line starts with //-
        let first_trimmed = part.lines().next().unwrap_or("").trim();
        let (_stripped, disabled) = strip_disable_marker(first_trimmed);

        let mut name = String::new();
        let mut filename = String::new();
        let mut content_type = String::new();
        let mut value = String::new();
        let mut field_type = "text".to_string();

        let mut in_headers = true;
        for line in part.lines() {
            let trimmed = line.trim();
            let (content, _disabled) = strip_disable_marker(trimmed);
            if in_headers {
                if content.is_empty() {
                    in_headers = false;
                    continue;
                }
                let lower = content.to_lowercase();
                if lower.starts_with("content-disposition:") {
                    let header_val = content["content-disposition:".len()..].trim();
                    for param in header_val.split(';') {
                        let param = param.trim();
                        if let Some((k, v)) = parse_kv_pair_quoted(param) {
                            match k {
                                "name" => name = v.to_string(),
                                "filename" => {
                                    filename = v.to_string();
                                    field_type = "file".to_string();
                                }
                                _ => {}
                            }
                        }
                    }
                } else if lower.starts_with("content-type:") {
                    content_type = content["content-type:".len()..].trim().to_string();
                }
            } else {
                if !value.is_empty() {
                    value.push('\n');
                }
                value.push_str(content);
            }
        }

        if !name.is_empty() {
            let final_value = if field_type == "file" {
                filename
            } else {
                value
            };
            fields.push(FormField {
                key: name,
                value: final_value,
                enabled: !disabled,
                field_type,
                content_type,
            });
        }
    }

    fields
}

/// Parse a key=value pair from a header parameter like `name="field"` or `name=value`.
fn parse_kv_pair_quoted(input: &str) -> Option<(&str, &str)> {
    let input = input.trim();
    if let Some(eq) = input.find('=') {
        let key = input[..eq].trim();
        let val = input[eq + 1..].trim();
        // Strip surrounding quotes
        let val = if val.len() >= 2
            && ((val.starts_with('"') && val.ends_with('"'))
                || (val.starts_with('\'') && val.ends_with('\'')))
        {
            &val[1..val.len() - 1]
        } else {
            val
        };
        Some((key, val))
    } else {
        None
    }
}

fn detect_body_mode(headers: &[HttpHeaderField], body: Option<&str>) -> Option<String> {
    if body.is_none() || body.unwrap().is_empty() {
        return Some("none".to_string());
    }
    let body = body.unwrap();
    let ct = headers
        .iter()
        .find(|h| h.key.eq_ignore_ascii_case("content-type"))
        .map(|h| h.value.to_lowercase());

    match ct.as_deref() {
        Some(ct) if ct.contains("application/json") => Some("raw/json".to_string()),
        Some(ct) if ct.contains("application/xml") || ct.contains("text/xml") => {
            Some("raw/xml".to_string())
        }
        Some(ct) if ct.contains("text/html") => Some("raw/html".to_string()),
        Some(ct) if ct.contains("application/x-www-form-urlencoded") => {
            Some("form-urlencoded".to_string())
        }
        Some(ct) if ct.contains("multipart/form-data") => Some("form-multipart".to_string()),
        _ => {
            let trimmed = body.trim();
            if (trimmed.starts_with('{') || trimmed.starts_with('['))
                && serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
            {
                Some("raw/json".to_string())
            } else if trimmed.starts_with("<?xml") {
                Some("raw/xml".to_string())
            } else {
                Some("raw/text".to_string())
            }
        }
    }
}

/// Extract a script block (pre or post). Handles both inline `{%%}` and `./file.js`.
fn extract_script_block(lines: &[&str], idx: &mut usize, first_line: &str) -> Option<String> {
    let first = first_line.trim();
    let after_lt = &first[1..].trim();

    if after_lt.starts_with("./") || after_lt.starts_with(".\\") {
        return Some(after_lt.to_string());
    }

    if after_lt.starts_with("{%") {
        let mut script = after_lt[2..].to_string();
        if script.contains("%}") {
            if let Some(end) = script.find("%}") {
                script = script[..end].trim().to_string();
            }
            return Some(script);
        }
        *idx += 1;
        while *idx < lines.len() {
            let line = lines[*idx];
            if line.trim().contains("%}") {
                if let Some(end) = line.find("%}") {
                    script.push('\n');
                    script.push_str(&line[..end].trim());
                }
                break;
            }
            script.push('\n');
            script.push_str(line);
            *idx += 1;
        }
        return Some(script);
    }

    None
}

// Serializers

/// Serialize top level file variables
pub fn serialize_file_variables(variables: &[FileVariable]) -> String {
    variables
        .iter()
        .map(|v| format!("@{} = {}", v.key, v.value))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Serialize a single request block into .http text format.
pub fn serialize_request_block(req: &ParsedRequest) -> String {
    let mut lines = Vec::new();

    // ### Title
    if req.title.is_empty() {
        lines.push("###".to_string());
    } else {
        lines.push(format!("### {}", req.title));
    }

    // Pre-request script
    if let Some(ref pre) = req.pre_script {
        if pre.contains('\n') {
            lines.push("< {%".to_string());
            for pre_line in pre.lines() {
                lines.push(pre_line.to_string());
            }
            lines.push("%}".to_string());
        } else {
            lines.push(format!("< {{%\n{}\n%}}", pre));
        }
    }

    // Query params: enabled in URL, disabled as multiline lines
    let enabled_params: Vec<_> = req.query_params.iter().filter(|q| q.enabled).collect();
    let disabled_params: Vec<_> = req.query_params.iter().filter(|q| !q.enabled).collect();

    let url_with_query = if enabled_params.is_empty() {
        req.url.clone()
    } else {
        let qs: Vec<String> = enabled_params
            .iter()
            .map(|q| format!("{}={}", q.key, q.value))
            .collect();
        format!("{}?{}", req.url, qs.join("&"))
    };
    lines.push(format!("{} {}", req.method, url_with_query));

    // Multiline disabled query params
    for q in &disabled_params {
        lines.push(format!("//- &{}={}", q.key, q.value));
    }

    // Headers
    for h in &req.headers {
        let prefix = if h.enabled { "" } else { "//- " };
        lines.push(format!("{}{}: {}", prefix, h.key, h.value));
    }

    // Body
    match req.body_mode.as_deref() {
        Some("form-urlencoded") if !req.form_urlencoded.is_empty() => {
            lines.push(String::new());
            for f in &req.form_urlencoded {
                let prefix = if f.enabled { "" } else { "//- " };
                lines.push(format!("{}&{}={}", prefix, f.key, f.value));
            }
        }
        Some("form-multipart") if !req.form_multipart.is_empty() => {
            lines.push(String::new());
            let boundary = "boundary";
            for f in &req.form_multipart {
                let p = if f.enabled { "" } else { "//- " };
                lines.push(format!("{}--{}", p, boundary));
                if f.field_type == "file" {
                    lines.push(format!(
                        "{}Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"",
                        p, f.key, f.value
                    ));
                    if !f.content_type.is_empty() {
                        lines.push(format!("{}Content-Type: {}", p, f.content_type));
                    }
                    lines.push(format!("{}< ./{}", p, f.value));
                } else {
                    lines.push(format!(
                        "{}Content-Disposition: form-data; name=\"{}\"",
                        p, f.key
                    ));
                    lines.push(format!("{}", p));
                    lines.push(format!("{}{}", p, f.value));
                }
            }
            lines.push(format!("--{}--", boundary));
        }
        _ => {
            if let Some(ref body) = req.body {
                if !body.is_empty() {
                    lines.push(String::new());
                    for body_line in body.lines() {
                        lines.push(body_line.to_string());
                    }
                }
            }
        }
    }

    // Post-request script
    if let Some(ref post) = req.post_script {
        if post.contains('\n') {
            lines.push("> {%".to_string());
            for post_line in post.lines() {
                lines.push(post_line.to_string());
            }
            lines.push("%}".to_string());
        } else {
            lines.push(format!("> {{%\n{}\n%}}", post));
        }
    }

    lines.join("\n") + "\n" // trailing newline ensures gap between ### blocks
}

pub fn apply_variable_update(content: &str, variables: &[FileVariable]) -> String {
    let first_delim = content.find("\n###").unwrap_or(content.len());

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
            result.push('\n');
        }
        result.push_str(line);
    }

    let var_text = serialize_file_variables(variables);
    if !var_text.is_empty() {
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(&var_text);
    }

    result.push_str(suffix);
    result
}

pub fn apply_request_update(content: &str, request_idx: usize, updated: &ParsedRequest) -> String {
    let parse_result = parse_http_file(content);
    if request_idx >= parse_result.requests.len() {
        return content.to_string();
    }

    let old_req = &parse_result.requests[request_idx];
    let new_text = serialize_request_block(updated);

    let mut result = String::with_capacity(content.len());
    result.push_str(&content[..old_req.offset_start]);
    result.push_str(&new_text);

    if old_req.offset_end < content.len() {
        result.push_str(&content[old_req.offset_end..]);
    }

    result
}

pub fn append_request_block(content: &str, new_request: &ParsedRequest) -> String {
    let block_text = serialize_request_block(new_request);
    let trimmed = content.trim_end();

    if trimmed.is_empty() {
        return block_text;
    }

    let separator = if content.ends_with("\n\n") {
        "\n"
    } else if content.ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };

    format!("{}{}{}", trimmed, separator, block_text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_empty_parsed() -> ParsedRequest {
        ParsedRequest {
            title: String::new(),
            method: String::new(),
            url: String::new(),
            headers: vec![],
            query_params: vec![],
            body: None,
            body_mode: Some("none".to_string()),
            form_urlencoded: vec![],
            form_multipart: vec![],
            offset_start: 0,
            offset_end: 0,
            pre_script: None,
            post_script: None,
        }
    }

    #[test]
    fn test_parse_empty() {
        let result = parse_http_file("");
        assert!(result.variables.is_empty());
        assert!(result.requests.is_empty());
    }

    #[test]
    fn test_parse_single_request() {
        let content = "
### Get Users
GET https://api.example.com/users HTTP/1.1
Accept: application/json
";
        let result = parse_http_file(content);
        assert_eq!(result.requests.len(), 1);
        assert_eq!(result.requests[0].title, "Get Users");
        assert_eq!(result.requests[0].method, "GET");
        assert_eq!(result.requests[0].url, "https://api.example.com/users");
        assert_eq!(result.requests[0].headers.len(), 1);
        assert_eq!(result.requests[0].headers[0].key, "Accept");
    }

    #[test]
    fn test_parse_with_variables() {
        let content = "
@baseUrl = https://api.example.com
@token = abc123

### Login
POST {{baseUrl}}/login HTTP/1.1
Authorization: Bearer {{token}}
Content-Type: application/json

{\"user\": \"test\"}
";
        let result = parse_http_file(content);
        assert_eq!(result.variables.len(), 2);
        assert_eq!(result.variables[0].key, "baseUrl");
        assert_eq!(result.variables[1].key, "token");
        assert_eq!(result.requests.len(), 1);
        assert_eq!(result.requests[0].method, "POST");
        assert!(result.requests[0].body.is_some());
    }

    #[test]
    fn test_parse_multiple_requests() {
        let content = "
### First
GET https://example.com/1 HTTP/1.1

### Second
POST https://example.com/2 HTTP/1.1

{\"key\": \"value\"}
### Third (edge-case)
POST https://example.com/2 HTTP/2

{\"key\": \"value\"}
";
        let result = parse_http_file(content);
        assert_eq!(result.requests.len(), 3);
        assert_eq!(result.requests[0].title, "First");
        assert_eq!(result.requests[1].title, "Second");
        assert_eq!(result.requests[2].method, "POST");
        assert!(result.requests[1].body.is_some());
        assert!(result.requests[2].body.is_some());
    }

    #[test]
    fn test_parse_with_comments() {
        let content = "
// File comment
@host = example.com

### Request
// Inline comment
GET https://{{host}}/api HTTP/1.1
";
        let result = parse_http_file(content);
        assert_eq!(result.variables.len(), 1);
        assert_eq!(result.requests.len(), 1);
    }

    // Query string tests

    #[test]
    fn test_parse_query_string_inline() {
        let content = "
### Search
GET https://api.example.com/users?page=1&limit=20&sort=name HTTP/1.1
";
        let result = parse_http_file(content);
        assert_eq!(result.requests.len(), 1);
        assert_eq!(result.requests[0].url, "https://api.example.com/users");
        assert_eq!(result.requests[0].query_params.len(), 3);
        assert_eq!(result.requests[0].query_params[0].key, "page");
        assert_eq!(result.requests[0].query_params[0].value, "1");
        assert_eq!(result.requests[0].query_params[1].key, "limit");
        assert_eq!(result.requests[0].query_params[2].key, "sort");
    }

    #[test]
    fn test_parse_query_string_multiline() {
        let content = "
### Search
GET https://api.example.com/users HTTP/1.1
    ?page=1
    &limit=20
    &sort=name
Accept: application/json
";
        let result = parse_http_file(content);
        assert_eq!(result.requests.len(), 1);
        assert_eq!(result.requests[0].url, "https://api.example.com/users");
        assert_eq!(result.requests[0].query_params.len(), 3);
        assert_eq!(result.requests[0].query_params[0].key, "page");
        assert_eq!(result.requests[0].query_params[1].key, "limit");
        assert_eq!(result.requests[0].query_params[2].key, "sort");
        // Headers should still be parsed after multiline query
        assert_eq!(result.requests[0].headers.len(), 1);
        assert_eq!(result.requests[0].headers[0].key, "Accept");
    }

    #[test]
    fn test_parse_query_string_combined() {
        let content = "
### Search
GET https://api.example.com/users?page=1 HTTP/1.1
    &limit=20
    &sort=name
";
        let result = parse_http_file(content);
        assert_eq!(result.requests[0].url, "https://api.example.com/users");
        assert_eq!(result.requests[0].query_params.len(), 3);
        assert_eq!(result.requests[0].query_params[0].key, "page");
        assert_eq!(result.requests[0].query_params[1].key, "limit");
        assert_eq!(result.requests[0].query_params[2].key, "sort");
    }

    #[test]
    fn test_parse_query_string_disabled() {
        let content = "
### Search
GET https://api.example.com/users?page=1 HTTP/1.1
    //- &limit=20
    //-&sort=name
";
        let result = parse_http_file(content);
        assert_eq!(result.requests[0].query_params.len(), 3);
        assert_eq!(result.requests[0].query_params[0].enabled, true); // page from URL
        assert_eq!(result.requests[0].query_params[1].enabled, false); // limit
        assert_eq!(result.requests[0].query_params[2].enabled, false); // sort
    }

    // Disabled header test

    #[test]
    fn test_parse_disabled_headers() {
        let content = "
### Test
GET / HTTP/1.1
X-Enabled: val1
//-X-Disabled: val2
//- X-Disabled2: val3
";
        let result = parse_http_file(content);
        assert_eq!(result.requests[0].headers.len(), 3);
        assert_eq!(result.requests[0].headers[0].enabled, true);
        assert_eq!(result.requests[0].headers[1].enabled, false);
        assert_eq!(result.requests[0].headers[2].enabled, false);
        assert_eq!(result.requests[0].headers[1].key, "X-Disabled");
        assert_eq!(result.requests[0].headers[1].value, "val2");
        assert_eq!(result.requests[0].headers[2].key, "X-Disabled2");
        assert_eq!(result.requests[0].headers[2].value, "val3");
    }

    // Disabled urlencoded test

    #[test]
    fn test_parse_form_urlencoded_disabled() {
        let content = "
### Login
POST /login HTTP/1.1
Content-Type: application/x-www-form-urlencoded

username=john
//-&password=secret
&remember=true
";
        let result = parse_http_file(content);
        assert_eq!(result.requests[0].form_urlencoded.len(), 3);
        assert_eq!(result.requests[0].form_urlencoded[0].enabled, true);
        assert_eq!(result.requests[0].form_urlencoded[0].key, "username");
        assert_eq!(result.requests[0].form_urlencoded[1].enabled, false);
        assert_eq!(result.requests[0].form_urlencoded[1].key, "password");
        assert_eq!(result.requests[0].form_urlencoded[2].enabled, true);
        assert_eq!(result.requests[0].form_urlencoded[2].key, "remember");
    }

    // Disabled multipart test

    #[test]
    fn test_parse_multipart_disabled() {
        let content = "
### Upload
POST /upload HTTP/1.1
Content-Type: multipart/form-data; boundary=boundary

--boundary
Content-Disposition: form-data; name=\"name\"

John
//---boundary
//-Content-Disposition: form-data; name=\"email\"
//-
//-disabled@email.com
--boundary--
";
        let result = parse_http_file(content);
        assert_eq!(result.requests[0].form_multipart.len(), 2);
        assert_eq!(result.requests[0].form_multipart[0].enabled, true);
        assert_eq!(result.requests[0].form_multipart[0].key, "name");
        assert_eq!(result.requests[0].form_multipart[1].enabled, false);
        assert_eq!(result.requests[0].form_multipart[1].key, "email");
    }

    #[test]
    fn test_parse_query_string_no_value() {
        let content = "
### Flag
GET https://api.example.com/data?debug HTTP/1.1
";
        let result = parse_http_file(content);
        assert_eq!(result.requests[0].query_params.len(), 1);
        assert_eq!(result.requests[0].query_params[0].key, "debug");
        assert_eq!(result.requests[0].query_params[0].value, "");
    }

    // Form URL-encoded tests

    #[test]
    fn test_parse_form_urlencoded_single_line() {
        let content = "
### Login
POST https://api.example.com/auth/login HTTP/1.1
Content-Type: application/x-www-form-urlencoded

username=john&password=secret&remember=true
";
        let result = parse_http_file(content);
        assert_eq!(
            result.requests[0].body_mode.as_deref(),
            Some("form-urlencoded")
        );
        assert_eq!(result.requests[0].form_urlencoded.len(), 3);
        assert_eq!(result.requests[0].form_urlencoded[0].key, "username");
        assert_eq!(result.requests[0].form_urlencoded[0].value, "john");
        assert_eq!(result.requests[0].form_urlencoded[1].key, "password");
        assert_eq!(result.requests[0].form_urlencoded[1].value, "secret");
        assert_eq!(result.requests[0].form_urlencoded[2].key, "remember");
        assert_eq!(result.requests[0].form_urlencoded[2].value, "true");
    }

    #[test]
    fn test_parse_form_urlencoded_multiline() {
        let content = "
### Login
POST https://api.example.com/auth/login HTTP/1.1
Content-Type: application/x-www-form-urlencoded

username=john
&password=secret
&remember=true
";
        let result = parse_http_file(content);
        assert_eq!(result.requests[0].form_urlencoded.len(), 3);
        assert_eq!(result.requests[0].form_urlencoded[0].key, "username");
        assert_eq!(result.requests[0].form_urlencoded[0].value, "john");
        assert_eq!(result.requests[0].form_urlencoded[1].key, "password");
        assert_eq!(result.requests[0].form_urlencoded[1].value, "secret");
        assert_eq!(result.requests[0].form_urlencoded[2].key, "remember");
        assert_eq!(result.requests[0].form_urlencoded[2].value, "true");
    }

    // Multipart tests

    #[test]
    fn test_parse_multipart_text_fields() {
        let content = "
### Upload
POST https://api.example.com/upload HTTP/1.1
Content-Type: multipart/form-data; boundary=boundary

--boundary
Content-Disposition: form-data; name=\"name\"

John Doe
--boundary
Content-Disposition: form-data; name=\"email\"

john@example.com
--boundary--
";
        let result = parse_http_file(content);
        assert_eq!(
            result.requests[0].body_mode.as_deref(),
            Some("form-multipart")
        );
        assert_eq!(result.requests[0].form_multipart.len(), 2);
        assert_eq!(result.requests[0].form_multipart[0].key, "name");
        assert_eq!(result.requests[0].form_multipart[0].value, "John Doe");
        assert_eq!(result.requests[0].form_multipart[0].field_type, "text");
        assert_eq!(result.requests[0].form_multipart[1].key, "email");
        assert_eq!(
            result.requests[0].form_multipart[1].value,
            "john@example.com"
        );
    }

    #[test]
    fn test_parse_multipart_file_field() {
        let content = "
### Avatar
POST https://api.example.com/users/1/avatar HTTP/1.1
Content-Type: multipart/form-data; boundary=boundary

--boundary
Content-Disposition: form-data; name=\"avatar\"; filename=\"photo.png\"
Content-Type: image/png

< ./photo.png
--boundary--
";
        let result = parse_http_file(content);
        assert_eq!(result.requests[0].form_multipart.len(), 1);
        assert_eq!(result.requests[0].form_multipart[0].key, "avatar");
        assert_eq!(result.requests[0].form_multipart[0].value, "photo.png");
        assert_eq!(result.requests[0].form_multipart[0].field_type, "file");
        assert_eq!(
            result.requests[0].form_multipart[0].content_type,
            "image/png"
        );
    }

    // Serializer tests

    #[test]
    fn test_serialize_roundtrip() {
        let content = "
### Get Users
GET https://api.example.com/users HTTP/1.1
Accept: application/json
";
        let result = parse_http_file(content);
        let serialized = serialize_request_block(&result.requests[0]);
        let reparsed = parse_http_file(&serialized);
        assert_eq!(reparsed.requests[0].method, "GET");
        assert_eq!(reparsed.requests[0].url, "https://api.example.com/users");
    }

    #[test]
    fn test_serialize_query_string_roundtrip() {
        let content = "
### Search
GET https://api.example.com/users?page=1&limit=20 HTTP/1.1
";
        let result = parse_http_file(content);
        let serialized = serialize_request_block(&result.requests[0]);
        let reparsed = parse_http_file(&serialized);
        assert_eq!(reparsed.requests[0].query_params.len(), 2);
        assert_eq!(reparsed.requests[0].query_params[0].key, "page");
        assert_eq!(reparsed.requests[0].query_params[1].key, "limit");
    }

    #[test]
    fn test_serialize_form_urlencoded_roundtrip() {
        let content = "
### Login
POST https://api.example.com/login HTTP/1.1
Content-Type: application/x-www-form-urlencoded

user=john&pass=secret
";
        let result = parse_http_file(content);
        assert_eq!(result.requests[0].form_urlencoded.len(), 2);
        let serialized = serialize_request_block(&result.requests[0]);
        // After roundtrip, body_mode detected again
        let reparsed = parse_http_file(&serialized);
        assert_eq!(reparsed.requests[0].form_urlencoded.len(), 2);
    }

    #[test]
    fn test_apply_variable_update() {
        let content = "
@old = value1
// a comment
### Test
GET /test HTTP/1.1
";
        let new_vars = vec![FileVariable {
            key: "new".to_string(),
            value: "value2".to_string(),
        }];
        let result = apply_variable_update(content, &new_vars);
        assert!(result.contains("@new = value2"));
        assert!(result.contains("// a comment"));
        assert!(result.contains("### Test"));
        assert!(!result.contains("@old = value1"));
    }

    #[test]
    fn test_apply_request_update() {
        let content = "
### First
GET /first HTTP/1.1

### Second
POST /second HTTP/1.1

{\"a\":1}
";
        let parse_result = parse_http_file(content);
        let mut updated = parse_result.requests[1].clone();
        updated.method = "PUT".to_string();
        updated.url = "/updated".to_string();
        let result = apply_request_update(content, 1, &updated);
        assert!(result.contains("### First"));
        assert!(result.contains("PUT /updated"));
        assert!(!result.contains("POST /second"));
    }

    #[test]
    fn test_append_request() {
        let content = "
### First
GET /first HTTP/1.1
";
        let mut new_req = make_empty_parsed();
        new_req.title = "Second".to_string();
        new_req.method = "POST".to_string();
        new_req.url = "/second".to_string();
        let result = append_request_block(content, &new_req);
        assert!(result.contains("### First"));
        assert!(result.contains("### Second"));
    }

    #[test]
    fn test_body_mode_detection_json() {
        let headers = vec![HttpHeaderField {
            key: "Content-Type".to_string(),
            value: "application/json".to_string(),
            enabled: true,
        }];
        let mode = detect_body_mode(&headers, Some("{\"a\": 1}"));
        assert_eq!(mode, Some("raw/json".to_string()));
    }

    #[test]
    fn test_body_mode_detection_none() {
        let mode = detect_body_mode(&[], None);
        assert_eq!(mode, Some("none".to_string()));
    }

    #[test]
    fn test_split_url_query() {
        let (url, params) = split_url_query("https://api.example.com/users?page=1&limit=20");
        assert_eq!(url, "https://api.example.com/users");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0], ("page".to_string(), "1".to_string()));
        assert_eq!(params[1], ("limit".to_string(), "20".to_string()));
    }

    #[test]
    fn test_split_url_no_query() {
        let (url, params) = split_url_query("https://api.example.com/users");
        assert_eq!(url, "https://api.example.com/users");
        assert!(params.is_empty());
    }
}
