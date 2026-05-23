//! `src/group.rs` is included here for Wave‑1 additive tests without `lib.rs` wiring.

pub use tracegrep::{Record, Value};

#[path = "../src/group.rs"]
mod group;

#[test]
fn group_counts_by_level() {
    let mut error1 = Record::new("r1");
    error1.insert("level", Value::Str("error".into()));
    let mut warn = Record::new("r2");
    warn.insert("level", Value::Str("warn".into()));
    let mut info = Record::new("r3");
    info.insert("level", Value::Str("info".into()));
    let mut error2 = Record::new("r4");
    error2.insert("level", Value::Str("error".into()));

    let counts = group::group_count(vec![error1, warn, info, error2], "level");
    assert_eq!(counts.get("error").copied(), Some(2));
    assert_eq!(counts.get("warn").copied(), Some(1));
    assert_eq!(counts.get("info").copied(), Some(1));
    assert_eq!(counts.len(), 3);
}

#[test]
fn group_buckets_missing_and_null_under_empty_key() {
    let mut labeled = Record::new("r1");
    labeled.insert("level", Value::Str("info".into()));
    let missing = Record::new("r2");
    let mut explicit_null = Record::new("r3");
    explicit_null.insert("level", Value::Null);

    let counts = group::group_count([labeled, missing, explicit_null], "level");
    assert_eq!(counts.get("").copied(), Some(2));
    assert_eq!(counts.get("info").copied(), Some(1));
}

#[test]
fn group_single_record() {
    let mut r = Record::new("only");
    r.insert("k", Value::Str("v".into()));

    let counts = group::group_count(std::iter::once(r), "k");
    assert_eq!(counts.len(), 1);
    assert_eq!(counts.get("v").copied(), Some(1));
}
