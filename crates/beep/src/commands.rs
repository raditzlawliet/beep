use beep_core::{HttpClient, HttpMethod, HttpRequest};

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

    let mut req = HttpRequest::new(url.to_string(), http_method);

    for header in headers {
        if let Some((key, value)) = header.split_once(':') {
            req = req.with_header(key.trim().to_string(), value.trim().to_string());
        } else {
            eprintln!(
                "Warning: ignoring malformed header '{}' (expected key:value)",
                header
            );
        }
    }

    if let Some(b) = body {
        req = req.with_body(b.to_string());
    }

    let client = HttpClient::new();
    let response = client.execute(&req).await?;

    println!("Status: {} ({}ms)", response.status, response.elapsed_ms);

    if !response.headers.is_empty() {
        println!("\nResponse Headers:");
        for (key, value) in &response.headers {
            println!("  {}: {}", key, value);
        }
    }

    if !response.body.is_empty() {
        println!("\nResponse Body:");
        println!("{}", response.body);
    }

    Ok(())
}
