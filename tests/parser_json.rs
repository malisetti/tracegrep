use tracegrep::parser::parse_json_line;
use tracegrep::{RecordError, Value};

#[test]
fn flattens_one_level_nested_object_with_dot_keys() {
    let r = parse_json_line(r#"{"a":1,"b":{"c":2}}"#).expect("parse");
    assert_eq!(r.get("a"), Some(&Value::Int(1)));
    assert_eq!(r.get("b.c"), Some(&Value::Int(2)));
    assert_eq!(r.raw(), r#"{"a":1,"b":{"c":2}}"#);
}

#[test]
fn flattens_deeply_nested_objects() {
    let r = parse_json_line(r#"{"x":{"y":{"z":true,"n":null}}}"#).expect("parse");
    assert_eq!(r.get("x.y.z"), Some(&Value::Bool(true)));
    assert_eq!(r.get("x.y.n"), Some(&Value::Null));
}

#[test]
fn preserves_scalar_types() {
    let raw = r#"{"s":"hi","i":42,"f":3.14,"t":true,"f2":false,"n":null}"#;
    let r = parse_json_line(raw).expect("parse");
    assert_eq!(r.get("s"), Some(&Value::Str("hi".into())));
    assert_eq!(r.get("i"), Some(&Value::Int(42)));
    match r.get("f") {
        Some(Value::Float(x)) => assert!((x - 3.14).abs() < 1e-9),
        o => panic!("expected float, got {:?}", o),
    }
    assert_eq!(r.get("t"), Some(&Value::Bool(true)));
    assert_eq!(r.get("f2"), Some(&Value::Bool(false)));
    assert_eq!(r.get("n"), Some(&Value::Null));
}

#[test]
fn arrays_are_json_strings() {
    let raw = r#"{"items":[1,2,"three"],"nested":[{"a":1}]}"#;
    let r = parse_json_line(raw).expect("parse");
    match r.get("items") {
        Some(Value::Str(s)) => assert_eq!(s, r#"[1,2,"three"]"#),
        o => panic!("expected Str for array, got {:?}", o),
    }
    match r.get("nested") {
        Some(Value::Str(s)) => assert_eq!(s, r#"[{"a":1}]"#),
        o => panic!("expected Str for nested array, got {:?}", o),
    }
}

#[test]
fn malformed_returns_err() {
    assert!(matches!(
        parse_json_line("not json").unwrap_err(),
        RecordError::Parse(_)
    ));
    assert!(matches!(
        parse_json_line("").unwrap_err(),
        RecordError::Parse(_)
    ));
}

#[test]
fn root_must_be_object() {
    assert!(matches!(
        parse_json_line(r#"[1,2]"#).unwrap_err(),
        RecordError::Parse(_)
    ));
}
