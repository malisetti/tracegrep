use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn count_jsonl_stdin() {
    Command::cargo_bin("tracegrep")
        .unwrap()
        .args(["level = \"error\"", "--format", "count"])
        .write_stdin("{\"level\":\"error\"}\n{\"level\":\"info\"}\n")
        .assert()
        .success()
        .stdout("1\n");
}

#[test]
fn json_explicit_prints_matching_line() {
    Command::cargo_bin("tracegrep")
        .unwrap()
        .args(["msg ~ \"timeout\"", "--format", "json"])
        .write_stdin(
            "{\"level\":\"error\",\"msg\":\"timeout\"}\n{\"level\":\"info\",\"msg\":\"ok\"}\n",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("timeout"))
        .stdout(predicate::str::contains("error"));
}

#[test]
fn table_mode_includes_columns() {
    Command::cargo_bin("tracegrep")
        .unwrap()
        .args(["level = \"error\"", "--format", "table"])
        .write_stdin("{\"level\":\"error\",\"msg\":\"boom\"}\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("level"))
        .stdout(predicate::str::contains("error"));
}

#[test]
fn logfmt_input_hint_filters() {
    Command::cargo_bin("tracegrep")
        .unwrap()
        .args([
            "level = \"error\"",
            "--input",
            "logfmt",
            "--format",
            "count",
        ])
        .write_stdin("level=error msg=timeout\nlevel=info msg=ok\n")
        .assert()
        .success()
        .stdout("1\n");
}

#[test]
fn bad_query_exits_error_code() {
    Command::cargo_bin("tracegrep")
        .unwrap()
        .args(["((((", "--format", "count"])
        .write_stdin("{\"a\":1}\n")
        .assert()
        .failure()
        .code(predicate::eq(2));
}

/// Strict parsing (default): malformed lines abort with grep-style exit 2 / error.
#[test]
fn strict_default_malformed_json_aborts_exit_two() {
    Command::cargo_bin("tracegrep")
        .unwrap()
        .args(["level = \"error\"", "--input", "json", "--format", "count"])
        .write_stdin("{not-json\n{\"level\":\"error\"}\n")
        .assert()
        .failure()
        .code(predicate::eq(2));
}

/// `--no-strict` skips malformed lines and continues evaluating.
#[test]
fn no_strict_skips_malformed_still_matches() {
    Command::cargo_bin("tracegrep")
        .unwrap()
        .args([
            "level = \"error\"",
            "--input",
            "json",
            "--format",
            "count",
            "--no-strict",
        ])
        .write_stdin("{bad\n{\"level\":\"error\"}\n")
        .assert()
        .success()
        .stdout("1\n");
}

#[test]
fn no_matches_exit_one() {
    Command::cargo_bin("tracegrep")
        .unwrap()
        .args(["level = \"bogus\"", "--input", "json", "--format", "count"])
        .write_stdin("{\"level\":\"info\"}\n")
        .assert()
        .code(predicate::eq(1));
}

#[test]
fn group_by_counts_tab_lines() {
    let assert = Command::cargo_bin("tracegrep")
        .unwrap()
        .args([
            "level = \"error\"",
            "--format",
            "count",
            "--group-by",
            "host",
            "--input",
            "json",
        ])
        .write_stdin(
            "{\"level\":\"error\",\"host\":\"a\"}\n\
             {\"level\":\"error\",\"host\":\"b\"}\n\
             {\"level\":\"error\",\"host\":\"a\"}\n\
             {\"level\":\"info\",\"host\":\"z\"}\n",
        )
        .assert()
        .success();

    let out = std::str::from_utf8(&assert.get_output().stdout)
        .unwrap()
        .to_string();
    let mut ls: Vec<_> = out.lines().filter(|l| !l.is_empty()).collect();
    ls.sort_unstable();
    assert_eq!(ls, vec!["a\t2", "b\t1"]);
}

#[test]
fn field_projection_json_subsets_output() {
    Command::cargo_bin("tracegrep")
        .unwrap()
        .args([
            r#"msg ~ "hi""#,
            "--format",
            "json",
            "--field",
            "msg",
            "--input",
            "json",
        ])
        .write_stdin("{\"level\":\"error\",\"msg\":\"hi\"}\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"msg\""))
        .stdout(predicate::str::contains("\"level\"").not())
        .stdout(predicate::str::contains("\"_raw\"").not());
}

/// `--jsonl` keeps per-line flushing (`JsonLinesFormatter::write`).
#[test]
fn jsonl_flag_writes_ndjson_stdout() {
    Command::cargo_bin("tracegrep")
        .unwrap()
        .args([
            "msg ~ \"timeout\"",
            "--format",
            "json",
            "--jsonl",
            "--input",
            "json",
        ])
        .write_stdin("{\"msg\":\"timeout\"}\n{\"msg\":\"ok\"}\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("timeout"))
        .stdout(predicate::str::contains("ok").not());
}
