use std::io::{self, Write};

use serde_json::{Map, Number, Value as JsonValue};

use crate::Record;

use super::Formatter;

/// Writes one compact JSON object per line (NDJSON) and [`Write::flush`]es after each record so
/// consumers like `tail -f` see output immediately (unlike [`super::JsonFormatter`], which may
/// buffer internally until flush).
pub struct JsonLinesFormatter<W: Write> {
    writer: W,
    attach_raw_footer: bool,
}

impl<W: Write> JsonLinesFormatter<W> {
    pub fn new(w: W) -> Self {
        Self {
            writer: w,
            attach_raw_footer: true,
        }
    }

    pub fn without_raw_footer(writer: W) -> Self {
        Self {
            writer,
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

impl<W: Write> Formatter for JsonLinesFormatter<W> {
    fn write(&mut self, r: &Record) -> io::Result<()> {
        serde_json::to_writer(&mut self.writer, &record_json(r, self.attach_raw_footer))?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
