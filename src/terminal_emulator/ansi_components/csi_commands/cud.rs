// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::terminal_emulator::ansi::{parse_param_as, ParserInner, TerminalOutput};

/// Cursor Down
///
/// CUD moves the cursor down by a specified number of lines without changing columns.
///
/// ESC [ Pn B

pub fn ansi_parser_inner_csi_finished_move_down(
    params: &[u8],
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>, ()> {
    let Ok(param) = parse_param_as::<i32>(params) else {
        warn!("Invalid cursor move down distance");
        output.push(TerminalOutput::Invalid);
        return Err(());
    };

    output.push(TerminalOutput::SetCursorPosRel {
        x: None,
        y: Some(param.unwrap_or(1)),
    });

    Ok(Some(ParserInner::Empty))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal_emulator::ansi::TerminalOutput;

    #[test]
    fn test_cud() {
        let mut output = Vec::new();
        ansi_parser_inner_csi_finished_move_down(&[], &mut output).unwrap();
        assert_eq!(
            output,
            vec![TerminalOutput::SetCursorPosRel {
                x: None,
                y: Some(1)
            }]
        );

        let mut output = Vec::new();
        ansi_parser_inner_csi_finished_move_down(b"1", &mut output).unwrap();
        assert_eq!(
            output,
            vec![TerminalOutput::SetCursorPosRel {
                x: None,
                y: Some(1)
            }]
        );

        let mut output = Vec::new();
        ansi_parser_inner_csi_finished_move_down(b"2", &mut output).unwrap();
        assert_eq!(
            output,
            vec![TerminalOutput::SetCursorPosRel {
                x: None,
                y: Some(2)
            }]
        );
    }
}