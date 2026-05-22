//! Output writers for matched records (`json` lines, `count`, etc.).
use crate::Record;

pub trait Formatter {
    fn write(&mut self, r: &Record) -> std::io::Result<()>;
    fn flush(&mut self) -> std::io::Result<()>;
}

mod color;
mod count;
mod json;

pub use color::ColorWriter;
pub use count::CountFormatter;
pub use json::JsonFormatter;
