//! Parse a single [logfmt](https://brandur.org/logfmt) line into a [`Record`](crate::Record).

use crate::{Record, RecordError, Value};

/// Parse one logfmt line into a [`Record`].
///
/// - Pairs are `key=value` separated by ASCII whitespace.
/// - Double-quoted values may contain spaces; inside quotes, `\"` and `\\` are recognized.
/// - Bare (unquoted) values are typed as `bool` (`true` / `false`), `i64`, `f64`, or `String`.
/// - The original line is stored as [`Record::raw`].
pub fn parse_logfmt_line(s: &str) -> Result<Record, RecordError> {
    let mut record = Record::new(s.to_string());
    let mut i = skip_ws(s, 0);
    if i >= s.len() {
        return Ok(record);
    }

    while i < s.len() {
        let eq = find_eq(s, i).ok_or_else(|| {
            RecordError::Parse(format!("expected '=' in field starting at byte {i}"))
        })?;
        let key = s
            .get(i..eq)
            .ok_or_else(|| RecordError::Parse("invalid key slice".into()))?;
        let key = key.trim();
        if key.is_empty() {
            return Err(RecordError::Parse("empty field name".into()));
        }

        i = eq + 1;
        if i > s.len() {
            return Err(RecordError::Parse("missing value after '='".into()));
        }

        let (value, next) = if s.as_bytes().get(i) == Some(&b'"') {
            parse_double_quoted(s, i)?
        } else {
            parse_bare_value(s, i)?
        };
        record.insert(key.to_string(), value);
        i = skip_ws(s, next);
    }

    Ok(record)
}

fn skip_ws(s: &str, mut i: usize) -> usize {
    let b = s.as_bytes();
    while i < b.len() && b[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

fn find_eq(s: &str, start: usize) -> Option<usize> {
    s[start..].find('=').map(|off| start + off)
}

fn parse_bare_value(s: &str, start: usize) -> Result<(Value, usize), RecordError> {
    let b = s.as_bytes();
    let mut j = start;
    while j < b.len() && !b[j].is_ascii_whitespace() {
        j += 1;
    }
    let token = s
        .get(start..j)
        .ok_or_else(|| RecordError::Parse("invalid bare value slice".into()))?;
    if token.is_empty() {
        return Err(RecordError::Parse("empty bare value".into()));
    }
    Ok((infer_bare_value(token), j))
}

fn infer_bare_value(token: &str) -> Value {
    match token {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        t => {
            if let Ok(i) = t.parse::<i64>() {
                return Value::Int(i);
            }
            if let Ok(f) = t.parse::<f64>() {
                return Value::Float(f);
            }
            Value::Str(t.to_string())
        }
    }
}

fn parse_double_quoted(s: &str, open_idx: usize) -> Result<(Value, usize), RecordError> {
    if s.as_bytes().get(open_idx) != Some(&b'"') {
        return Err(RecordError::Parse(
            "expected opening '\"' for quoted value".into(),
        ));
    }
    let mut out = String::new();
    let mut it = s[open_idx + 1..].char_indices();
    while let Some((rel, ch)) = it.next() {
        let abs = open_idx + 1 + rel;
        match ch {
            '"' => return Ok((Value::Str(out), abs + '"'.len_utf8())),
            '\\' => match it.next() {
                Some((_, '"')) => out.push('"'),
                Some((_, '\\')) => out.push('\\'),
                Some((_, c)) => {
                    return Err(RecordError::Parse(format!(
                        "unsupported escape sequence in quoted value: \\{c}"
                    )));
                }
                None => {
                    return Err(RecordError::Parse(
                        "dangling backslash in quoted value".into(),
                    ));
                }
            },
            c => out.push(c),
        }
    }
    Err(RecordError::Parse("unclosed quoted value".into()))
}
