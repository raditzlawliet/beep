#[allow(dead_code)]
#[derive(serde::Serialize)]
pub struct AppConstants {
    pub version: String,
    pub platform: String,
    pub default_headers: Vec<(String, String)>,
}
