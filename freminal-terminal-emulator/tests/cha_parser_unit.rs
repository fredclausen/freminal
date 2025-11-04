// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use freminal_terminal_emulator::ansi::{ParserInner, TerminalOutput};
use freminal_terminal_emulator::ansi_components::csi_commands::cha::ansi_parser_inner_csi_finished_set_cursor_position_g;
use freminal_terminal_emulator::error::ParserFailures;

#[test]
fn valid_param_normal_number() {
    let mut output = Vec::new();
    let res = ansi_parser_inner_csi_finished_set_cursor_position_g(b"42", &mut output).unwrap();
    assert_eq!(res, Some(ParserInner::Empty));
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPos {
            x: Some(42),
            y: None
        }]
    );
}

#[test]
fn valid_param_zero_treated_as_one() {
    let mut output = Vec::new();
    let res = ansi_parser_inner_csi_finished_set_cursor_position_g(b"0", &mut output).unwrap();
    assert_eq!(res, Some(ParserInner::Empty));
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPos {
            x: Some(1),
            y: None
        }]
    );
}

#[test]
fn valid_param_one_treated_as_one() {
    let mut output = Vec::new();
    let res = ansi_parser_inner_csi_finished_set_cursor_position_g(b"1", &mut output).unwrap();
    assert_eq!(res, Some(ParserInner::Empty));
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPos {
            x: Some(1),
            y: None
        }]
    );
}

#[test]
fn empty_param_defaults_to_one() {
    let mut output = Vec::new();
    let res = ansi_parser_inner_csi_finished_set_cursor_position_g(b"", &mut output).unwrap();
    assert_eq!(res, Some(ParserInner::Empty));
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPos {
            x: Some(1),
            y: None
        }]
    );
}

#[test]
fn invalid_ascii_param_results_in_error_and_invalid_output() {
    let mut output = Vec::new();
    let err = ansi_parser_inner_csi_finished_set_cursor_position_g(b"abc", &mut output)
        .expect_err("Expected an error for invalid param");
    assert_eq!(output, vec![TerminalOutput::Invalid]);

    // ✅ Updated to match real ParserFailures Display
    let msg = err.to_string();
    assert!(
        msg.contains("Invalid cursor (CHA)"),
        "Unexpected error message: {msg}"
    );
}

#[test]
fn invalid_utf8_param_results_in_error_and_invalid_output() {
    let mut output = Vec::new();
    let err = ansi_parser_inner_csi_finished_set_cursor_position_g(&[0xFF], &mut output)
        .expect_err("Expected an error for invalid UTF-8 param");
    assert_eq!(output, vec![TerminalOutput::Invalid]);

    // ✅ Same substring check as above
    let msg = err.to_string();
    assert!(
        msg.contains("Invalid cursor (CHA)"),
        "Unexpected error message: {msg}"
    );
}

#[test]
fn correct_error_type_is_parser_failures() {
    let mut output = Vec::new();
    let err = ansi_parser_inner_csi_finished_set_cursor_position_g(b"x", &mut output).unwrap_err();

    // Downcast to ParserFailures to verify type
    let downcasted = err.downcast_ref::<ParserFailures>();
    assert!(downcasted.is_some(), "Error is not ParserFailures");
}
