use tracegrep::{parse_auto, sniff, Format, RecordError, Value};

#[test]
fn sniff_plain_empty_skips_then_json() {
    let fmt = sniff("\n  \r\n{\"lvl\":\"warn\",\"msg\":\"x\"}\nafterignored");
    assert_eq!(fmt, Format::JsonLines);
}

#[test]
fn sniff_logfmt_need_word_key_prefix_at_trim_start_and_equals_in_line() {
    assert_eq!(sniff("  level=error msg=dial tcp"), Format::Logfmt);
    assert_eq!(sniff("plain text tail key=value"), Format::Plain);
}

#[test]
fn sniff_plain_when_only_equals_without_key() {
    assert_eq!(sniff("=nokey"), Format::Plain);
}

#[test]
fn parse_auto_plain_wraps_entire_line_as_msg() {
    let r = parse_auto("hello world", Format::Plain).unwrap();
    assert_eq!(r.get("msg").unwrap(), &Value::Str("hello world".into()));
    assert_eq!(r.raw(), "hello world");
}

#[test]
fn parse_auto_json_round_trips_via_sniff_heuristic_branch() {
    let line = r#"{"a":{"b":1}}"#;
    assert_eq!(sniff(line), Format::JsonLines);
    let r = parse_auto(line, Format::JsonLines).unwrap();
    assert_eq!(r.get("a.b").unwrap(), &Value::Int(1));
}

#[test]
fn parse_auto_logfmt_respects_logfmt_semantics() {
    let line = "level=info msg=started";
    assert_eq!(sniff(line), Format::Logfmt);
    let r = parse_auto(line, Format::Logfmt).unwrap();
    assert_eq!(r.get("level").unwrap(), &Value::Str("info".into()));
}

#[test]
fn parse_auto_json_errors_on_plain_line_when_forced_json() {
    let err = parse_auto("not-json", Format::JsonLines).unwrap_err();
    assert!(matches!(err, RecordError::Parse(_)));
}
