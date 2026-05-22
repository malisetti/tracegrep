//! ANSI coloring helpers with TTY-aware defaults.
use std::io::{self, IsTerminal, Write};

/// Wraps a [`Write`] target and tracks whether ANSI styling should be applied.
pub struct ColorWriter<W: Write> {
    inner: W,
    enabled: bool,
}

impl<W: Write> ColorWriter<W> {
    /// Creates a writer that enables colors when process stdout is a terminal.
    pub fn new_auto(inner: W) -> Self {
        let enabled = io::stdout().is_terminal();
        Self { inner, enabled }
    }

    /// Creates a writer with an explicit color policy (for example: tests).
    pub fn with_enabled(inner: W, enabled: bool) -> Self {
        Self { inner, enabled }
    }

    /// Wraps a log level with ANSI colors when `enabled` is true.
    ///
    /// Mapping: `error` → red, `warn`/`warning` → yellow, `info` → cyan, `debug` → dim.
    /// Unknown levels are returned unchanged.
    pub fn colorize_level(&self, level: &str) -> String {
        if !self.enabled {
            return level.to_owned();
        }
        let lower = level.to_ascii_lowercase();
        let (open, close) = match lower.as_str() {
            "error" => ("\x1b[31m", "\x1b[0m"),
            "warn" | "warning" => ("\x1b[33m", "\x1b[0m"),
            "info" => ("\x1b[36m", "\x1b[0m"),
            "debug" => ("\x1b[2m", "\x1b[0m"),
            _ => return level.to_owned(),
        };
        format!("{open}{level}{close}")
    }

    pub fn inner_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    pub fn into_inner(self) -> W {
        self.inner
    }
}

impl<W: Write> Write for ColorWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
