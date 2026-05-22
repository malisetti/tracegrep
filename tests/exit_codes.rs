#[path = "src/exit_codes.rs"]
mod exit_codes;

use exit_codes::ExitCode;

#[test]
fn exit_codes_as_i32_match() {
    assert_eq!(ExitCode::Match.as_i32(), 0);
}

#[test]
fn exit_codes_as_i32_no_match() {
    assert_eq!(ExitCode::NoMatch.as_i32(), 1);
}

#[test]
fn exit_codes_as_i32_error() {
    assert_eq!(ExitCode::Error.as_i32(), 2);
}

#[test]
fn exit_codes_from_bool_maps_to_match_or_no_match() {
    assert_eq!(ExitCode::from(true), ExitCode::Match);
    assert_eq!(ExitCode::from(false), ExitCode::NoMatch);
}
