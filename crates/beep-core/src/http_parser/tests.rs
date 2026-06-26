#[cfg(test)]
mod tests {
    use crate::http_parser::*;

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
            pre_script: None,
            post_script: None,
            http_version: None,
            block_region: Region::default(),
            request_line_region: Region::default(),
            query_region: Region::default(),
            headers_region: Region::default(),
            body_region: Region::default(),
        }
    }

    // =======================================================================
    // Parse tests
    // =======================================================================

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
### Third
POST https://example.com/3 HTTP/2

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

    // =======================================================================
    // Region offset tests
    // =======================================================================

    #[test]
    fn test_region_offsets_single_request() {
        let content = "
### Get Users
GET https://api.example.com/users HTTP/1.1
Accept: application/json
";
        let result = parse_http_file(content);
        let req = &result.requests[0];

        assert_eq!(req.block_region.start, 1);
        assert_eq!(req.block_region.end, content.len());
        // request line starts after "### Get Users\n" (14 chars from byte 1)
        assert_eq!(req.request_line_region.start, 15);
        assert!(req.headers_region.start > req.request_line_region.start);
        assert!(req.body_region.is_empty());
    }

    #[test]
    fn test_region_offsets_with_body() {
        let content = "
### Post
POST https://api.example.com/users HTTP/1.1
Content-Type: application/json

{\"a\":1}
";
        let result = parse_http_file(content);
        let req = &result.requests[0];

        assert!(!req.body_region.is_empty());
        assert_eq!(
            &content[req.body_region.start..req.body_region.end],
            "\n{\"a\":1}"
        );
    }

    #[test]
    fn test_region_offsets_multiple_requests() {
        let content = "
### First
GET /first HTTP/1.1

### Second
POST /second HTTP/1.1

{\"x\":1}
";
        let result = parse_http_file(content);
        assert_eq!(result.requests.len(), 2);

        let r0 = &result.requests[0];
        let r1 = &result.requests[1];

        assert!(r0.block_region.end <= r1.block_region.start);
        assert_eq!(
            &content[r1.body_region.start..r1.body_region.end],
            "\n{\"x\":1}"
        );
    }

    #[test]
    fn test_region_query_multiline() {
        let content = "
### Search
GET https://api.example.com/users HTTP/1.1
    ?page=1
    &limit=20
Accept: application/json
";
        let result = parse_http_file(content);
        let req = &result.requests[0];

        assert!(!req.query_region.is_empty());
        let query_slice = &content[req.query_region.start..req.query_region.end];
        assert!(query_slice.contains("?page=1"));
        assert!(query_slice.contains("&limit=20"));
    }

    // =======================================================================
    // Query string tests
    // =======================================================================

    #[test]
    fn test_parse_query_string_inline() {
        let content = "
### Search
GET https://api.example.com/users?page=1&limit=20&sort=name HTTP/1.1
";
        let result = parse_http_file(content);
        assert_eq!(result.requests[0].url, "https://api.example.com/users");
        assert_eq!(result.requests[0].query_params.len(), 3);
        assert_eq!(result.requests[0].query_params[0].key, "page");
        assert_eq!(result.requests[0].query_params[0].is_inline, true);
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
        assert_eq!(result.requests[0].url, "https://api.example.com/users");
        assert_eq!(result.requests[0].query_params.len(), 3);
        assert_eq!(result.requests[0].query_params[0].is_inline, false);
        assert_eq!(result.requests[0].headers.len(), 1);
        assert_eq!(result.requests[0].headers[0].key, "Accept");
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
        // page from URL is inline
        assert_eq!(result.requests[0].query_params[0].enabled, true);
        assert_eq!(result.requests[0].query_params[0].is_inline, true);
        // limit and sort are multiline + disabled
        assert_eq!(result.requests[0].query_params[1].enabled, false);
        assert_eq!(result.requests[0].query_params[1].is_inline, false);
        assert_eq!(result.requests[0].query_params[2].enabled, false);
        assert_eq!(result.requests[0].query_params[2].is_inline, false);
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
        // page from URL, limit/sort from multiline
        assert_eq!(result.requests[0].query_params[0].key, "page");
        assert_eq!(result.requests[0].query_params[0].is_inline, true);
        assert_eq!(result.requests[0].query_params[1].is_inline, false);
        assert_eq!(result.requests[0].query_params[2].is_inline, false);
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
        assert_eq!(result.requests[0].query_params[0].is_inline, true);
    }

    // =======================================================================
    // Header tests
    // =======================================================================

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
        assert_eq!(result.requests[0].headers[2].key, "X-Disabled2");
    }

    // =======================================================================
    // Form URL-encoded tests
    // =======================================================================

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
        assert_eq!(result.requests[0].form_urlencoded[0].is_inline, true);
        assert_eq!(result.requests[0].form_urlencoded[1].key, "password");
        assert_eq!(result.requests[0].form_urlencoded[2].key, "remember");
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
        assert_eq!(result.requests[0].form_urlencoded[0].is_inline, false);
        assert_eq!(result.requests[0].form_urlencoded[0].key, "username");
        assert_eq!(result.requests[0].form_urlencoded[1].key, "password");
    }

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
        assert_eq!(result.requests[0].form_urlencoded[1].enabled, false);
        assert_eq!(result.requests[0].form_urlencoded[2].enabled, true);
    }

    // =======================================================================
    // Multipart tests
    // =======================================================================

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

    // =======================================================================
    // Serializer tests
    // =======================================================================

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

    // =======================================================================
    // Update / append tests
    // =======================================================================

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

    // =======================================================================
    // Body mode detection
    // =======================================================================

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

    // =======================================================================
    // URL query splitting
    // =======================================================================

    #[test]
    fn test_split_url_query() {
        let content = "
### S
GET https://api.example.com/users?page=1&limit=20 HTTP/1.1
";
        let result = parse_http_file(content);
        assert_eq!(result.requests[0].url, "https://api.example.com/users");
        assert_eq!(result.requests[0].query_params.len(), 2);
    }

    #[test]
    fn test_split_url_no_query() {
        let content = "
### S
GET https://api.example.com/users HTTP/1.1
";
        let result = parse_http_file(content);
        assert_eq!(result.requests[0].url, "https://api.example.com/users");
        assert!(result.requests[0].query_params.is_empty());
    }

    // =======================================================================
    // Surgical update tests
    // =======================================================================

    #[test]
    fn test_surgical_update_preserves_comments_in_body() {
        let content = "
### Upload
POST /upload HTTP/1.1
Content-Type: application/json

// This is a comment about the body
{\"file\": \"data\"}
// End comment
";
        let parse_result = parse_http_file(content);
        let mut updated = parse_result.requests[0].clone();
        // Only change the method.
        updated.method = "PUT".to_string();
        let result = apply_request_update(content, 0, &updated);
        // Body comments must be preserved.
        assert!(result.contains("// This is a comment about the body"));
        assert!(result.contains("// End comment"));
        assert!(result.contains("PUT /upload"));
    }

    #[test]
    fn test_surgical_update_preserves_header_comments() {
        let content = "
### Test
GET /api HTTP/1.1
// auth header
Authorization: Bearer token
// custom header
X-Custom: value
";
        let parse_result = parse_http_file(content);
        let mut updated = parse_result.requests[0].clone();
        // Only change URL, headers and their comments should stay.
        updated.url = "/api/v2".to_string();
        let result = apply_request_update(content, 0, &updated);
        assert!(result.contains("// auth header"));
        assert!(result.contains("// custom header"));
        assert!(result.contains("/api/v2"));
    }

    #[test]
    fn test_surgical_update_no_change_returns_original() {
        let content = "
### Test
GET /api HTTP/1.1
X-Custom: value
";
        let parse_result = parse_http_file(content);
        let updated = parse_result.requests[0].clone();
        let result = apply_request_update(content, 0, &updated);
        // Should be byte-identical to original.
        assert_eq!(result, content);
    }

    #[test]
    fn test_surgical_update_body_change_preserves_headers() {
        let content = "
### Test
POST /api HTTP/1.1
Content-Type: application/json

{\"old\": true}
";
        let parse_result = parse_http_file(content);
        let mut updated = parse_result.requests[0].clone();
        updated.body = Some("{\"new\": true}".to_string());
        let result = apply_request_update(content, 0, &updated);
        // Headers must be preserved verbatim.
        assert!(result.contains("Content-Type: application/json"));
        assert!(result.contains("{\"new\": true}"));
        assert!(!result.contains("{\"old\": true}"));
    }

    #[test]
    fn test_surgical_update_query_param_only() {
        let content = "
### Search
GET https://api.example.com/search?q=old HTTP/1.1
Accept: application/json
";
        let parse_result = parse_http_file(content);
        let mut updated = parse_result.requests[0].clone();
        // Change only a query param.
        if let Some(q) = updated.query_params.iter_mut().find(|q| q.key == "q") {
            q.value = "new".to_string();
        }
        let result = apply_request_update(content, 0, &updated);
        // Headers must stay verbatim.
        assert!(result.contains("Accept: application/json"));
        assert!(result.contains("q=new"));
        assert!(!result.contains("q=old"));
    }

    #[test]
    fn test_surgical_update_multiple_requests_only_affects_target() {
        let content = "
### First
GET /first HTTP/1.1
X-One: 1

### Second
GET /second HTTP/1.1
X-Two: 2
";
        let parse_result = parse_http_file(content);
        let mut updated = parse_result.requests[1].clone();
        updated.method = "POST".to_string();
        let result = apply_request_update(content, 1, &updated);
        // First request unchanged.
        assert!(result.contains("### First"));
        assert!(result.contains("GET /first"));
        assert!(result.contains("X-One: 1"));
        // Second request updated.
        assert!(result.contains("POST /second"));
        assert!(result.contains("X-Two: 2"));
    }

    #[test]
    fn test_surgical_update_query_no_value_roundtrip() {
        let content = "
### Flag
GET https://api.example.com/data?debug HTTP/1.1
";
        let parse_result = parse_http_file(content);
        let serialized = serialize_request_block(&parse_result.requests[0]);
        // The no-value param should be preserved without a trailing =.
        assert!(serialized.contains("?debug"));
        assert!(!serialized.contains("?debug="));
        // Roundtrip should parse back correctly.
        let reparsed = parse_http_file(&serialized);
        assert_eq!(reparsed.requests[0].query_params.len(), 1);
        assert_eq!(reparsed.requests[0].query_params[0].key, "debug");
        assert_eq!(reparsed.requests[0].query_params[0].value, "");
    }

    #[test]
    fn test_surgical_update_disable_inline_moves_to_multiline() {
        // Disabling an inline param should move it to multiline and stay there.
        let content = "
### Search
GET https://api.example.com/search?q=test&page=1 HTTP/1.1
";
        let parse_result = parse_http_file(content);
        let mut updated = parse_result.requests[0].clone();
        // Disable "page"
        if let Some(q) = updated.query_params.iter_mut().find(|q| q.key == "page") {
            q.enabled = false;
            q.is_inline = false;
        }
        let result = apply_request_update(content, 0, &updated);
        // q=test should still be inline in URL
        assert!(result.contains("?q=test"));
        // page=1 should now be multiline disabled
        assert!(result.contains("//- &page=1"));
        // Re-enable: should stay multiline (not back to URL)
        if let Some(q) = updated.query_params.iter_mut().find(|q| q.key == "page") {
            q.enabled = true;
            // is_inline stays false
        }
        let result2 = apply_request_update(&result, 0, &updated);
        // page stays multiline (is_inline: false), first enabled multiline param uses '?'
        assert!(result2.contains("?page=1"));
        // q=test is still inline in URL
        assert!(result2.contains("?q=test"));
    }

    #[test]
    fn test_surgical_edit_no_body_keep_blank_line() {
        let content = "
### GET Request
GET https://httpbingo.org/get HTTP/1.1

### GET with Query Parameters
GET https://httpbingo.org/get?foo=bar&baz=qux HTTP/1.1
";
        let parse_result = parse_http_file(content);
        let mut updated = parse_result.requests[0].clone();
        updated.url = "https://httpbingo.org/get1".to_string();
        let result = apply_request_update(content, 0, &updated);
        assert!(result.contains("GET https://httpbingo.org/get1 HTTP/1.1\n\n###"));
    }

    #[test]
    fn test_surgical_edit_url_with_header_no_body_keep_blank_line() {
        let content = "
### GET Request
GET https://httpbingo.org/get HTTP/1.1
Authorization: Bearer key

### GET with Query Parameters
GET https://httpbingo.org/get?foo=bar&baz=qux HTTP/1.1
    ";
        let parse_result = parse_http_file(content);
        let mut updated = parse_result.requests[0].clone();
        updated.url = "https://httpbingo.org/get1".to_string();
        updated.headers[0].value = "Bearer key_updated".to_string();
        let result = apply_request_update(content, 0, &updated);
        assert!(result.contains("Authorization: Bearer key_updated\n\n###"));

        updated.headers.push(HttpHeaderField {
            key: "TheHeader".to_string(),
            value: "TheValue".to_string(),
            enabled: true,
        });
        let result2 = apply_request_update(content, 0, &updated);
        assert!(result2.contains("Authorization: Bearer key_updated\nTheHeader: TheValue\n\n###"));
    }
}
