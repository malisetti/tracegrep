use std::cmp::Ordering;
use std::collections::BTreeMap;

use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Record {
    fields: BTreeMap<String, Value>,
    raw: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Field<'a> {
    pub name: &'a str,
    pub value: &'a Value,
}

#[derive(Debug, Error)]
pub enum RecordError {
    #[error("parse: {0}")]
    Parse(String),
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        use Value::*;
        match (self, other) {
            (Str(a), Str(b)) => Some(a.cmp(b)),
            (Int(a), Int(b)) => Some(a.cmp(b)),
            (Float(a), Float(b)) => a.partial_cmp(b),
            (Int(a), Float(b)) => (*a as f64).partial_cmp(b),
            (Float(a), Int(b)) => a.partial_cmp(&(*b as f64)),
            (Bool(a), Bool(b)) => Some(a.cmp(b)),
            (Null, Null) => Some(Ordering::Equal),
            _ => None,
        }
    }
}

impl Record {
    pub fn new(raw: impl Into<String>) -> Self {
        Self {
            fields: BTreeMap::new(),
            raw: raw.into(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.fields.get(name)
    }

    pub fn insert(&mut self, name: impl Into<String>, value: Value) {
        self.fields.insert(name.into(), value);
    }

    pub fn raw(&self) -> &str {
        self.raw.as_str()
    }

    pub fn iter(&self) -> impl Iterator<Item = Field<'_>> + '_ {
        self.fields.iter().map(|(name, value)| Field {
            name: name.as_str(),
            value,
        })
    }
}
