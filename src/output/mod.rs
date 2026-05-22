//! Output writers for matched records (`json` lines, `count`, table, etc.).
use crate::Record;

pub trait Formatter {
    fn write(&mut self, r: &Record) -> std::io::Result<()>;
    fn flush(&mut self) -> std::io::Result<()>;
}

mod count;
mod json;
pub mod table;

pub use count::CountFormatter;
pub use json::JsonFormatter;
pub use table::TableFormatter;
