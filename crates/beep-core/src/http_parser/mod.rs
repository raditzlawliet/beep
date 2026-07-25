pub mod edit;
pub mod parser;
pub mod serializer;
pub mod types;

// Re-export
pub use edit::{append_request_block, apply_request_update, apply_variable_update};
pub use parser::{detect_body_mode, parse, strip_disable_marker};
pub use serializer::{serialize_file_variables, serialize_request_block};
pub use types::*;

#[cfg(test)]
mod tests;
