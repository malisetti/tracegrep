use std::io::Write;

use crate::Record;

use super::Formatter;

pub struct CountFormatter<W: Write> {
    inner: W,
    count: u64,
}

impl<W: Write> CountFormatter<W> {
    pub fn new(inner: W) -> Self {
        Self { inner, count: 0 }
    }
}

impl<W: Write> Formatter for CountFormatter<W> {
    fn write(&mut self, _: &Record) -> std::io::Result<()> {
        self.count = self.count.saturating_add(1);
        Ok(())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        writeln!(self.inner, "{}", self.count)?;
        self.inner.flush()
    }
}
