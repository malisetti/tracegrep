//! Golden integration tests covering v020 CLI flags via insta snapshots.

use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;

fn tracegrep_bin() -> Command {
    Command::cargo_bin("tracegrep").expect("cargo compiled tracegrep")
}

fn v020_fixture(name: impl AsRef<Path>) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/v020")
        .join(name)
}

fn assert_snapshot_stderr(snapshot_file: &'static str, stderr: &[u8]) {
    let s = std::str::from_utf8(stderr).expect("stderr utf8");
    insta::with_settings!({
        snapshot_path => "golden",
        prepend_module_to_snapshot => false,
    }, {
        insta::assert_snapshot!(snapshot_file, s);
    });
}

fn assert_snapshot_stdout(snapshot_file: &'static str, stdout: &[u8]) {
    let s = std::str::from_utf8(stdout).expect("stdout utf8");
    insta::with_settings!({
        snapshot_path => "golden",
        prepend_module_to_snapshot => false,
    }, {
        insta::assert_snapshot!(snapshot_file, s);
    });
}

/// `--strict` default (`true`): first bad physical line emits diagnostic stderr and exits 2.
#[test]
fn strict_default_bad_line_diagnostic_exit_two() {
    let path = v020_fixture("bad_line_skip.jsonl");
    let assert = tracegrep_bin()
        .args([r#"kind = "warm""#, "--input", "json", "--format", "json"])
        .arg(path.as_os_str())
        .assert()
        .failure()
        .code(predicate::eq(2));
    assert_snapshot_stderr(
        "stderr_strict_bad_line_skip",
        assert.get_output().stderr.as_slice(),
    );
}

/// `--no-strict`: malformed line skipped with stderr warning summary; stdout reflects matches.
#[test]
fn no_strict_bad_line_stderr_and_matches_stdout() {
    let path = v020_fixture("bad_line_skip.jsonl");
    let output = tracegrep_bin()
        .args([
            r#"kind = "warm""#,
            "--input",
            "json",
            "--no-strict",
            "--format",
            "json",
        ])
        .arg(path.as_os_str())
        .output()
        .expect("spawn");

    assert_snapshot_stderr("stderr_no_strict_bad_line_skip", &output.stderr);
    assert_snapshot_stdout("stdout_no_strict_bad_line_skip_matches", &output.stdout);
    assert!(output.status.success(), "expects exit code 0 (matches)");
}

/// `--jsonl` produces the same per-record JSON payloads as `--format=json` batch mode here.
#[test]
fn jsonl_stdout_matches_plain_json_formatter() {
    let path = v020_fixture("stream_multi.jsonl");
    let plain = tracegrep_bin()
        .args([r#"svc ~ "alp""#, "--input", "json", "--format", "json"])
        .arg(path.as_os_str())
        .output()
        .expect("json run");
    let jsonl = tracegrep_bin()
        .args([
            r#"svc ~ "alp""#,
            "--input",
            "json",
            "--format",
            "json",
            "--jsonl",
        ])
        .arg(path.as_os_str())
        .output()
        .expect("jsonl run");

    assert_eq!(
        plain.stdout, jsonl.stdout,
        "stdout must match byte-for-byte"
    );
    assert_snapshot_stdout("stdout_json_and_jsonl_alp_match", &plain.stdout);
}

/// `--field` projection: emitted JSON keeps only declared keys after filtering.
#[test]
fn field_projection_json_subset() {
    let path = v020_fixture("stream_multi.jsonl");
    let out = tracegrep_bin()
        .args([
            "all = true",
            "--input",
            "json",
            "--format",
            "json",
            "--field",
            "level,svc",
        ])
        .arg(path.as_os_str())
        .assert()
        .success();
    assert_snapshot_stdout(
        "stdout_field_projection_level_svc",
        out.get_output().stdout.as_slice(),
    );
}

/// `--group-by level` with `--format=count`: tab-separated buckets sorted lexicographically.
#[test]
fn group_by_level_sorted_counts() {
    let path = v020_fixture("stream_multi.jsonl");
    let out = tracegrep_bin()
        .args([
            "all = true",
            "--input",
            "json",
            "--format",
            "count",
            "--group-by",
            "level",
        ])
        .arg(path.as_os_str())
        .assert()
        .success();
    assert_snapshot_stdout(
        "stdout_group_by_level_counts",
        out.get_output().stdout.as_slice(),
    );
}

/// Mixed NDJSON plus logfmt lines with `--input=auto`.
#[test]
fn mixed_json_and_logfmt_input_auto_stdout() {
    let path = v020_fixture("mixed_auto.txt");
    let assert = tracegrep_bin()
        .args([
            r#"(tier = "silver") or (code = "READY")"#,
            "--input",
            "auto",
            "--format",
            "json",
        ])
        .arg(path.as_os_str())
        .assert()
        .success();

    assert_snapshot_stdout(
        "stdout_mixed_auto_tier_or_ready",
        assert.get_output().stdout.as_slice(),
    );
}

#[test]
fn grep_exit_codes_zero_one_two_fixture_backed() {
    let exit0 = tracegrep_bin()
        .args([r#"tier = "gold""#, "--input", "auto", "--format", "count"])
        .arg(v020_fixture("mixed_auto.txt").as_os_str())
        .assert()
        .success()
        .code(predicate::eq(0));

    insta::with_settings!({
        snapshot_path => "golden",
        prepend_module_to_snapshot => false,
    }, {
        insta::assert_snapshot!(
            "stdout_exit0_match_mixed_auto",
            std::str::from_utf8(&exit0.get_output().stdout).unwrap()
        );
    });

    tracegrep_bin()
        .args([
            r#"tier = "absent-tier""#,
            "--input",
            "auto",
            "--format",
            "count",
        ])
        .arg(v020_fixture("mixed_auto.txt").as_os_str())
        .assert()
        .failure()
        .code(predicate::eq(1));

    tracegrep_bin()
        .args([r#"tier = "gold""#, "--input", "json", "--format", "json"])
        .arg(v020_fixture("bad_line_skip.jsonl").as_os_str())
        .assert()
        .failure()
        .code(predicate::eq(2));

    tracegrep_bin()
        .args(["((((/", "--input", "json", "--format", "count"])
        .arg(v020_fixture("stream_multi.jsonl").as_os_str())
        .assert()
        .failure()
        .code(predicate::eq(2));
}
