// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::terminal_emulator::ansi::{
    extract_param, split_params_into_semicolon_delimited_usize, ParserInner, TerminalOutput,
};

/// Cursor Position
///
/// CUP moves the cursor to the specified position. If the cursor is already at the specified position, no action occurs.
///
/// ESC [ Pn ; Pn H

pub fn ansi_parser_inner_csi_finished_set_position_h(
    params: &[u8],
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>, ()> {
    let params = split_params_into_semicolon_delimited_usize(params);

    let Ok(params) = params else {
        warn!("Invalid cursor set position sequence");
        output.push(TerminalOutput::Invalid);
        return Err(());
    };

    output.push(TerminalOutput::SetCursorPos {
        x: Some(extract_param(1, &params).unwrap_or(1)),
        y: Some(extract_param(0, &params).unwrap_or(1)),
    });

    Ok(Some(ParserInner::Empty))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal_emulator::ansi::TerminalOutput;

    #[test]
    fn test_cup() {
        let mut output = Vec::new();
        ansi_parser_inner_csi_finished_set_position_h(b"1;1", &mut output).unwrap();
        assert_eq!(
            output,
            vec![TerminalOutput::SetCursorPos {
                x: Some(1),
                y: Some(1)
            }]
        );

        let mut output = Vec::new();
        ansi_parser_inner_csi_finished_set_position_h(b"1;", &mut output).unwrap();
        assert_eq!(
            output,
            vec![TerminalOutput::SetCursorPos {
                x: Some(1),
                y: Some(1)
            }]
        );

        let mut output = Vec::new();
        ansi_parser_inner_csi_finished_set_position_h(b";1", &mut output).unwrap();
        assert_eq!(
            output,
            vec![TerminalOutput::SetCursorPos {
                x: Some(1),
                y: Some(1)
            }]
        );

        let mut output = Vec::new();
        ansi_parser_inner_csi_finished_set_position_h(b"", &mut output).unwrap();
        assert_eq!(
            output,
            vec![TerminalOutput::SetCursorPos {
                x: Some(1),
                y: Some(1)
            }]
        );
    }
}