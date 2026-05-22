//! Predicate evaluation against a structured log [`Record`](crate::record::Record).

use std::borrow::Cow;
use std::cmp::Ordering;

use crate::query::ast::{Expr, Literal, Op};
use crate::record::{Record, Value};

impl Expr {
    /// Evaluate this expression against `r`.
    ///
    /// - A missing field makes any predicate **`false`** (no error).
    /// - Ordering and equality comparisons **coerce [`Value::Int`] and [`Value::Float`] with
    ///   [`Literal::Int`] and [`Literal::Float`].**
    /// - Regex match ([`Op::Match`]) runs [`regex::Regex::is_match`] on a string derived from the
    ///   field value.
    /// - `NOT` / `AND` / `OR` **short-circuit** left-to-right. An empty `AND` evaluates to `true`
    ///   and an empty `OR` to `false`.
    pub fn eval(&self, r: &Record) -> bool {
        match self {
            Expr::Pred { field, op, rhs } => eval_pred(r, field, *op, rhs),
            Expr::And(children) => {
                for e in children {
                    if !e.eval(r) {
                        return false;
                    }
                }
                true
            }
            Expr::Or(children) => {
                for e in children {
                    if e.eval(r) {
                        return true;
                    }
                }
                false
            }
            Expr::Not(inner) => !inner.eval(r),
        }
    }
}

fn eval_pred(r: &Record, field: &str, op: Op, rhs: &Literal) -> bool {
    let Some(lhs) = r.get(field) else {
        return false;
    };

    match op {
        Op::Eq => eq_value_literal(lhs, rhs) == Some(true),
        Op::Ne => eq_value_literal(lhs, rhs) == Some(false),
        Op::Lt | Op::Le | Op::Gt | Op::Ge => {
            let Some(ord) = cmp_value_literal(lhs, rhs) else {
                return false;
            };
            match op {
                Op::Lt => ord == Ordering::Less,
                Op::Le => ord != Ordering::Greater,
                Op::Gt => ord == Ordering::Greater,
                Op::Ge => ord != Ordering::Less,
                Op::Eq | Op::Ne | Op::Match => false,
            }
        }
        Op::Match => {
            let Literal::Regex(re) = rhs else {
                return false;
            };
            let haystack = value_as_match_str(lhs);
            re.is_match(haystack.as_ref())
        }
    }
}

fn eq_value_literal(v: &Value, lit: &Literal) -> Option<bool> {
    match (v, lit) {
        (Value::Str(a), Literal::Str(b)) => Some(a == b),
        (Value::Bool(a), Literal::Bool(b)) => Some(a == b),
        (Value::Int(a), Literal::Int(b)) => Some(a == b),
        (Value::Float(a), Literal::Float(b)) => Some(a == b),
        (Value::Int(a), Literal::Float(b)) => Some((*a as f64) == *b),
        (Value::Float(a), Literal::Int(b)) => Some(*a == (*b as f64)),
        _ => None,
    }
}

fn cmp_value_literal(v: &Value, lit: &Literal) -> Option<Ordering> {
    match (v, lit) {
        (Value::Str(a), Literal::Str(b)) => Some(a.cmp(b)),
        (Value::Bool(a), Literal::Bool(b)) => Some(a.cmp(b)),
        (Value::Int(a), Literal::Int(b)) => Some(a.cmp(b)),
        (Value::Float(a), Literal::Float(b)) => (*a).partial_cmp(b),
        (Value::Int(a), Literal::Float(b)) => (*a as f64).partial_cmp(b),
        (Value::Float(a), Literal::Int(b)) => (*a).partial_cmp(&(*b as f64)),
        _ => None,
    }
}

fn value_as_match_str(v: &Value) -> Cow<'_, str> {
    match v {
        Value::Str(s) => Cow::Borrowed(s.as_str()),
        Value::Int(n) => Cow::Owned(n.to_string()),
        Value::Float(x) => Cow::Owned(x.to_string()),
        Value::Bool(b) => Cow::Owned(b.to_string()),
        Value::Null => Cow::Borrowed(""),
    }
}
