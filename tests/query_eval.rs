use regex::Regex;

use tracegrep::query::{Expr, Literal, Op};
use tracegrep::{Record, Value};

#[test]
fn eval_missing_field_predicate_is_false() {
    let r = Record::new("");
    assert!(!Expr::Pred {
        field: "lvl".into(),
        op: Op::Eq,
        rhs: Literal::Str("info".into()),
    }
    .eval(&r));
}

#[test]
fn eval_predicate_eq_types() {
    let mut r = Record::new("");
    r.insert("msg", Value::Str("hello".into()));
    r.insert("ok", Value::Bool(true));
    r.insert("n", Value::Int(42));
    r.insert("x", Value::Float(2.5));

    assert!(Expr::Pred {
        field: "msg".into(),
        op: Op::Eq,
        rhs: Literal::Str("hello".into()),
    }
    .eval(&r));
    assert!(!Expr::Pred {
        field: "msg".into(),
        op: Op::Eq,
        rhs: Literal::Str("nope".into()),
    }
    .eval(&r));

    assert!(Expr::Pred {
        field: "ok".into(),
        op: Op::Eq,
        rhs: Literal::Bool(true),
    }
    .eval(&r));

    assert!(Expr::Pred {
        field: "n".into(),
        op: Op::Eq,
        rhs: Literal::Int(42),
    }
    .eval(&r));

    assert!(Expr::Pred {
        field: "x".into(),
        op: Op::Eq,
        rhs: Literal::Float(2.5),
    }
    .eval(&r));
}

#[test]
fn eval_predicate_numeric_coercion_eq_and_ord() {
    let mut r = Record::new("");
    r.insert("i", Value::Int(10));
    r.insert("f", Value::Float(10.0));

    assert!(Expr::Pred {
        field: "i".into(),
        op: Op::Eq,
        rhs: Literal::Float(10.0),
    }
    .eval(&r));

    assert!(Expr::Pred {
        field: "f".into(),
        op: Op::Eq,
        rhs: Literal::Int(10),
    }
    .eval(&r));

    assert!(Expr::Pred {
        field: "i".into(),
        op: Op::Lt,
        rhs: Literal::Float(11.0),
    }
    .eval(&r));

    assert!(Expr::Pred {
        field: "f".into(),
        op: Op::Ge,
        rhs: Literal::Int(10),
    }
    .eval(&r));
}

#[test]
fn eval_predicate_ne_incompatible_types_yields_false() {
    let mut r = Record::new("");
    r.insert("msg", Value::Str("x".into()));

    assert!(!Expr::Pred {
        field: "msg".into(),
        op: Op::Ne,
        rhs: Literal::Int(3),
    }
    .eval(&r));
    assert!(!Expr::Pred {
        field: "msg".into(),
        op: Op::Eq,
        rhs: Literal::Int(3),
    }
    .eval(&r));
}

#[test]
fn eval_predicate_regex_match_uses_regex_is_match() {
    let mut r = Record::new("");
    r.insert("path", Value::Str("/health v2".into()));
    assert!(Expr::Pred {
        field: "path".into(),
        op: Op::Match,
        rhs: Literal::Regex(Regex::new("health").unwrap()),
    }
    .eval(&r));
    assert!(!Expr::Pred {
        field: "path".into(),
        op: Op::Match,
        rhs: Literal::Regex(Regex::new("^timeout$").unwrap()),
    }
    .eval(&r));
}

#[test]
fn eval_predicate_regex_matches_coerced_numeric_string() {
    let mut r = Record::new("");
    r.insert("code", Value::Int(429));
    assert!(Expr::Pred {
        field: "code".into(),
        op: Op::Match,
        rhs: Literal::Regex(Regex::new(r"^\d+$").unwrap()),
    }
    .eval(&r));
}

#[test]
fn eval_and_requires_all_true_short_circuits_false() {
    let mut r = Record::new("");
    r.insert("a", Value::Bool(true));

    assert!(!Expr::And(vec![
        Expr::Pred {
            field: "a".into(),
            op: Op::Eq,
            rhs: Literal::Bool(false),
        },
        Expr::Pred {
            field: "missing".into(),
            op: Op::Eq,
            rhs: Literal::Bool(true),
        },
    ])
    .eval(&r));
}

#[test]
fn eval_or_requires_any_true_short_circuits_true() {
    let mut r = Record::new("");
    r.insert("a", Value::Bool(true));

    assert!(Expr::Or(vec![
        Expr::Pred {
            field: "a".into(),
            op: Op::Eq,
            rhs: Literal::Bool(true),
        },
        Expr::Pred {
            field: "missing".into(),
            op: Op::Eq,
            rhs: Literal::Bool(true),
        },
    ])
    .eval(&r));
}

#[test]
fn eval_not_inverts() {
    let mut r = Record::new("");
    r.insert("x", Value::Bool(false));

    assert!(Expr::Not(Box::new(Expr::Pred {
        field: "x".into(),
        op: Op::Eq,
        rhs: Literal::Bool(true),
    }))
    .eval(&r));
}

#[test]
fn eval_and_or_not_combo() {
    let mut r = Record::new("");
    r.insert("lvl", Value::Str("error".into()));
    r.insert("msg", Value::Str("timed out contacting db".into()));

    let expr = Expr::And(vec![
        Expr::Pred {
            field: "lvl".into(),
            op: Op::Eq,
            rhs: Literal::Str("error".into()),
        },
        Expr::Or(vec![
            Expr::Not(Box::new(Expr::Pred {
                field: "svc".into(),
                op: Op::Eq,
                rhs: Literal::Str("payments".into()),
            })),
            Expr::Pred {
                field: "msg".into(),
                op: Op::Match,
                rhs: Literal::Regex(Regex::new("db").unwrap()),
            },
        ]),
    ]);

    assert!(expr.eval(&r));
}
