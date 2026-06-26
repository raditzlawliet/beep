pub mod edit;
pub mod parse;
pub mod serialize;
pub mod types;

// Re-export
pub use edit::{append_request_block, apply_request_update, apply_variable_update};
pub use parse::{detect_body_mode, parse_http_file, strip_disable_marker};
pub use serialize::{serialize_file_variables, serialize_request_block};
pub use types::*;

#[cfg(test)]
mod tests;
