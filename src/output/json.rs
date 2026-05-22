use std::io::Write;

use serde_json::{Map, Number, Value as JsonValue};

use crate::Record;

use super::Formatter;

pub struct JsonFormatter<W: Write> {
    inner: W,
    attach_raw_footer: bool,
}

impl<W: Write> JsonFormatter<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            attach_raw_footer: true,
        }
    }

    /// Field projection emits only listed keys — skip the implicit `_raw` shim to avoid duplication.
    pub fn without_raw_footer(inner: W) -> Self {
        Self {
            inner,
            attach_raw_footer: false,
        }
    }
}

fn scalar_to_json(value: &crate::Value) -> JsonValue {
    use crate::Value;
    match value {
        Value::Str(s) => JsonValue::String(s.clone()),
        Value::Int(i) => JsonValue::Number(Number::from(*i)),
        Value::Float(f) => Number::from_f64(*f)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        Value::Bool(v) => JsonValue::Bool(*v),
        Value::Null => JsonValue::Null,
    }
}

fn record_json(r: &Record, attach_raw_footer: bool) -> JsonValue {
    let mut map = Map::new();
    for field in r.iter() {
        map.insert(field.name.to_string(), scalar_to_json(field.value));
    }
    if attach_raw_footer {
        map.insert("_raw".to_owned(), JsonValue::String(r.raw().to_string()));
    }
    JsonValue::Object(map)
}

impl<W: Write> Formatter for JsonFormatter<W> {
    fn write(&mut self, r: &Record) -> std::io::Result<()> {
        let value = record_json(r, self.attach_raw_footer);
        serde_json::to_writer(&mut self.inner, &value)?;
        self.inner.write_all(b"\n")?;
        Ok(())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
