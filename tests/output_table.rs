use std::collections::BTreeMap;

use tracegrep::output::TableFormatter;

fn row(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn column_auto_detect_from_heterogeneous_records() {
    let mut tf = TableFormatter::new(Vec::<u8>::new(), 1000);
    tf.push_row(row(&[("host", "a"), ("msg", "alpha")]))
        .unwrap();
    tf.push_row(row(&[("msg", "beta"), ("trace", "t1")]))
        .unwrap();
    tf.flush().unwrap();

    let out = String::from_utf8(tf.into_inner()).unwrap();
    let header = out.lines().next().expect("header");
    assert!(
        header.contains("host") && header.contains("msg") && header.contains("trace"),
        "expected union of keys in header, got {header:?}"
    );

    let host = header.find("host").unwrap();
    let msg = header.find("msg").unwrap();
    let trace = header.find("trace").unwrap();
    assert!(
        host < msg && msg < trace,
        "columns should sort lexicographically inside the header ({header:?})"
    );
}

#[test]
fn width_sizing_uses_widest_cell_seen_at_flush() {
    let mut tf = TableFormatter::new(Vec::<u8>::new(), 1000);
    tf.push_row(row(&[("k", "no")])).unwrap();
    tf.push_row(row(&[("k", "sizeable_value")])).unwrap();
    tf.flush().unwrap();

    let out = String::from_utf8(tf.into_inner()).unwrap();
    let data_lines: Vec<&str> = out.lines().skip(2).collect();
    assert_eq!(data_lines.len(), 2);
    let padded_short = data_lines[0];
    let wide_trimmed = data_lines[1].trim_end();

    assert_eq!(wide_trimmed, "sizeable_value");
    assert!(
        padded_short.ends_with(' '),
        "shorter cells pad with trailing spaces ({padded_short:?})"
    );
}

#[test]
fn post_flush_streaming_with_locked_widths() {
    let mut tf = TableFormatter::new(Vec::<u8>::new(), 2);

    tf.push_row(row(&[("id", "1")])).unwrap();
    assert!(!tf.streaming());
    assert_eq!(tf.buffered_rows.len(), 1);

    tf.push_row(row(&[("id", "22")])).unwrap();
    assert!(
        tf.streaming(),
        "formatter should flip to streaming once buffer fills"
    );
    assert!(
        tf.buffered_rows.is_empty(),
        "threshold flush drains buffered rows before streaming resumes"
    );

    tf.push_row(row(&[("id", "3")])).unwrap();
    assert!(
        tf.buffered_rows.is_empty(),
        "post-threshold rows must not accumulate in-memory"
    );

    tf.flush().unwrap();
    let out = String::from_utf8(tf.into_inner()).unwrap();
    assert_eq!(
        out.lines().count(),
        5,
        "expected header + separator + 3 data rows, got {}",
        out.lines().count()
    );

    assert_eq!(
        out.lines().filter(|line| line.trim_end() == "3").count(),
        1,
        "streamed third row missing from rendered output ({out:?})"
    );
}
