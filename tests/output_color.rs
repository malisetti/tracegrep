use std::io::Write;

use tracegrep::output::ColorWriter;

#[test]
fn colorize_level_enabled_error_is_red() {
    let w = ColorWriter::with_enabled(Vec::new(), true);
    let out = w.colorize_level("error");
    assert!(
        out.starts_with("\x1b[31m") && out.ends_with("\x1b[0m"),
        "unexpected wrapped output: {:?}",
        out
    );
    assert!(out.contains("error"));
}

#[test]
fn colorize_level_enabled_warn_is_yellow() {
    let w = ColorWriter::with_enabled(Vec::new(), true);
    let out = w.colorize_level("warn");
    assert!(
        out.starts_with("\x1b[33m") && out.ends_with("\x1b[0m"),
        "unexpected wrapped output: {:?}",
        out
    );
}

#[test]
fn colorize_level_enabled_info_is_cyan() {
    let w = ColorWriter::with_enabled(Vec::new(), true);
    let out = w.colorize_level("info");
    assert!(
        out.starts_with("\x1b[36m") && out.ends_with("\x1b[0m"),
        "unexpected wrapped output: {:?}",
        out
    );
}

#[test]
fn colorize_level_enabled_debug_is_dim() {
    let w = ColorWriter::with_enabled(Vec::new(), true);
    let out = w.colorize_level("debug");
    assert!(
        out.starts_with("\x1b[2m") && out.ends_with("\x1b[0m"),
        "unexpected wrapped output: {:?}",
        out
    );
}

#[test]
fn colorize_level_disabled_emits_plain_text() {
    let w = ColorWriter::with_enabled(Vec::new(), false);
    assert_eq!(w.colorize_level("error"), "error");
    assert_eq!(w.colorize_level("warn"), "warn");
    assert!(!w.colorize_level("info").contains('\x1b'));
}

#[test]
fn colorize_level_case_insensitive_known_levels_when_enabled() {
    let w = ColorWriter::with_enabled(Vec::new(), true);
    let out = w.colorize_level("ERROR");
    assert!(
        out.starts_with("\x1b[31m"),
        "unexpected wrapped output: {:?}",
        out
    );
}

#[test]
fn color_writer_forwards_writes() {
    let mut buf = Vec::new();
    {
        let mut w = ColorWriter::with_enabled(&mut buf, false);
        w.write_all(b"plain\n").unwrap();
        w.flush().unwrap();
    }
    assert_eq!(buf, b"plain\n");
}
