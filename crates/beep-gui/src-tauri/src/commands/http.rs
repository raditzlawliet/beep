//! Tauri commands for handling HTTP file parsing, serialization, and mutations.
use beep_core::{FileVariable, ParseHttpFileResult, ParsedRequest, http_parser};

#[tauri::command]
pub fn http_parse(content: String) -> ParseHttpFileResult {
    http_parser::parse_http_file(&content)
}

#[tauri::command]
pub fn http_serialize_req(req: ParsedRequest) -> String {
    http_parser::serialize_request_block(&req)
}

#[tauri::command]
pub fn http_serialize_vars(variables: Vec<FileVariable>) -> String {
    http_parser::serialize_file_variables(&variables)
}

#[tauri::command]
pub fn http_update_vars(content: String, variables: Vec<FileVariable>) -> String {
    http_parser::apply_variable_update(&content, &variables)
}

#[tauri::command]
pub fn http_update_req(content: String, request_idx: usize, updated: ParsedRequest) -> String {
    http_parser::apply_request_update(&content, request_idx, &updated)
}

#[tauri::command]
pub fn http_append_req(content: String, new_request: ParsedRequest) -> String {
    http_parser::append_request_block(&content, &new_request)
}
