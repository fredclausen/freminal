// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::terminal_emulator::ansi::{parse_param_as, ParserInner, TerminalOutput};

/// Cursor Backward
///
/// CUU moves the cursor backward by a specified number of lines without changing columns.
///
/// ESC [ Pn D

pub fn ansi_parser_inner_csi_finished_move_left(
    params: &[u8],
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>, ()> {
    let Ok(param) = parse_param_as::<i32>(params) else {
        warn!("Invalid cursor move left distance");
        output.push(TerminalOutput::Invalid);
        return Err(());
    };

    output.push(TerminalOutput::SetCursorPosRel {
        x: Some(-param.unwrap_or(1)),
        y: None,
    });

    Ok(Some(ParserInner::Empty))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal_emulator::ansi::TerminalOutput;

    #[test]
    fn test_cub() {
        let mut output = Vec::new();
        ansi_parser_inner_csi_finished_move_left(&[], &mut output).unwrap();
        assert_eq!(
            output,
            vec![TerminalOutput::SetCursorPosRel {
                x: Some(-1),
                y: None
            }]
        );

        let mut output = Vec::new();
        ansi_parser_inner_csi_finished_move_left(b"1", &mut output).unwrap();
        assert_eq!(
            output,
            vec![TerminalOutput::SetCursorPosRel {
                x: Some(-1),
                y: None
            }]
        );

        let mut output = Vec::new();
        ansi_parser_inner_csi_finished_move_left(b"2", &mut output).unwrap();
        assert_eq!(
            output,
            vec![TerminalOutput::SetCursorPosRel {
                x: Some(-2),
                y: None
            }]
        );
    }
}
