use regex::Regex;
use tracegrep::query::{Expr, Literal, Op};

fn split_top_level<'a>(s: &'a str, sep: &str) -> Option<Vec<&'a str>> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth = 0i32;
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < s.len() {
        match bytes[i] {
            b'"' => {
                i += 1;
                while i < s.len() {
                    match bytes[i] {
                        b'\\' => i += 2,
                        b'"' => {
                            i += 1;
                            break;
                        }
                        _ => i += 1,
                    }
                }
            }
            b'/' => {
                i += 1;
                while i < s.len() {
                    match bytes[i] {
                        b'\\' => i += 2,
                        b'/' => {
                            i += 1;
                            break;
                        }
                        _ => i += 1,
                    }
                }
            }
            b'(' => {
                depth += 1;
                i += 1;
            }
            b')' => {
                depth -= 1;
                i += 1;
            }
            _ if depth == 0 && s[i..].starts_with(sep) => {
                parts.push(&s[start..i]);
                i += sep.len();
                start = i;
            }
            _ => i += 1,
        }
    }
    if parts.is_empty() {
        return None;
    }
    parts.push(&s[start..]);
    Some(parts.into_iter().map(str::trim).collect())
}

fn parse_expr(mut s: &str) -> Expr {
    s = s.trim();
    if let Some(or_parts) = split_top_level(s, " OR ") {
        return Expr::Or(or_parts.into_iter().map(parse_and).collect());
    }
    parse_and(s)
}

fn parse_and(mut s: &str) -> Expr {
    s = s.trim();
    if let Some(and_parts) = split_top_level(s, " AND ") {
        return Expr::And(and_parts.into_iter().map(parse_unary).collect());
    }
    parse_unary(s)
}

fn matching_close_paren(s: &str, open_index: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.get(open_index) != Some(&b'(') {
        return None;
    }
    let mut depth = 0i32;
    let mut i = open_index;
    while i < s.len() {
        match bytes[i] {
            b'"' => {
                i += 1;
                while i < s.len() {
                    match bytes[i] {
                        b'\\' => i += 2,
                        b'"' => {
                            i += 1;
                            break;
                        }
                        _ => i += 1,
                    }
                }
            }
            b'/' => {
                i += 1;
                while i < s.len() {
                    match bytes[i] {
                        b'\\' => i += 2,
                        b'/' => {
                            i += 1;
                            break;
                        }
                        _ => i += 1,
                    }
                }
            }
            b'(' => {
                depth += 1;
                i += 1;
            }
            b')' => {
                depth -= 1;
                i += 1;
                if depth == 0 {
                    return Some(i - 1);
                }
            }
            _ => i += 1,
        }
    }
    None
}

fn strip_outer_wrapping_parens(s: &str) -> &str {
    let t = s.trim();
    let Some(close) = matching_close_paren(t, 0) else {
        return t;
    };
    if close == t.len() - 1 {
        &t[1..close]
    } else {
        t
    }
}

fn parse_unary(mut s: &str) -> Expr {
    s = s.trim();
    if let Some(rest) = s.strip_prefix("NOT ") {
        return Expr::Not(Box::new(parse_unary(rest)));
    }
    if s.starts_with('(') && s.ends_with(')') {
        return parse_expr(strip_outer_wrapping_parens(s));
    }
    parse_pred(s)
}

fn parse_pred(mut s: &str) -> Expr {
    s = s.trim();
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len()
        && (bytes[i].is_ascii_alphanumeric()
            || bytes[i] == b'.'
            || bytes[i] == b'_'
            || bytes[i] == b'-')
    {
        i += 1;
    }
    if i == 0 {
        panic!("invalid predicate (missing field): {s:?}");
    }
    let field = &s[..i];
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    let rest = &s[i..];
    let (op, tail) = if let Some(r) = rest.strip_prefix("!=") {
        (Op::Ne, r)
    } else if let Some(r) = rest.strip_prefix("<=") {
        (Op::Le, r)
    } else if let Some(r) = rest.strip_prefix(">=") {
        (Op::Ge, r)
    } else if let Some(r) = rest.strip_prefix('=') {
        (Op::Eq, r)
    } else if let Some(r) = rest.strip_prefix('<') {
        (Op::Lt, r)
    } else if let Some(r) = rest.strip_prefix('>') {
        (Op::Gt, r)
    } else if let Some(r) = rest.strip_prefix('~') {
        (Op::Match, r)
    } else {
        panic!("unknown operator at {rest:?}");
    };
    let rhs = parse_literal(tail.trim_start());
    Expr::Pred {
        field: field.to_string(),
        op,
        rhs,
    }
}

fn parse_literal(s: &str) -> Literal {
    let s = s.trim();
    if s.starts_with('"') {
        return Literal::Str(parse_rust_string(s));
    }
    if s.starts_with('/') {
        return Literal::Regex(parse_regex_literal(s));
    }
    match s {
        "true" => return Literal::Bool(true),
        "false" => return Literal::Bool(false),
        _ => {}
    }
    if let Ok(v) = s.parse::<i64>() {
        return Literal::Int(v);
    }
    if let Ok(v) = s.parse::<f64>() {
        return Literal::Float(v);
    }
    panic!("unhandled literal {s:?}")
}

fn parse_rust_string(s: &str) -> String {
    let mut chars = s.chars();
    assert_eq!(chars.next(), Some('"'));
    let mut out = String::new();
    while let Some(ch) = chars.next() {
        if ch == '"' {
            break;
        }
        if ch == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some(c) => out.push(c),
                None => panic!("dangling escape"),
            }
        } else {
            out.push(ch);
        }
    }
    out
}

fn parse_regex_literal(s: &str) -> Regex {
    let mut chars = s.chars();
    assert_eq!(chars.next(), Some('/'));
    let mut raw = String::new();
    while let Some(ch) = chars.next() {
        if ch == '/' {
            break;
        }
        if ch == '\\' {
            match chars.next().expect("regex escape") {
                '/' => raw.push('/'),
                '\\' => raw.push('\\'),
                c => panic!("unsupported regex escape: {c:?}"),
            }
        } else {
            raw.push(ch);
        }
    }
    Regex::new(&raw).expect("valid regex literal")
}

fn roundtrip_same(expr: Expr) {
    let printed = expr.to_string();
    let replay = parse_expr(&printed);
    assert_eq!(
        format!("{expr:#?}"),
        format!("{replay:#?}"),
        "debug mismatch printed={printed:?}",
    );
    assert_eq!(printed, replay.to_string(), "pretty round-trip mismatch");
}

#[test]
fn query_ast_literals_and_preds_roundtrip() {
    roundtrip_same(Expr::Pred {
        field: "code".into(),
        op: Op::Match,
        rhs: Literal::Regex(Regex::new(r"^\d+$").unwrap()),
    });
    roundtrip_same(Expr::Pred {
        field: "status".into(),
        op: Op::Ge,
        rhs: Literal::Int(400),
    });
    roundtrip_same(Expr::Pred {
        field: "msg".into(),
        op: Op::Eq,
        rhs: Literal::Str(String::from("hello \"world\"")),
    });
    roundtrip_same(Expr::Pred {
        field: "ok".into(),
        op: Op::Eq,
        rhs: Literal::Bool(true),
    });
}

#[test]
fn query_ast_bool_and_not_roundtrip() {
    roundtrip_same(Expr::Not(Box::new(Expr::Pred {
        field: "x".into(),
        op: Op::Ne,
        rhs: Literal::Float(1.5),
    })));
}

#[test]
fn query_ast_and_or_precedence_roundtrip() {
    let a = Expr::Pred {
        field: "lvl".into(),
        op: Op::Eq,
        rhs: Literal::Str(String::from("error")),
    };
    let b = Expr::Pred {
        field: "msg".into(),
        op: Op::Match,
        rhs: Literal::Regex(Regex::new("timeout").unwrap()),
    };
    let c = Expr::Pred {
        field: "svc".into(),
        op: Op::Eq,
        rhs: Literal::Str(String::from("api")),
    };
    let or_branch = Expr::Or(vec![
        Expr::And(vec![
            Expr::Pred {
                field: "x".into(),
                op: Op::Gt,
                rhs: Literal::Int(0),
            },
            Expr::Pred {
                field: "y".into(),
                op: Op::Lt,
                rhs: Literal::Int(10),
            },
        ]),
        Expr::Pred {
            field: "z".into(),
            op: Op::Eq,
            rhs: Literal::Bool(false),
        },
    ]);
    let expr = Expr::And(vec![Expr::Or(vec![Expr::And(vec![a, b]), c]), or_branch]);
    roundtrip_same(expr);
}
