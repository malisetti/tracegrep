use std::io::{self, Write};

pub struct DiagnosticsSink<W: Write> {
    writer: W,
    skipped_lines: u64,
    errors: u64,
}

impl<W: Write> DiagnosticsSink<W> {
    pub fn new(w: W) -> Self {
        Self {
            writer: w,
            skipped_lines: 0,
            errors: 0,
        }
    }

    pub fn record_skip(&mut self, line_num: u64, reason: &str) {
        self.skipped_lines += 1;
        let _ = writeln!(
            self.writer,
            "tracegrep: warning: line {}: {}",
            line_num, reason
        );
    }

    pub fn record_error(&mut self, line_num: u64, err: &str) {
        self.errors += 1;
        let _ = writeln!(self.writer, "tracegrep: error: line {}: {}", line_num, err);
    }

    pub fn summary(&mut self) -> io::Result<()> {
        if self.skipped_lines != 0 {
            let line_word = if self.skipped_lines == 1 {
                "line"
            } else {
                "lines"
            };
            writeln!(
                self.writer,
                "tracegrep: {} {} skipped (use --strict to abort on first)",
                self.skipped_lines, line_word
            )?;
        }
        if self.errors != 0 {
            let err_word = if self.errors == 1 { "error" } else { "errors" };
            writeln!(self.writer, "tracegrep: {} {}", self.errors, err_word)?;
        }
        if self.skipped_lines != 0 || self.errors != 0 {
            self.writer.flush()?;
        }
        Ok(())
    }

    pub fn into_inner(self) -> W {
        self.writer
    }
}
