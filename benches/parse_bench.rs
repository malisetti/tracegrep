use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tracegrep::{parse_json_line, parse_logfmt_line};

fn json_fixture(n: usize) -> Vec<String> {
    (0..n)
        .map(|i| {
            format!(
                "{{\"seq\":{},\"level\":\"info\",\"msg\":\"evt-{}\",\"elapsed_ms\":{}}}",
                i,
                i % 97,
                i * 7
            )
        })
        .collect()
}

fn logfmt_fixture(n: usize) -> Vec<String> {
    (0..n)
        .map(|i| {
            format!(
                "seq={} level=info msg=evt-{} elapsed_ms={}",
                i,
                i % 97,
                i * 7
            )
        })
        .collect()
}

fn parse_benchmarks(c: &mut Criterion) {
    let json_lines = json_fixture(1000);
    let logfmt_lines = logfmt_fixture(1000);

    c.bench_function("parse_json_line_1000_records", |b| {
        b.iter(|| {
            for line in black_box(json_lines.as_slice()) {
                let _ = parse_json_line(black_box(line.as_str())).unwrap();
            }
        })
    });

    c.bench_function("parse_logfmt_line_1000_records", |b| {
        b.iter(|| {
            for line in black_box(logfmt_lines.as_slice()) {
                let _ = parse_logfmt_line(black_box(line.as_str())).unwrap();
            }
        })
    });
}

criterion_group!(benches, parse_benchmarks);
criterion_main!(benches);
