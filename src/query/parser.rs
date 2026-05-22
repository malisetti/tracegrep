//! Recursive-descent parser for tracegrep queries.

use super::ast::{Expr, Literal, Op};
use regex::Regex;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum QueryParseError {
    #[error("unexpected end of input")]
    UnexpectedEof,
    #[error("unexpected token `{0}`")]
    UnexpectedToken(String),
    #[error("unexpected trailing input `{0}`")]
    TrailingInput(String),
    #[error("expected `{expected}`")]
    Expected { expected: &'static str },
    #[error("invalid number `{0}`")]
    InvalidNumber(String),
    #[error("missing closing `\"`")]
    UnterminatedString,
    #[error("invalid regex pattern: {0}")]
    InvalidRegex(regex::Error),
    #[error("keyword `{keyword}` cannot be used as a field name")]
    KeywordField { keyword: String },
}

struct Parser<'a> {
    input: &'a str,
    buf: &'a [u8],
    pos: usize,
}

fn is_ident_cont(b: u8) -> bool {
    matches!(
        b,
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b'-'
    )
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            buf: input.as_bytes(),
            pos: 0,
        }
    }

    fn skip_ws(&mut self) {
        while let Some(&b) = self.buf.get(self.pos) {
            if b.is_ascii_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn eof(&self) -> bool {
        self.pos >= self.buf.len()
    }

    fn peek_byte(&self) -> Option<u8> {
        self.buf.get(self.pos).copied()
    }

    fn slice_span(&self, start: usize) -> &'a str {
        let end = self.pos.min(self.buf.len());
        &self.input[start..end]
    }

    fn snippet_at(&self, start: usize) -> String {
        let end = start.saturating_add(32).min(self.buf.len());
        self.input[start..end].trim().chars().take(24).collect()
    }

    fn try_keyword(&mut self, kw: &str) -> bool {
        self.skip_ws();
        if !self.input[self.pos..].starts_with(kw) {
            return false;
        }
        let boundary = !matches!(
            self.buf.get(self.pos.saturating_add(kw.len())).copied(),
            Some(b) if is_ident_cont(b)
        );
        if boundary {
            self.pos += kw.len();
            true
        } else {
            false
        }
    }

    fn expect_byte(&mut self, b: u8) -> Result<(), QueryParseError> {
        self.skip_ws();
        if self.peek_byte() != Some(b) {
            return match self.peek_byte() {
                None => Err(QueryParseError::UnexpectedEof),
                Some(c) => Err(QueryParseError::UnexpectedToken(String::from(char::from(
                    c,
                )))),
            };
        }
        self.pos += 1;
        Ok(())
    }

    fn parse_ident_for_field(&mut self) -> Result<String, QueryParseError> {
        self.skip_ws();
        let start = self.pos;
        let Some(first) = self.peek_byte() else {
            return Err(QueryParseError::UnexpectedEof);
        };
        if !first.is_ascii_alphabetic() && first != b'_' {
            return Err(QueryParseError::UnexpectedToken(self.snippet_at(start)));
        }
        self.pos += 1;
        while let Some(&b) = self.buf.get(self.pos) {
            if is_ident_cont(b) {
                self.pos += 1;
            } else {
                break;
            }
        }
        let name = self.slice_span(start).to_owned();
        if matches!(name.as_str(), "and" | "or" | "not" | "true" | "false") {
            return Err(QueryParseError::KeywordField { keyword: name });
        }
        Ok(name)
    }

    fn parse_number(&mut self) -> Result<Literal, QueryParseError> {
        let start = self.pos;
        if matches!(self.peek_byte(), Some(b'-' | b'+')) {
            self.pos += 1;
        }
        while self.peek_byte().is_some_and(|b| b.is_ascii_digit()) {
            self.pos += 1;
        }
        if self.peek_byte() == Some(b'.')
            || self.peek_byte().is_some_and(|b| matches!(b, b'e' | b'E'))
        {
            if self.peek_byte() == Some(b'.') {
                self.pos += 1;
                while self.peek_byte().is_some_and(|b| b.is_ascii_digit()) {
                    self.pos += 1;
                }
            }
            if matches!(self.peek_byte(), Some(b'e' | b'E')) {
                self.pos += 1;
                if matches!(self.peek_byte(), Some(b'+' | b'-')) {
                    self.pos += 1;
                }
                while self.peek_byte().is_some_and(|b| b.is_ascii_digit()) {
                    self.pos += 1;
                }
            }
            let s = self.slice_span(start);
            let v: f64 = s
                .parse()
                .map_err(|_| QueryParseError::InvalidNumber(s.to_string()))?;
            return Ok(Literal::Float(v));
        }
        let s = self.slice_span(start);
        if s == "+" || s == "-" {
            return Err(QueryParseError::InvalidNumber(s.to_string()));
        }
        let v: i64 = s
            .parse()
            .map_err(|_| QueryParseError::InvalidNumber(s.to_string()))?;
        Ok(Literal::Int(v))
    }

    fn parse_string_literal(&mut self) -> Result<String, QueryParseError> {
        self.expect_byte(b'"')?;
        let mut out = String::new();
        while let Some(&b) = self.buf.get(self.pos) {
            if b == b'"' {
                self.pos += 1;
                return Ok(out);
            }
            if b == b'\\' {
                self.pos += 1;
                let Some(esc) = self.buf.get(self.pos).copied() else {
                    return Err(QueryParseError::UnterminatedString);
                };
                self.pos += 1;
                match esc {
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'\\' => out.push('\\'),
                    b'"' => out.push('"'),
                    x => out.push(char::from(x)),
                }
            } else {
                out.push(char::from(b));
                self.pos += 1;
            }
        }
        Err(QueryParseError::UnterminatedString)
    }

    fn parse_literal(&mut self) -> Result<Literal, QueryParseError> {
        self.skip_ws();
        match self.peek_byte() {
            Some(b'"') => {
                let s = self.parse_string_literal()?;
                Ok(Literal::Str(s))
            }
            Some(b't' | b'f') => {
                if self.try_keyword("true") {
                    Ok(Literal::Bool(true))
                } else if self.try_keyword("false") {
                    Ok(Literal::Bool(false))
                } else {
                    Err(QueryParseError::UnexpectedToken(self.snippet_at(self.pos)))
                }
            }
            Some(b'0'..=b'9') | Some(b'+') | Some(b'-') => self.parse_number(),
            None => Err(QueryParseError::UnexpectedEof),
            _ => Err(QueryParseError::UnexpectedToken(self.snippet_at(self.pos))),
        }
    }

    fn parse_predicate(&mut self) -> Result<Expr, QueryParseError> {
        let field = self.parse_ident_for_field()?;
        self.skip_ws();
        let op = if self.input[self.pos..].starts_with("!=") {
            self.pos += 2;
            Op::Ne
        } else if self.input[self.pos..].starts_with("<=") {
            self.pos += 2;
            Op::Le
        } else if self.input[self.pos..].starts_with(">=") {
            self.pos += 2;
            Op::Ge
        } else if self.peek_byte() == Some(b'=') {
            self.pos += 1;
            Op::Eq
        } else if self.peek_byte() == Some(b'<') {
            self.pos += 1;
            Op::Lt
        } else if self.peek_byte() == Some(b'>') {
            self.pos += 1;
            Op::Gt
        } else if self.peek_byte() == Some(b'~') {
            self.pos += 1;
            Op::Match
        } else {
            return Err(QueryParseError::Expected {
                expected: "operator",
            });
        };

        let mut rhs = self.parse_literal()?;
        if matches!(op, Op::Match) {
            let pattern = match rhs {
                Literal::Str(s) => s,
                other => {
                    return Err(QueryParseError::UnexpectedToken(format!("{other:?}")));
                }
            };
            let re = Regex::new(&pattern).map_err(QueryParseError::InvalidRegex)?;
            rhs = Literal::Regex(re);
        }
        Ok(Expr::Pred { field, op, rhs })
    }

    fn parse_primary(&mut self) -> Result<Expr, QueryParseError> {
        self.skip_ws();
        if self.peek_byte() == Some(b'(') {
            self.pos += 1;
            let inner = self.parse_expr()?;
            self.expect_byte(b')')?;
            return Ok(inner);
        }
        self.parse_predicate()
    }

    fn parse_unary(&mut self) -> Result<Expr, QueryParseError> {
        if self.try_keyword("not") {
            return Ok(Expr::Not(Box::new(self.parse_unary()?)));
        }
        self.parse_primary()
    }

    fn parse_and(&mut self) -> Result<Expr, QueryParseError> {
        let first = self.parse_unary()?;
        let mut parts = vec![first];
        while self.try_keyword("and") {
            parts.push(self.parse_unary()?);
        }
        Ok(if parts.len() == 1 {
            parts.remove(0)
        } else {
            Expr::And(parts)
        })
    }

    fn parse_or(&mut self) -> Result<Expr, QueryParseError> {
        let first = self.parse_and()?;
        let mut parts = vec![first];
        while self.try_keyword("or") {
            parts.push(self.parse_and()?);
        }
        Ok(if parts.len() == 1 {
            parts.remove(0)
        } else {
            Expr::Or(parts)
        })
    }

    fn parse_expr(&mut self) -> Result<Expr, QueryParseError> {
        self.parse_or()
    }
}

/// Parse a query string into an expression tree.
pub fn parse(input: &str) -> Result<Expr, QueryParseError> {
    let mut p = Parser::new(input);
    let expr = p.parse_expr()?;
    p.skip_ws();
    if !p.eof() {
        return Err(QueryParseError::TrailingInput(p.input[p.pos..].to_string()));
    }
    Ok(expr)
}
