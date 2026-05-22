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
fn bad_query_exits_nonzero() {
    Command::cargo_bin("tracegrep")
        .unwrap()
        .args(["((((", "--format", "count"])
        .write_stdin("{\"a\":1}\n")
        .assert()
        .failure();
}
