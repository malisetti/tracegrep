#[path = "src/diagnostics.rs"]
mod diagnostics;

use diagnostics::DiagnosticsSink;
use std::io::Cursor;
use std::str;

#[test]
fn diagnostics_record_skip_writes_exact_warning_line() {
    let mut sink = DiagnosticsSink::new(Cursor::new(Vec::<u8>::new()));
    sink.record_skip(3, "unparseable");
    let buf = sink.into_inner().into_inner();
    assert_eq!(
        str::from_utf8(&buf).unwrap(),
        "tracegrep: warning: line 3: unparseable\n"
    );
}

#[test]
fn diagnostics_two_skips_reflect_in_summary_lines_tally() {
    let mut sink = DiagnosticsSink::new(Cursor::new(Vec::<u8>::new()));
    sink.record_skip(10, "a");
    sink.record_skip(11, "b");
    sink.summary().unwrap();
    assert_eq!(
        str::from_utf8(&sink.into_inner().into_inner()).unwrap(),
        concat!(
            "tracegrep: warning: line 10: a\n",
            "tracegrep: warning: line 11: b\n",
            "tracegrep: 2 lines skipped (use --strict to abort on first)\n"
        )
    );
}

#[test]
fn diagnostics_record_error_writes_exact_error_line() {
    let mut sink = DiagnosticsSink::new(Cursor::new(Vec::<u8>::new()));
    sink.record_error(7, "boom");
    let buf = sink.into_inner().into_inner();
    assert_eq!(
        str::from_utf8(&buf).unwrap(),
        "tracegrep: error: line 7: boom\n"
    );
}

#[test]
fn diagnostics_summary_emits_error_tally_plural_and_singular() {
    let mut sink = DiagnosticsSink::new(Cursor::new(Vec::<u8>::new()));
    sink.record_error(1, "e1");
    sink.record_error(2, "e2");
    sink.summary().unwrap();
    assert_eq!(
        str::from_utf8(&sink.into_inner().into_inner()).unwrap(),
        concat!(
            "tracegrep: error: line 1: e1\n",
            "tracegrep: error: line 2: e2\n",
            "tracegrep: 2 errors\n"
        )
    );

    let mut sink = DiagnosticsSink::new(Cursor::new(Vec::<u8>::new()));
    sink.record_error(99, "only");
    sink.summary().unwrap();
    assert_eq!(
        str::from_utf8(&sink.into_inner().into_inner()).unwrap(),
        concat!("tracegrep: error: line 99: only\n", "tracegrep: 1 error\n")
    );
}

#[test]
fn diagnostics_summary_formats_example_twelve_lines_skipped_line() {
    let mut sink = DiagnosticsSink::new(Cursor::new(Vec::<u8>::new()));
    for n in 1..=12 {
        sink.record_skip(n, "bad line");
    }
    sink.summary().unwrap();
    assert!(str::from_utf8(&sink.into_inner().into_inner())
        .unwrap()
        .ends_with("tracegrep: 12 lines skipped (use --strict to abort on first)\n"));
}

#[test]
fn diagnostics_summary_writes_nothing_when_no_skips_or_errors() {
    let mut sink = DiagnosticsSink::new(Cursor::new(Vec::<u8>::new()));
    sink.summary().unwrap();
    assert!(sink.into_inner().into_inner().is_empty());
}

#[test]
fn diagnostics_summary_emits_skip_then_error_tally() {
    let mut sink = DiagnosticsSink::new(Cursor::new(Vec::<u8>::new()));
    sink.record_skip(5, "s");
    sink.record_error(6, "e");
    sink.summary().unwrap();
    assert_eq!(
        str::from_utf8(&sink.into_inner().into_inner()).unwrap(),
        concat!(
            "tracegrep: warning: line 5: s\n",
            "tracegrep: error: line 6: e\n",
            "tracegrep: 1 line skipped (use --strict to abort on first)\n",
            "tracegrep: 1 error\n"
        )
    );
}
