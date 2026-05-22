use serde_json::{Number, Value as JsonVal};

use tracegrep::output::{Formatter, JsonFormatter};
use tracegrep::{Record, Value};

#[test]
fn json_formatter_emits_one_compact_object_line_per_record_with_raw_field() {
    let mut buf = Vec::new();
    {
        let mut f = JsonFormatter::new(&mut buf);
        let mut record = Record::new(r#"{"msg":"hello"}"#.to_owned());
        record.insert("level", Value::Str("error".into()));
        f.write(&record).unwrap();
        f.flush().unwrap();
    }
    let rendered = String::from_utf8(buf).unwrap();
    let line = rendered.lines().next().unwrap();
    assert!(
        line.contains("\"level\""),
        "expected emitted JSON to quote field keys: {}",
        line
    );
    let parsed: JsonVal = serde_json::from_str(line).unwrap();
    assert_eq!(parsed.get("level"), Some(&JsonVal::String("error".into())));
    assert_eq!(
        parsed.get("_raw"),
        Some(&JsonVal::String(r#"{"msg":"hello"}"#.into()))
    );
}

#[test]
fn json_formatter_serializes_distinct_records_on_separate_lines() {
    let mut buf = Vec::new();
    {
        let mut f = JsonFormatter::new(&mut buf);
        let mut first = Record::new("{}".to_owned());
        first.insert("k", Value::Int(1));
        f.write(&first).unwrap();
        let mut second = Record::new("{}".to_owned());
        second.insert("k", Value::Int(2));
        f.write(&second).unwrap();
        f.flush().unwrap();
    }
    let text = String::from_utf8(buf).unwrap();
    let mut iter = text.lines();
    assert_eq!(
        serde_json::from_str::<JsonVal>(iter.next().unwrap())
            .unwrap()
            .get("k"),
        Some(&JsonVal::Number(Number::from(1)))
    );
    assert_eq!(
        serde_json::from_str::<JsonVal>(iter.next().unwrap())
            .unwrap()
            .get("k"),
        Some(&JsonVal::Number(Number::from(2)))
    );
    assert!(iter.next().is_none());
}

#[test]
fn json_formatter_maps_numeric_bool_and_null_scalars() {
    let mut buf = Vec::new();
    {
        let mut f = JsonFormatter::new(&mut buf);
        let mut record = Record::new(String::from("raw-ref"));
        record.insert("pi", Value::Float(std::f64::consts::PI));
        record.insert("ok", Value::Bool(false));
        record.insert("n", Value::Null);
        record.insert("tiny", Value::Int(-7));
        f.write(&record).unwrap();
        f.flush().unwrap();
    }
    let line = String::from_utf8(buf)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_owned();
    let parsed: JsonVal = serde_json::from_str(&line).unwrap();
    let pi_number = serde_json::Number::from_f64(std::f64::consts::PI).unwrap();
    assert_eq!(parsed.get("pi"), Some(&JsonVal::Number(pi_number)));
    assert_eq!(parsed.get("ok"), Some(&JsonVal::Bool(false)));
    assert_eq!(parsed.get("n"), Some(&JsonVal::Null));
    assert_eq!(parsed.get("tiny"), Some(&JsonVal::Number(Number::from(-7))));
}
