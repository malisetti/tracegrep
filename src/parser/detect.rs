use super::{parse_json_line, parse_logfmt_line};
use crate::{Record, RecordError, Value};

/// Detected principal line-oriented format used by sniff/parse-auto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    JsonLines,
    Logfmt,
    Plain,
}

/// Decide format from the **first non-empty line** of `input` (blank lines skipped).
///
/// Heuristic order for that line:
///
/// - Trim-start begins with `{` → [`Format::JsonLines`].
/// - Trim-start begins with `word=value` (`\[A-Za-z0-9_]+=`) **and**
///   the trimmed line contains `=` → [`Format::Logfmt`].
/// - Otherwise → [`Format::Plain`] (see [`parse_auto`]).
///
/// Empty or whitespace-only inputs yield [`Format::Plain`].
pub fn sniff(input: &str) -> Format {
    for raw in input.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let t = line.trim_start();

        if t.starts_with('{') {
            return Format::JsonLines;
        }

        if line.contains('=') && has_word_key_equals_prefix(t) {
            return Format::Logfmt;
        }

        return Format::Plain;
    }
    Format::Plain
}

/// Parse `line` using `fmt` (`sniff`-compatible split between JSON, logfmt, and plain fallback).
pub fn parse_auto(line: &str, fmt: Format) -> Result<Record, RecordError> {
    match fmt {
        Format::JsonLines => parse_json_line(line),
        Format::Logfmt => parse_logfmt_line(line),
        Format::Plain => {
            let mut record = Record::new(line.to_string());
            record.insert("msg", Value::Str(line.to_string()));
            Ok(record)
        }
    }
}

fn has_word_key_equals_prefix(s: &str) -> bool {
    let b = s.as_bytes();
    let Some(&first) = b.first() else {
        return false;
    };
    if !is_word_byte(first) {
        return false;
    }

    let mut i = 0usize;
    while i < b.len() && is_word_byte(b[i]) {
        i += 1;
    }

    i < b.len() && b[i] == b'='
}

fn is_word_byte(c: u8) -> bool {
    matches!(c, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_')
}
