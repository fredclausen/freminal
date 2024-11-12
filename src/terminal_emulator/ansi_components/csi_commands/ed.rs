// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::terminal_emulator::ansi::{parse_param_as, ParserInner, TerminalOutput};

/// Erase in Display
///
/// ED clears part of the screen.
///
/// Values for param:
/// 0 - Erase from the cursor to the end of the screen (default)
/// 1 - Erase from the beginning of the screen to the cursor
/// 2 - Erase the entire screen
/// 3 - Erase the entire screen including the scrollback buffer
///
/// ESC [ Pn J

pub fn ansi_parser_inner_csi_finished_set_position_j(
    params: &[u8],
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>, ()> {
    let Ok(param) = parse_param_as::<usize>(params) else {
        warn!("Invalid clear command");
        output.push(TerminalOutput::Invalid);

        return Err(());
    };

    let ret = match param.unwrap_or(0) {
        0 => TerminalOutput::ClearDisplayfromCursortoEndofDisplay,
        1 => TerminalOutput::ClearDiplayfromStartofDisplaytoCursor,
        2 => TerminalOutput::ClearDisplay,
        3 => TerminalOutput::ClearScrollbackandDisplay,
        _ => TerminalOutput::Invalid,
    };
    output.push(ret);

    Ok(Some(ParserInner::Empty))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal_emulator::ansi::TerminalOutput;

    #[test]
    fn test_ed() {
        let mut output = Vec::new();
        ansi_parser_inner_csi_finished_set_position_j(&[], &mut output).unwrap();
        assert_eq!(
            output,
            vec![TerminalOutput::ClearDisplayfromCursortoEndofDisplay]
        );

        let mut output = Vec::new();
        ansi_parser_inner_csi_finished_set_position_j(b"1", &mut output).unwrap();
        assert_eq!(
            output,
            vec![TerminalOutput::ClearDiplayfromStartofDisplaytoCursor]
        );

        let mut output = Vec::new();
        ansi_parser_inner_csi_finished_set_position_j(b"2", &mut output).unwrap();
        assert_eq!(output, vec![TerminalOutput::ClearDisplay]);

        let mut output = Vec::new();
        ansi_parser_inner_csi_finished_set_position_j(b"3", &mut output).unwrap();
        assert_eq!(output, vec![TerminalOutput::ClearScrollbackandDisplay]);

        let mut output = Vec::new();
        ansi_parser_inner_csi_finished_set_position_j(b"4", &mut output).unwrap();
        assert_eq!(output, vec![TerminalOutput::Invalid]);
    }
}
