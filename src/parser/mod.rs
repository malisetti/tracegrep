//! Structured log parsers (JSON-lines and logfmt).

mod json;

pub mod detect;
pub mod logfmt;
pub mod resilience;

pub use detect::{parse_auto, sniff, Format};
pub use json::parse_json_line;
pub use logfmt::parse_logfmt_line;
