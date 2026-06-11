mod commands;

use clap::Parser;

#[derive(Parser)]
#[command(name = "beep")]
#[command(about = "Intuitive API Client", long_about = None)]
#[command(override_usage = "beep [OPTIONS] [METHOD] <URL>")]
#[command(version = "0.1.0")]
struct Args {
    /// Request headers (key:value)
    #[arg(short = 'H', long)]
    header: Vec<String>,

    /// Request body
    #[arg(short = 'b', long)]
    body: Option<String>,

    /// HTTP method (e.g. GET, POST, PUT, DELETE, PATCH) - if omitted, GET is used
    /// and the next argument is treated as the URL
    #[arg(value_name = "METHOD")]
    arg1: Option<String>,

    /// URL of the request
    #[arg(value_name = "URL")]
    arg2: Option<String>,
}

fn resolve_method_and_url(
    arg1: Option<&str>,
    arg2: Option<&str>,
) -> Result<(String, String), String> {
    match (arg1, arg2) {
        // Two positional args: first is METHOD, second is URL
        (Some(method), Some(url)) => Ok((method.to_uppercase(), url.to_string())),
        // One positional arg: URL only, method defaults to GET
        (Some(url), None) => Ok(("GET".to_string(), url.to_string())),
        // No URL provided
        (None, _) => Err("No URL provided".to_string()),
    }
}

fn main() {
    let args = Args::parse();

    let (method, url) = match resolve_method_and_url(args.arg1.as_deref(), args.arg2.as_deref()) {
        Ok(result) => result,
        Err(msg) => {
            eprintln!("Error: {}\nUsage: beep [METHOD] <URL> [OPTIONS]", msg);
            std::process::exit(1);
        }
    };

    if let Err(e) = commands::request(&url, &method, &args.header, args.body.as_deref()) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
