// tests/args_tests.rs

use anyhow::Result;
use freminal_common::args::Args;

fn parse_from<I: IntoIterator<Item = S>, S: Into<String>>(args: I) -> Result<Args> {
    Args::parse(args.into_iter().map(Into::into))
}

#[test]
fn parses_empty_args_defaults() {
    let args = parse_from(["freminal"]).unwrap();
    assert!(args.recording.is_none());
    assert!(args.shell.is_none());
    assert!(!args.show_all_debug);
    #[cfg(debug_assertions)]
    assert!(args.write_logs_to_file);
    #[cfg(not(debug_assertions))]
    assert!(!args.write_logs_to_file);
}

#[test]
fn parses_recording_path() {
    let args = parse_from(["freminal", "--recording-path", "rec.log"]).unwrap();
    assert_eq!(args.recording.as_deref(), Some("rec.log"));
}

#[test]
fn missing_recording_path_argument() {
    let result = parse_from(["freminal", "--recording-path"]);
    assert!(result.is_err());
}

#[test]
fn parses_shell_argument() {
    let args = parse_from(["freminal", "--shell", "/bin/bash"]).unwrap();
    assert_eq!(args.shell.as_deref(), Some("/bin/bash"));
}

#[test]
fn missing_shell_argument() {
    let result = parse_from(["freminal", "--shell"]);
    assert!(result.is_err());
}

#[test]
fn parses_show_all_debug_flag() {
    let args = parse_from(["freminal", "--show-all-debug"]).unwrap();
    assert!(args.show_all_debug);
}

#[test]
fn parses_write_logs_to_file_true() {
    let args = parse_from(["freminal", "--write-logs-to-file=true"]).unwrap();
    assert!(args.write_logs_to_file);
}

#[test]
fn parses_write_logs_to_file_false() {
    let args = parse_from(["freminal", "--write-logs-to-file=false"]).unwrap();
    assert!(!args.write_logs_to_file);
}

#[test]
fn missing_write_logs_to_file_value() {
    let result = parse_from(["freminal", "--write-logs-to-file"]);
    assert!(result.is_err());
}

#[test]
fn invalid_write_logs_to_file_value() {
    let result = parse_from(["freminal", "--write-logs-to-file=maybe"]);
    assert!(result.is_err());
}

#[test]
fn invalid_argument_is_error() {
    let result = parse_from(["freminal", "--not-a-real-flag"]);
    assert!(result.is_err());
}

#[test]
fn help_flag_does_not_error() {
    let result = parse_from(["freminal", "--help"]);
    // help just prints but shouldn't fail
    assert!(result.is_ok());
}
