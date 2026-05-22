use std::cmp::Ordering;

use tracegrep::{Record, Value};

#[test]
fn insert_and_get_roundtrip() {
    let mut record = Record::new("{}");
    record.insert("level", Value::Str("error".into()));
    record.insert("code", Value::Int(42));
    assert_eq!(record.get("level"), Some(&Value::Str("error".into())));
    assert_eq!(record.get("code"), Some(&Value::Int(42)));
    assert!(record.get("missing").is_none());
}

#[test]
fn raw_preserved_independent_of_fields() {
    let raw = r#"{"trace_id":"abc","msg":"boom"}"#;
    let mut record = Record::new(raw.to_string());
    record.insert("extra", Value::Bool(true));
    assert_eq!(record.raw(), raw);
}

#[test]
fn ordering_compares_integer_and_float_numerically() {
    assert_eq!(
        Value::Int(1).partial_cmp(&Value::Float(2.0)),
        Some(Ordering::Less)
    );
    assert_eq!(
        Value::Float(2.0).partial_cmp(&Value::Int(1)),
        Some(Ordering::Greater)
    );
    assert_eq!(
        Value::Float(1.25).partial_cmp(&Value::Int(1)),
        Some(Ordering::Greater)
    );
}

#[test]
fn ordering_lexicographic_for_strings() {
    assert_eq!(
        Value::Str("aaa".into()).partial_cmp(&Value::Str("zzz".into())),
        Some(Ordering::Less)
    );
}

#[test]
fn mixed_types_do_not_partial_compare() {
    assert_eq!(Value::Int(1).partial_cmp(&Value::Str("1".into())), None);
}
