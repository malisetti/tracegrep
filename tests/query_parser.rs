use tracegrep::query::{parse, Expr, Literal, Op, QueryParseError};

#[test]
fn precedence_and_before_or() {
    let expr = parse("a = 1 or b = 2 and c = 3").expect("parse");
    let want = Expr::Or(vec![
        Expr::Pred {
            field: "a".into(),
            op: Op::Eq,
            rhs: Literal::Int(1),
        },
        Expr::And(vec![
            Expr::Pred {
                field: "b".into(),
                op: Op::Eq,
                rhs: Literal::Int(2),
            },
            Expr::Pred {
                field: "c".into(),
                op: Op::Eq,
                rhs: Literal::Int(3),
            },
        ]),
    ]);
    assert_eq!(expr, want);
}

#[test]
fn parens_override_precedence() {
    let expr = parse("( a = 1 or b = 2 ) and c = 3").expect("parse");
    let want = Expr::And(vec![
        Expr::Or(vec![
            Expr::Pred {
                field: "a".into(),
                op: Op::Eq,
                rhs: Literal::Int(1),
            },
            Expr::Pred {
                field: "b".into(),
                op: Op::Eq,
                rhs: Literal::Int(2),
            },
        ]),
        Expr::Pred {
            field: "c".into(),
            op: Op::Eq,
            rhs: Literal::Int(3),
        },
    ]);
    assert_eq!(expr, want);
}

#[test]
fn regex_match_operator() {
    let expr = parse(r#"msg ~ "timeout|deadline""#).expect("parse");
    match expr {
        Expr::Pred {
            field,
            op: Op::Match,
            rhs: Literal::Regex(re),
        } => {
            assert_eq!(field, "msg");
            assert_eq!(re.as_str(), "timeout|deadline");
            assert!(re.is_match("deadline exceeded"));
        }
        other => panic!("unexpected expr: {other:?}"),
    }
}

#[test]
fn malformed_trailing_tokens() {
    let err = parse("a = 1 foo").unwrap_err();
    assert!(matches!(err, QueryParseError::TrailingInput(_)));
}

#[test]
fn malformed_missing_rhs() {
    let err = parse("a=").unwrap_err();
    assert!(matches!(err, QueryParseError::UnexpectedEof), "{err:?}");
}

#[test]
fn malformed_unclosed_paren() {
    let err = parse("(a = 1").unwrap_err();
    assert!(
        matches!(
            err,
            QueryParseError::UnexpectedEof | QueryParseError::Expected { .. }
        ),
        "{err:?}"
    );
}

#[test]
fn malformed_invalid_regex_literal() {
    let err = parse(r#"svc ~ "[" "#).unwrap_err();
    match err {
        QueryParseError::InvalidRegex(_) => {}
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn unary_not_associates_correctly() {
    let expr = parse("not a = false and b = true").expect("parse");
    let want = Expr::And(vec![
        Expr::Not(Box::new(Expr::Pred {
            field: "a".into(),
            op: Op::Eq,
            rhs: Literal::Bool(false),
        })),
        Expr::Pred {
            field: "b".into(),
            op: Op::Eq,
            rhs: Literal::Bool(true),
        },
    ]);
    assert_eq!(expr, want);
}

#[test]
fn keyword_field_names_are_rejected() {
    let err = parse("and = 1").unwrap_err();
    assert!(
        matches!(err, QueryParseError::KeywordField { .. }),
        "{err:?}"
    );
}
