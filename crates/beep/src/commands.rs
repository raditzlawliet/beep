use std::sync::{Arc, Mutex};

use beep_core::{
    ExecuteInput, ExecutionContext, HttpMethod, ParsedHeaderField, ParsedRequest, RequestHistory,
};

pub async fn request(
    url: &str,
    method: &str,
    headers: &[String],
    body: Option<&str>,
) -> Result<(), String> {
    let http_method = match method.to_uppercase().as_str() {
        "GET" => HttpMethod::Get,
        "POST" => HttpMethod::Post,
        "PUT" => HttpMethod::Put,
        "DELETE" => HttpMethod::Delete,
        "PATCH" => HttpMethod::Patch,
        "HEAD" => HttpMethod::Head,
        "OPTIONS" => HttpMethod::Options,
        _ => return Err(format!("Unknown HTTP method: {}", method)),
    };

    let mut parsed_headers: Vec<ParsedHeaderField> = Vec::new();
    for header in headers {
        if let Some((key, value)) = header.split_once(':') {
            parsed_headers.push(ParsedHeaderField {
                key: key.trim().to_string(),
                value: value.trim().to_string(),
                enabled: true,
                auto: false,
            });
        } else {
            eprintln!(
                "Warning: ignoring malformed header '{}' (expected key:value)",
                header
            );
        }
    }

    let parsed = ParsedRequest {
        method: http_method.to_string(),
        url: url.to_string(),
        headers: parsed_headers,
        body: body.map(|b| b.to_string()),
        ..Default::default()
    };

    let history = Arc::new(Mutex::new(RequestHistory::new()));
    let mut ctx = ExecutionContext::new(history);

    let result = beep_core::execute(ExecuteInput::Parsed(parsed), &mut ctx).await?;

    println!(
        "Status: {} ({}ms)",
        result.response.status, result.response.elapsed_ms
    );

    if !result.response.headers.is_empty() {
        println!("\nResponse Headers:");
        for (key, value) in &result.response.headers {
            println!("  {}: {}", key, value);
        }
    }

    if !result.response.body.is_empty() {
        println!("\nResponse Body:");
        println!("{}", result.response.body);
    }

    Ok(())
}
