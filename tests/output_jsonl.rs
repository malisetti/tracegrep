use std::cell::Cell;
use std::io::{self, Write};
use std::rc::Rc;

use serde_json::{Number, Value as JsonVal};

use tracegrep::output::{Formatter, JsonLinesFormatter};
use tracegrep::{Record, Value};

#[test]
fn jsonl_writes_one_compact_object_line_per_record() {
    let mut buf = Vec::new();
    {
        let mut f = JsonLinesFormatter::new(&mut buf);
        let mut record = Record::new(r#"{"msg":"hello"}"#.to_owned());
        record.insert("level", Value::Str("error".into()));
        f.write(&record).unwrap();
        f.flush().unwrap();
    }
    let rendered = String::from_utf8(buf).unwrap();
    assert_eq!(
        rendered.lines().count(),
        1,
        "expected exactly one newline-terminated line per record write"
    );
    let line = rendered.lines().next().unwrap();
    let parsed: JsonVal = serde_json::from_str(line).unwrap();
    assert_eq!(parsed.get("level"), Some(&JsonVal::String("error".into())));
    assert_eq!(
        parsed.get("_raw"),
        Some(&JsonVal::String(r#"{"msg":"hello"}"#.into()))
    );
}

#[test]
fn jsonl_serializes_distinct_records_on_separate_lines() {
    let mut buf = Vec::new();
    {
        let mut f = JsonLinesFormatter::new(&mut buf);
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
fn jsonl_flushes_writer_after_every_record_write() {
    let flush_calls = Rc::new(Cell::new(0usize));
    let mut w = FlushCountWriter {
        inner: Vec::new(),
        flush_calls: Rc::clone(&flush_calls),
    };
    {
        let mut f = JsonLinesFormatter::new(&mut w);
        let mut a = Record::new("{}".to_owned());
        a.insert("k", Value::Int(1));
        f.write(&a).unwrap();
        assert_eq!(flush_calls.get(), 1, "flush after first write");

        let mut b = Record::new("{}".to_owned());
        b.insert("k", Value::Int(2));
        f.write(&b).unwrap();
        assert_eq!(
            flush_calls.get(),
            2,
            "flush after second write for streaming consumers"
        );
        f.flush().unwrap();
        assert_eq!(
            flush_calls.get(),
            3,
            "explicit flush forwarded to underlying writer"
        );
    }

    let text = String::from_utf8(w.inner).unwrap();
    let mut iter = text.lines();
    serde_json::from_str::<JsonVal>(iter.next().unwrap())
        .unwrap()
        .get("k")
        .unwrap();
    serde_json::from_str::<JsonVal>(iter.next().unwrap())
        .unwrap()
        .get("k")
        .unwrap();
    assert!(iter.next().is_none());
}

#[test]
fn jsonl_field_order_follows_records_btreemap_iteration() {
    let mut buf = Vec::new();
    {
        let mut f = JsonLinesFormatter::new(&mut buf);
        let mut record = Record::new("line");
        record.insert("zzz", Value::Str("third".into()));
        record.insert("aaa", Value::Str("first".into()));
        record.insert("mmm", Value::Str("middle".into()));
        f.write(&record).unwrap();
    }
    let line = String::from_utf8(buf).unwrap();
    let line = line.lines().next().unwrap();
    assert!(line.contains("\"_raw\""));

    let aaa = line.find("\"aaa\"").expect("missing aaa key segment");
    let mmm = line.find("\"mmm\"").expect("missing mmm key segment");
    let zzz = line.find("\"zzz\"").expect("missing zzz key segment");
    assert!(
        aaa < mmm && mmm < zzz,
        "structured fields emitted in Record BTree iteration order ({line})"
    );
}

struct FlushCountWriter {
    inner: Vec<u8>,
    flush_calls: Rc<Cell<usize>>,
}

impl Write for FlushCountWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let n = self.flush_calls.get();
        self.flush_calls.set(n + 1);
        Ok(())
    }
}
