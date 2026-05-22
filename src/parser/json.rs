use serde_json::Number;
use serde_json::Value as JsonValue;

use crate::Record;
use crate::RecordError;
use crate::Value;

/// Parse a single JSON object line into a [`Record`] with dot-flattened nested object keys.
///
/// Nested objects are flattened recursively: `{"b":{"c":2}}` becomes field key `b.c`.
/// Arrays are stored as [`Value::Str`] containing compact serialized JSON from `serde_json`.
pub fn parse_json_line(s: &str) -> Result<Record, RecordError> {
    let v: JsonValue = serde_json::from_str(s).map_err(|e| RecordError::Parse(e.to_string()))?;
    let obj = v
        .as_object()
        .ok_or_else(|| RecordError::Parse("expected JSON object".into()))?;
    let mut record = Record::new(s.to_string());
    flatten_object("", obj, &mut record)?;
    Ok(record)
}

fn flatten_object(
    prefix: &str,
    obj: &serde_json::Map<String, JsonValue>,
    record: &mut Record,
) -> Result<(), RecordError> {
    for (key, child) in obj {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };

        match child {
            JsonValue::Object(nested) => flatten_object(&path, nested, record)?,
            JsonValue::Array(_) => record.insert(path, Value::Str(child.to_string())),
            JsonValue::String(inner) => record.insert(path, Value::Str(inner.clone())),
            JsonValue::Bool(b) => record.insert(path, Value::Bool(*b)),
            JsonValue::Null => record.insert(path, Value::Null),
            JsonValue::Number(n) => record.insert(path, json_number(n.clone())?),
        }
    }
    Ok(())
}

fn json_number(n: Number) -> Result<Value, RecordError> {
    if let Some(i) = n.as_i64() {
        return Ok(Value::Int(i));
    }
    if let Some(u) = n.as_u64() {
        if u <= i64::MAX as u64 {
            return Ok(Value::Int(u as i64));
        }
    }
    n.as_f64()
        .map(Value::Float)
        .ok_or_else(|| RecordError::Parse(format!("unsupported JSON number {n:?}")))
}
