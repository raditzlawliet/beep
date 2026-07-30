//! .http file parser produces structured data with region offsets.

use super::types::*;

/// Parse an .http file into structured data with region offsets.
pub fn parse(content: &str) -> ParsedHttpFile {
    let mut variables = Vec::new();
    let mut requests = Vec::new();

    // Parse file-level variables: lines before the first ### that start with @
    let preamble_end = content.find("\n###");
    let preamble = if let Some(pos) = preamble_end {
        &content[..pos]
    } else if content.starts_with("###") {
        ""
    } else {
        content
    };

    for line in preamble.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('@') {
            let rest = &trimmed[1..];
            if let Some(eq_pos) = rest.find('=') {
                let key = rest[..eq_pos].trim().to_string();
                let value_raw = rest[eq_pos + 1..].trim();
                // Strip trailing inline comment, but only // preceded by
                // whitespace (avoids false match inside https:// or similar).
                let value = if let Some(comment_pos) = value_raw.find(" //") {
                    value_raw[..comment_pos].trim().to_string()
                } else {
                    value_raw.to_string()
                };
                variables.push(ParsedFileVariable { key, value });
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

    ParsedHttpFile {
        variables,
        requests,
    }
}

// ---------------------------------------------------------------------------
// Internal: delimiter & offset helpers
// ---------------------------------------------------------------------------

/// Find the next `###` at the start of a line (supports both \n and \r\n).
fn find_delim(bytes: &[u8], start: usize) -> Option<usize> {
    if start >= bytes.len() {
        return None;
    }
    if start + 3 <= bytes.len() && &bytes[start..start + 3] == b"###" {
        return Some(start);
    }
    // Try \n### first
    let pattern = b"\n###";
    if let Some(p) = bytes[start..].windows(4).position(|w| w == pattern) {
        return Some(start + p + 1);
    }
    // Try \r\n### (CRLF)
    let pattern_crlf = b"\r\n###";
    bytes[start..]
        .windows(5)
        .position(|w| w == pattern_crlf)
        .map(|p| start + p + 2)
}

/// Compute the absolute byte offset of line `line_idx` within a block,
/// given the block's base offset and its lines.
fn line_start_offset(base: usize, block: &str, line_idx: usize) -> usize {
    let block_len = block.len();
    let nl_len = if block.contains("\r\n") { 2 } else { 1 };
    let mut pos = 0;
    for (i, line) in block.lines().enumerate() {
        if i == line_idx {
            return base + pos.min(block_len);
        }
        pos += line.len() + nl_len;
        if pos >= block_len {
            break;
        }
    }
    base + block_len
}

/// Compute the absolute byte offset of the end of line `line_idx`
/// (i.e. one past the line ending).
fn line_end_offset(base: usize, block: &str, line_idx: usize) -> usize {
    let start = line_start_offset(base, block, line_idx);
    let line = block.lines().nth(line_idx).unwrap_or("");
    let nl_len = if block.contains("\r\n") { 2 } else { 1 };
    (start + line.len() + nl_len).min(base + block.len())
}

// ---------------------------------------------------------------------------
// ParsedRequest block parser
// ---------------------------------------------------------------------------

/// Parse a single request block (from ### to next ### or EOF).
fn parse_request_block(block: &str, base: usize) -> ParsedRequest {
    let lines: Vec<&str> = block.lines().collect();

    // Title
    let first_line = lines.first().map(|l| l.trim()).unwrap_or("");
    let title = if first_line.starts_with("###") {
        first_line[3..].trim().to_string()
    } else {
        String::new()
    };

    let mut i = 1usize;

    // Pre-request script
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

    // ParsedRequest line
    let mut method = String::new();
    let mut url = String::new();
    let mut http_version: Option<String> = None;
    let mut request_line_region = Region::default();

    if let Some(rl_idx) = request_line_idx {
        let rl = lines[rl_idx].trim();
        let parts: Vec<&str> = rl.splitn(3, ' ').collect();
        if parts.len() >= 2 {
            method = parts[0].to_uppercase();
            url = parts[1].to_string();
            if parts.len() >= 3 {
                http_version = Some(parts[2].to_uppercase());
            }
        }
        request_line_region = Region::new(
            line_start_offset(base, block, rl_idx),
            line_end_offset(base, block, rl_idx),
        );
        i = rl_idx + 1;
    }

    // Multiline query params
    let query_region_start = if i < lines.len() {
        line_start_offset(base, block, i)
    } else {
        base + block.len()
    };
    let mut query_region_end = query_region_start;
    let mut multiline_query: Vec<(String, String, bool)> = Vec::new();
    let mut had_query_lines = false;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        let (content, disabled) = strip_disable_marker(trimmed);
        if content.starts_with('?') || content.starts_with('&') {
            let kv_str = &content[1..];
            for pair in kv_str.split('&') {
                if let Some((k, v)) = parse_kv_pair(pair) {
                    multiline_query.push((k.to_string(), v.to_string(), disabled));
                }
            }
            query_region_end = line_end_offset(base, block, i);
            had_query_lines = true;
            i += 1;
        } else {
            break;
        }
    }

    let query_region = if had_query_lines {
        Region::new(query_region_start, query_region_end)
    } else {
        Region::new(query_region_start, query_region_start)
    };

    // Resolve query params
    let (clean_url, url_query) = split_url_query(&url);
    let mut query_params: Vec<ParsedQueryField> = Vec::new();

    // URL params are inline
    for (k, v) in url_query {
        query_params.push(ParsedQueryField {
            key: k,
            value: v,
            enabled: true,
            is_inline: true,
        });
    }
    // Multiline params override/add; always non-inline
    for (k, v, disabled) in multiline_query {
        if let Some(existing) = query_params.iter_mut().find(|q| q.key == k) {
            existing.value = v;
            existing.enabled = !disabled;
            existing.is_inline = false;
        } else {
            query_params.push(ParsedQueryField {
                key: k,
                value: v,
                enabled: !disabled,
                is_inline: false,
            });
        }
    }

    // Headers
    let headers_region_start = if i < lines.len() {
        line_start_offset(base, block, i)
    } else {
        base + block.len()
    };
    let mut headers_region_end = headers_region_start;
    let mut headers = Vec::new();
    let mut body_directive = None;
    let mut body_line_start: Option<usize> = None;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            // blank line = end of headers, start of body after this
            headers_region_end = line_start_offset(base, block, i);
            body_line_start = Some(i + 1);
            break;
        }
        let (content, disabled) = strip_disable_marker(trimmed);
        if !disabled {
            if let Some(kind) = content.strip_prefix("// @body ") {
                let kind = kind.trim();
                if is_body_kind(kind) {
                    body_directive = Some(kind.to_string());
                }
                headers_region_end = line_end_offset(base, block, i);
                i += 1;
                continue;
            }
        }
        // Check for auto-header opt-out: //- @headerAuto Key
        if let Some(key) = content
            .strip_prefix("@headerAuto ")
            .or_else(|| content.strip_prefix("@headerauto "))
        {
            let key = key.trim().to_string();
            if !key.is_empty() {
                headers.push(ParsedHeaderField {
                    key,
                    value: String::new(),
                    enabled: false,
                    auto: true,
                });
            }
            headers_region_end = line_end_offset(base, block, i);
            i += 1;
            continue;
        }
        if let Some(colon) = content.find(':') {
            let key = content[..colon].trim().to_string();
            let value = content[colon + 1..].trim().to_string();
            headers.push(ParsedHeaderField {
                key,
                value,
                enabled: !disabled,
                auto: false,
            });
        }
        headers_region_end = line_end_offset(base, block, i);
        i += 1;
    }

    let headers_region = Region::new(headers_region_start, headers_region_end);

    // Body region starts at the blank separator (headers_region.end), not after it.
    // This ensures the `\n` separator byte is covered by body_region, preventing
    // it from being lost during surgical splicing between regions.
    let body_region_start = if body_line_start.is_some() {
        headers_region_end
    } else {
        base + block.len()
    };

    let mut body_lines: Vec<&str> = Vec::new();
    let mut body_region_end = body_region_start;
    let mut post_script: Option<String> = None;

    if let Some(bs) = body_line_start {
        let mut j = bs;
        while j < lines.len() {
            let trimmed = lines[j].trim();
            if trimmed.starts_with(">>") {
                j += 1;
                continue;
            }
            if trimmed.starts_with("> {") || trimmed.starts_with("> .") {
                let script_content = extract_script_block(&lines, &mut j, trimmed);
                post_script = script_content;
                j += 1;
                continue;
            }
            body_lines.push(lines[j]);
            body_region_end = line_end_offset(base, block, j);
            j += 1;
        }
    }

    // When a blank separator exists but body is empty and the block ends
    // with \n, str::lines() omits the trailing empty line.
    // Extend the region to cover the separator byte up to the block end.
    let body_region = if body_lines.is_empty() && body_line_start.is_some() {
        Region::new(body_region_start, base + block.len())
    } else {
        Region::new(body_region_start, body_region_end)
    };

    // When body region extends to the block end, back off by 1 byte to leave
    // the trailing `\n` (blank line before next `###`) outside the region.
    // This ensures surgical body replacement always preserves the inter-request
    // blank line as trailing content.
    let blk_end = base + block.len();
    let body_region = if body_region.end == blk_end
        && body_region.end > body_region.start
        && block.ends_with('\n')
    {
        Region::new(body_region.start, body_region.end - 1)
    } else {
        body_region
    };

    let body = if body_lines.is_empty() {
        None
    } else {
        Some(body_lines.join("\n"))
    };

    let body_kind = effective_body_kind(&headers, body_directive.as_deref(), body.as_deref());
    let (form_urlencoded, form_multipart, _body_boundary) =
        parse_body_fields(body.as_deref(), Some(body_kind));

    // Boundary: from Content-Type header param only.
    // No boundary param = auto (None). Body `--boundary` lines are a code detail.
    let multipart_boundary = header_boundary(&headers);

    let block_end = base + block.len();

    ParsedRequest {
        title,
        method,
        url: clean_url,
        headers,
        query_params,
        body,
        body_directive,
        form_urlencoded,
        form_multipart,
        multipart_boundary,
        pre_script,
        post_script,
        http_version,
        block_region: Region::new(base, block_end),
        request_line_region,
        query_region,
        headers_region,
        body_region,
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

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

fn parse_query_string(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter_map(|pair| {
            let (k, v) = parse_kv_pair(pair)?;
            Some((k.to_string(), v.to_string()))
        })
        .collect()
}

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

/// Strip the `//-` disable marker. Returns (content, is_disabled).
/// Accepts both `//- ` (with space) and `//-` (without space).
pub fn strip_disable_marker(trimmed: &str) -> (&str, bool) {
    if let Some(rest) = trimmed.strip_prefix("//-") {
        let stripped = rest.strip_prefix(' ').unwrap_or(rest);
        (stripped, true)
    } else {
        (trimmed, false)
    }
}

fn parse_body_fields(
    body: Option<&str>,
    body_kind: Option<&str>,
) -> (Vec<ParsedFormField>, Vec<ParsedFormField>, Option<String>) {
    let body = match body {
        Some(b) if !b.is_empty() => b,
        _ => return (Vec::new(), Vec::new(), None),
    };

    match body_kind {
        Some("form-urlencoded") => (parse_urlencoded_body(body), Vec::new(), None),
        Some("form-multipart") => {
            let (fields, boundary) = parse_multipart_body(body);
            (Vec::new(), fields, boundary)
        }
        _ => (Vec::new(), Vec::new(), None),
    }
}

fn parse_urlencoded_body(body: &str) -> Vec<ParsedFormField> {
    let mut fields: Vec<ParsedFormField> = Vec::new();
    let lines: Vec<&str> = body.lines().collect();

    // Determine if this is inline (single-line with `&` joins) or multiline.
    // First non-empty, non-disabled-marker line tells us.
    let first_content_line = lines.iter().map(|l| l.trim()).find(|t| !t.is_empty());
    let is_inline = match first_content_line {
        Some(t) => {
            let (content, _) = strip_disable_marker(t);
            !content.starts_with('?') && !content.starts_with('&') && content.contains('&')
        }
        None => true,
    };

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let (content, disabled) = strip_disable_marker(trimmed);
        let cleaned = content.strip_prefix('?').unwrap_or(content);
        let cleaned = cleaned.strip_prefix('&').unwrap_or(cleaned);
        for pair in cleaned.split('&') {
            if let Some((k, v)) = parse_kv_pair(pair) {
                // When a multiline field key matches an existing single-line key,
                // override value and mark as non-inline.
                if let Some(existing) = fields.iter_mut().find(|f| f.key == k) {
                    existing.value = v.to_string();
                    existing.enabled = !disabled;
                    existing.is_inline = false;
                } else {
                    fields.push(ParsedFormField {
                        key: k.to_string(),
                        value: v.to_string(),
                        enabled: !disabled,
                        is_inline,
                        field_type: "text".to_string(),
                        content_type: None,
                    });
                }
            }
        }
    }
    fields
}

fn parse_multipart_body(body: &str) -> (Vec<ParsedFormField>, Option<String>) {
    let first_line = body.lines().next().unwrap_or("").trim();
    let (first_content, _) = strip_disable_marker(first_line);
    if !first_content.starts_with("--") {
        return (Vec::new(), None);
    }
    let boundary = &first_content[2..];
    // Handle closing boundary --boundary-- which may appear at the end
    let boundary_trimmed = boundary.strip_suffix("--").unwrap_or(boundary);
    let normalized = body.replace(&format!("//---{}", boundary), &format!("--{}", boundary));

    let mut fields = Vec::new();
    let parts: Vec<&str> = normalized.split(&format!("--{}", boundary)).collect();

    for part in parts.iter().skip(1) {
        let part = part.trim();
        if part.is_empty() || part == "--" {
            continue;
        }

        let first_trimmed = part.lines().next().unwrap_or("").trim();
        let (_, disabled) = strip_disable_marker(first_trimmed);

        let mut name = String::new();
        let mut filename = String::new();
        let mut content_type: Option<String> = None;
        let mut value = String::new();
        let mut field_type = "text".to_string();
        let mut in_headers = true;

        for line in part.lines() {
            let trimmed = line.trim();
            let (content, _) = strip_disable_marker(trimmed);
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
                    let ct_val = content["content-type:".len()..].trim().to_string();
                    content_type = Some(ct_val);
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
                value.trim_end().to_string()
            };
            fields.push(ParsedFormField {
                key: name,
                value: final_value,
                enabled: !disabled,
                // Multipart is always multiline (is_inline defaults to true via serde,
                // but the concept doesn't apply here).
                is_inline: true,
                field_type,
                content_type,
            });
        }
    }

    (fields, Some(boundary_trimmed.to_string()))
}

fn parse_kv_pair_quoted(input: &str) -> Option<(&str, &str)> {
    let input = input.trim();
    if let Some(eq) = input.find('=') {
        let key = input[..eq].trim();
        let val = input[eq + 1..].trim();
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

pub fn is_body_kind(kind: &str) -> bool {
    matches!(
        kind,
        "none"
            | "raw/json"
            | "raw/xml"
            | "raw/html"
            | "raw/text"
            | "form-urlencoded"
            | "form-multipart"
    )
}

/// Extract boundary from Content-Type header's `boundary=...` parameter.
fn header_boundary(headers: &[ParsedHeaderField]) -> Option<String> {
    for h in headers {
        if h.key.eq_ignore_ascii_case("content-type") && h.enabled {
            let lower = h.value.to_lowercase();
            if let Some(idx) = lower.find("boundary=") {
                let after = &h.value[idx + "boundary=".len()..];
                let end = after
                    .find(|c: char| c == ';' || c.is_whitespace())
                    .unwrap_or(after.len());
                let boundary = after[..end].trim();
                if !boundary.is_empty() {
                    return Some(boundary.to_string());
                }
            }
        }
    }
    None
}

pub fn effective_body_kind<'a>(
    headers: &[ParsedHeaderField],
    directive: Option<&'a str>,
    body: Option<&str>,
) -> &'a str {
    if let Some(kind) = directive.filter(|kind| is_body_kind(kind)) {
        return kind;
    }
    let ct = headers
        .iter()
        .filter(|h| h.enabled)
        .find(|h| h.key.eq_ignore_ascii_case("content-type"))
        .map(|h| h.value.to_lowercase());

    match ct.as_deref() {
        Some(ct) if ct.contains("json") => "raw/json",
        Some(ct) if ct.contains("xml") => "raw/xml",
        Some(ct) if ct.contains("text/html") => "raw/html",
        Some(ct) if ct.contains("application/x-www-form-urlencoded") => "form-urlencoded",
        Some(ct) if ct.contains("multipart/form-data") => "form-multipart",
        Some(_) => "raw/text",
        None => {
            if body.map_or(true, |b| b.is_empty()) {
                "none"
            } else {
                "raw/text"
            }
        }
    }
}

// TODO script is only parsed, need implementation executor or validator
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
