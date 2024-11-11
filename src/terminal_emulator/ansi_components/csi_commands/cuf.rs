// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::terminal_emulator::ansi::{parse_param_as, ParserInner, TerminalOutput};

/// Cursor Right
///
/// CUF moves the cursor right by a specified number of columns without changing lines.
///
/// ESC [ Pn C
pub fn ansi_parser_inner_csi_finished_move_right(
    params: &[u8],
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>, ()> {
    let Ok(param) = parse_param_as::<i32>(params) else {
        warn!("Invalid cursor move right distance");
        output.push(TerminalOutput::Invalid);
        return Err(());
    };

    output.push(TerminalOutput::SetCursorPosRel {
        x: Some(param.unwrap_or(1)),
        y: None,
    });

    Ok(Some(ParserInner::Empty))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal_emulator::ansi::TerminalOutput;

    #[test]
    fn test_cuf() {
        let mut output = Vec::new();
        ansi_parser_inner_csi_finished_move_right(&[], &mut output).unwrap();
        assert_eq!(
            output,
            vec![TerminalOutput::SetCursorPosRel {
                x: Some(1),
                y: None
            }]
        );

        let mut output = Vec::new();
        ansi_parser_inner_csi_finished_move_right(b"1", &mut output).unwrap();
        assert_eq!(
            output,
            vec![TerminalOutput::SetCursorPosRel {
                x: Some(1),
                y: None
            }]
        );

        let mut output = Vec::new();
        ansi_parser_inner_csi_finished_move_right(b"2", &mut output).unwrap();
        assert_eq!(
            output,
            vec![TerminalOutput::SetCursorPosRel {
                x: Some(2),
                y: None
            }]
        );
    }
}
