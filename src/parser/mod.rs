//! Structured log parsers (JSON-lines and logfmt).

mod json;

pub mod logfmt;

pub use json::parse_json_line;
pub use logfmt::parse_logfmt_line;
