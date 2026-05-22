//! Integration tests for logfmt parsing (quoted + bare, escapes, inference, errors).

use tracegrep::{parse_logfmt_line, RecordError, Value};

#[test]
fn parses_quoted_and_unquoted_fields_full_sample() {
    let line = r#"ts=2026-05-22T10:00:00Z level=error msg="a b c" lat=15.2 ok=true"#;
    let r = parse_logfmt_line(line).expect("parses");
    assert_eq!(r.raw(), line);
    assert_eq!(
        r.get("ts"),
        Some(&Value::Str("2026-05-22T10:00:00Z".into()))
    );
    assert_eq!(r.get("level"), Some(&Value::Str("error".into())));
    assert_eq!(r.get("msg"), Some(&Value::Str("a b c".into())));
    assert_eq!(r.get("lat"), Some(&Value::Float(15.2)));
    assert_eq!(r.get("ok"), Some(&Value::Bool(true)));
}

#[test]
fn quoted_double_quote_escape() {
    let line = r#"msg="escaped \"quotes\" inside""#;
    let r = parse_logfmt_line(line).expect("parses");
    assert_eq!(
        r.get("msg"),
        Some(&Value::Str(r#"escaped "quotes" inside"#.into()))
    );
}

#[test]
fn quoted_backslash_escape() {
    let line = r#"p="path\\to\\file""#;
    let r = parse_logfmt_line(line).expect("parses");
    assert_eq!(r.get("p"), Some(&Value::Str(r"path\to\file".into())));
}

#[test]
fn bare_type_inference_int_float_bool_string() {
    let line = "n=42 pi=3.14 flag=false label=not-a-number";
    let r = parse_logfmt_line(line).unwrap();
    assert_eq!(r.get("n"), Some(&Value::Int(42)));
    let parsed_pi: f64 = "3.14".parse().expect("literal");
    match r.get("pi") {
        Some(Value::Float(x)) => assert!((*x - parsed_pi).abs() < 1e-9),
        o => panic!("expected float pi, got {:?}", o),
    }
    assert_eq!(r.get("flag"), Some(&Value::Bool(false)));
    assert_eq!(r.get("label"), Some(&Value::Str("not-a-number".into())));
}

#[test]
fn rejects_unclosed_quote() {
    let err = parse_logfmt_line(r#"x="oops"#).unwrap_err();
    assert!(matches!(err, RecordError::Parse(_)));
    let msg = format!("{}", err);
    assert!(
        msg.contains("unclosed") || msg.contains("quoted"),
        "unexpected message: {}",
        msg
    );
}

#[test]
fn rejects_empty_field_name() {
    let err = parse_logfmt_line(r#"=v"#).unwrap_err();
    assert!(matches!(err, RecordError::Parse(_)));
}

#[test]
fn rejects_missing_equals() {
    let err = parse_logfmt_line("notlogfmt").unwrap_err();
    let msg = format!("{}", err);
    assert!(
        msg.contains('=') || msg.contains("field"),
        "unexpected message: {}",
        msg
    );
}

#[test]
fn rejects_empty_bare_value() {
    let err = parse_logfmt_line("key=").unwrap_err();
    assert!(matches!(err, RecordError::Parse(_)));
}

#[test]
fn rejects_unknown_escape_in_quotes() {
    let err = parse_logfmt_line(r#"z="\z""#).unwrap_err();
    assert!(matches!(err, RecordError::Parse(_)));
}
