// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use freminal::terminal_emulator::{
    ansi::TerminalOutput,
    ansi_components::csi_commands::cha::ansi_parser_inner_csi_finished_set_position_g,
};

#[test]
fn test_cha() {
    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_set_position_g(b"1", &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPos {
            x: Some(1),
            y: None
        }]
    );

    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_set_position_g(b"2", &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPos {
            x: Some(2),
            y: None
        }]
    );

    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_set_position_g(b"3", &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPos {
            x: Some(3),
            y: None
        }]
    );
}
