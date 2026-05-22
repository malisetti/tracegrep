use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tracegrep::{parse_json_line, query};

fn bench_eval_simple(c: &mut Criterion) {
    let r = parse_json_line("{\"level\":\"error\",\"msg\":\"timeout\",\"latency\":523}").unwrap();
    let expr = query::parser::parse("level = \"error\" and latency > 100").unwrap();
    c.bench_function("eval_simple", |b| {
        b.iter(|| black_box(expr.eval(black_box(&r))))
    });
}

fn bench_eval_regex(c: &mut Criterion) {
    let r =
        parse_json_line("{\"level\":\"warn\",\"msg\":\"connection deadline exceeded\",\"ms\":900}")
            .unwrap();
    let expr = query::parser::parse(r#"msg ~ "deadline|timeout""#).unwrap();
    c.bench_function("eval_regex", |b| {
        b.iter(|| black_box(expr.eval(black_box(&r))))
    });
}

fn bench_eval_compound(c: &mut Criterion) {
    let r =
        parse_json_line("{\"level\":\"error\",\"svc\":\"auth\",\"latency\":42,\"retry\":false}")
            .unwrap();
    let expr = query::parser::parse(
        r#"(level = "error" and latency > 40) or (svc = "other" and not retry = false)"#,
    )
    .unwrap();
    c.bench_function("eval_compound", |b| {
        b.iter(|| black_box(expr.eval(black_box(&r))))
    });
}

criterion_group!(
    benches,
    bench_eval_simple,
    bench_eval_regex,
    bench_eval_compound
);
criterion_main!(benches);
