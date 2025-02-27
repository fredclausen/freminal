use test_log::test;
//use tracing::info;

// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.
#[cfg(test)]
use freminal_terminal_emulator::ansi_components::standard::StandardParser;

#[test]
fn test_parser_default() {
    let parser = StandardParser::default();
    assert_eq!(parser, StandardParser::new());
}

#[test]
fn test_contains_string_terminator() {
    let mut parser = StandardParser::default();
    let mut output = Vec::new();
    assert!(!parser.contains_string_terminator());

    let sequence = b"Ptest\x1b\\";
    for byte in sequence.iter() {
        parser.standard_parser_inner(*byte, &mut output).unwrap();
    }

    assert!(parser.contains_string_terminator());
}

#[test]
fn test_err_push_finished() {
    let mut parser = StandardParser::default();
    let mut output = Vec::new();

    let sequence = b"Ptest\x1b\\";
    for byte in sequence.iter() {
        parser.standard_parser_inner(*byte, &mut output).unwrap();
    }

    let result = parser.standard_parser_inner(b'8', &mut output);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "Parsed pushed to once finished"
    );
}
