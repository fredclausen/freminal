// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::terminal_emulator::ansi::{parse_param_as, ParserInner, TerminalOutput};

/// Move cursor to indicated column in current row
///
/// CHA moves the cursor to the specified column in the current row. If the cursor is already at the specified position, no action occurs.
///
/// ESC [ Pn G

pub fn ansi_parser_inner_csi_finished_set_position_g(
    params: &[u8],
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>, ()> {
    let Ok(param) = parse_param_as::<usize>(params) else {
        warn!("Invalid cursor set position sequence");
        output.push(TerminalOutput::Invalid);
        return Err(());
    };

    let x_pos = param.unwrap_or(1);

    output.push(TerminalOutput::SetCursorPos {
        x: Some(x_pos),
        y: None,
    });

    Ok(Some(ParserInner::Empty))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal_emulator::ansi::TerminalOutput;

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
}
