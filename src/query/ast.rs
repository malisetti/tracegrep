//! Abstract syntax tree for tracegrep queries.

use regex::Regex;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    /// Regular-expression match (~).
    Match,
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Op::Eq => "=",
            Op::Ne => "!=",
            Op::Lt => "<",
            Op::Le => "<=",
            Op::Gt => ">",
            Op::Ge => ">=",
            Op::Match => "~",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone)]
pub enum Literal {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Regex(Regex),
}

impl PartialEq for Literal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Literal::Str(a), Literal::Str(b)) => a == b,
            (Literal::Int(a), Literal::Int(b)) => a == b,
            (Literal::Float(a), Literal::Float(b)) => a == b,
            (Literal::Bool(a), Literal::Bool(b)) => a == b,
            (Literal::Regex(a), Literal::Regex(b)) => a.as_str() == b.as_str(),
            _ => false,
        }
    }
}

impl Eq for Literal {}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::Str(s) => write!(f, "{s:?}"),
            Literal::Int(n) => write!(f, "{n}"),
            Literal::Float(x) => write!(f, "{x}"),
            Literal::Bool(b) => write!(f, "{b}"),
            Literal::Regex(r) => {
                write!(f, "/")?;
                escape_regex_pattern(f, r.as_str())?;
                write!(f, "/")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Pred { field: String, op: Op, rhs: Literal },
    And(Vec<Expr>),
    Or(Vec<Expr>),
    Not(Box<Expr>),
}

impl Expr {
    fn fmt_inner(&self, f: &mut fmt::Formatter<'_>, ctx: Ctx) -> fmt::Result {
        match self {
            Expr::Pred { field, op, rhs } => write!(f, "{field} {op} {rhs}"),
            Expr::And(es) => {
                let wrap_outer = matches!(ctx, Ctx::InsideOr);
                if wrap_outer {
                    write!(f, "(")?;
                }
                for (i, e) in es.iter().enumerate() {
                    if i > 0 {
                        write!(f, " AND ")?;
                    }
                    match e {
                        ch if matches!(ch, Expr::Or(_)) => {
                            write!(f, "(")?;
                            ch.fmt_inner(f, Ctx::InsideParen)?;
                            write!(f, ")")?;
                        }
                        ch => ch.fmt_inner(f, Ctx::InsideAnd)?,
                    }
                }
                if wrap_outer {
                    write!(f, ")")?;
                }
                Ok(())
            }
            Expr::Or(es) => {
                for (i, e) in es.iter().enumerate() {
                    if i > 0 {
                        write!(f, " OR ")?;
                    }
                    match e {
                        ch if matches!(ch, Expr::And(_)) => {
                            write!(f, "(")?;
                            ch.fmt_inner(f, Ctx::InsideOr)?;
                            write!(f, ")")?;
                        }
                        ch => ch.fmt_inner(f, Ctx::InsideOr)?,
                    }
                }
                Ok(())
            }
            Expr::Not(inner) => {
                write!(f, "NOT ")?;
                let paren_body = matches!(inner.as_ref(), Expr::And(_) | Expr::Or(_));
                if paren_body {
                    write!(f, "(")?;
                    inner.fmt_inner(f, Ctx::None)?;
                    write!(f, ")")
                } else {
                    inner.fmt_inner(f, Ctx::None)
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Ctx {
    None,
    InsideAnd,
    InsideOr,
    InsideParen,
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_inner(f, Ctx::None)
    }
}

fn escape_regex_pattern(f: &mut fmt::Formatter<'_>, raw: &str) -> fmt::Result {
    for ch in raw.chars() {
        if ch == '/' || ch == '\\' {
            write!(f, "\\{}", ch)?;
        } else {
            write!(f, "{ch}")?;
        }
    }
    Ok(())
}
