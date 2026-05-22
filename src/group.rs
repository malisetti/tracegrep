use crate::Record;
use crate::Value;
use std::collections::BTreeMap;

/// Bucket each [`Record`] by the string representation of `field`.
///
/// The group key for a missing field is `""`. [`Value::Null`] also buckets as `""`.
/// Other variants use a stable textual form suitable for grouping (strings as-is,
/// integers and booleans via `to_string`, floats formatted without spurious decimals when exact).
pub fn group_count<I: IntoIterator<Item = Record>>(it: I, field: &str) -> BTreeMap<String, u64> {
    let mut out: BTreeMap<String, u64> = BTreeMap::new();
    for record in it {
        let key = match record.get(field) {
            None => String::new(),
            Some(v) => value_to_group_key(v),
        };
        *out.entry(key).or_insert(0) += 1;
    }
    out
}

fn value_to_group_key(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Str(s) => s.clone(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => format_float_bucket(*f),
        Value::Bool(b) => b.to_string(),
    }
}

fn format_float_bucket(f: f64) -> String {
    if f.is_nan() {
        return "NaN".to_string();
    }
    if f == f64::INFINITY {
        return "inf".to_string();
    }
    if f == f64::NEG_INFINITY {
        return "-inf".to_string();
    }
    // Preserve integer-like floats without trailing ".0" when exact.
    if f.fract() == 0.0 && f.abs() <= (i64::MAX as f64) {
        return (f as i64).to_string();
    }
    format!("{f}")
}
