//! Skip-blank line parsing wrappers and per-line `--input=auto` style detection.

use crate::parser::detect::{parse_auto as parse_auto_with_format, Format};
use crate::{Record, RecordError};

fn is_blank_line(line: &str) -> bool {
    line.trim().is_empty()
}

/// Skip blank lines: [`None`]. Otherwise [`Some`] with [`Ok`] / [`Err`] from JSON parsing.
pub fn parse_json_line_skip(line: &str) -> Option<Result<crate::Record, crate::RecordError>> {
    if is_blank_line(line) {
        return None;
    }
    Some(super::parse_json_line(line.trim()))
}

/// Skip blank lines; otherwise delegate to [`crate::parser::parse_logfmt_line`].
pub fn parse_logfmt_line_skip(line: &str) -> Option<Result<Record, RecordError>> {
    if is_blank_line(line) {
        return None;
    }
    Some(super::parse_logfmt_line(line.trim()))
}

/// Sniff format from this single trimmed non-empty line (same heuristics as file [`crate::parser::detect::sniff`] uses for its first substantive line).
fn sniff_this_line(trimmed_nonempty: &str) -> Format {
    let t = trimmed_nonempty.trim_start();
    if t.starts_with('{') {
        return Format::JsonLines;
    }
    if trimmed_nonempty.contains('=') && has_word_key_equals_prefix(t) {
        return Format::Logfmt;
    }
    Format::Plain
}

/// Blank line → [`None`]. Otherwise sniff format **on this line** and parse (`plain` inserts `msg`).
pub fn parse_auto_perline(line: &str) -> Option<Result<Record, RecordError>> {
    if is_blank_line(line) {
        return None;
    }
    let trimmed = line.trim();
    let fmt = sniff_this_line(trimmed);
    Some(parse_auto_with_format(trimmed, fmt))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;

    #[test]
    fn json_skip_blank_is_none() {
        assert!(parse_json_line_skip("").is_none());
        assert!(parse_json_line_skip("   \t  ").is_none());
        assert!(parse_json_line_skip("\n").is_none());
    }

    #[test]
    fn json_skip_valid_returns_ok() {
        let r = parse_json_line_skip(r#"{"a":1}"#).expect("some");
        let rec = r.expect("ok");
        assert_eq!(rec.get("a"), Some(&Value::Int(1)));
    }

    #[test]
    fn json_skip_malformed_returns_err() {
        let r = parse_json_line_skip("{not-json").expect("some");
        assert!(r.is_err());
    }

    #[test]
    fn logfmt_skip_blank_is_none() {
        assert!(parse_logfmt_line_skip("").is_none());
        assert!(parse_logfmt_line_skip("  ").is_none());
    }

    #[test]
    fn logfmt_skip_valid_returns_ok() {
        let r = parse_logfmt_line_skip("k=v").expect("some");
        let rec = r.expect("ok");
        assert_eq!(rec.get("k"), Some(&Value::Str("v".into())));
    }

    #[test]
    fn logfmt_skip_malformed_returns_err() {
        let r = parse_logfmt_line_skip("noequals").expect("some");
        assert!(r.is_err());
    }

    #[test]
    fn auto_perline_blank_is_none() {
        assert!(parse_auto_perline("").is_none());
        assert!(parse_auto_perline("   ").is_none());
    }

    #[test]
    fn auto_perline_swaps_json_then_logfmt() {
        let j = parse_auto_perline(r#"{"x":42}"#).expect("some");
        let jr = j.expect("json ok");
        assert_eq!(jr.get("x"), Some(&Value::Int(42)));

        let lf = parse_auto_perline("level=info msg=\"hi\"").expect("some");
        let lr = lf.expect("logfmt ok");
        assert_eq!(lr.get("level"), Some(&Value::Str("info".into())));
        assert_eq!(lr.get("msg"), Some(&Value::Str("hi".into())));
    }

    #[test]
    fn auto_perline_plain_even_with_equals_inside_json_shape_priority() {
        // JSON heuristic wins before logfmt '=' rule
        let p = parse_auto_perline(r#"{"q":"a=b"}"#).expect("some");
        let rec = p.expect("ok");
        assert_eq!(rec.get("q"), Some(&Value::Str("a=b".into())));
    }
}
