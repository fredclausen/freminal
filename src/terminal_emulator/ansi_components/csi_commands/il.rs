// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::terminal_emulator::ansi::{parse_param_as, ParserInner, TerminalOutput};

/// Insert Lines
///
/// IL inserts a specified number of lines at the cursor position.
///
/// ESC [ Pn L
pub fn ansi_parser_inner_csi_finished_set_position_l(
    params: &[u8],
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>, ()> {
    let Ok(param) = parse_param_as::<usize>(params) else {
        warn!("Invalid il command");
        output.push(TerminalOutput::Invalid);

        return Err(());
    };

    output.push(TerminalOutput::InsertLines(param.unwrap_or(1)));

    Ok(Some(ParserInner::Empty))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal_emulator::ansi::TerminalOutput;

    #[test]
    fn test_el() {
        let mut output = Vec::new();
        ansi_parser_inner_csi_finished_set_position_l(&[], &mut output).unwrap();
        assert_eq!(output, vec![TerminalOutput::InsertLines(1)]);

        let mut output = Vec::new();
        ansi_parser_inner_csi_finished_set_position_l(b"1", &mut output).unwrap();
        assert_eq!(output, vec![TerminalOutput::InsertLines(1)]);

        let mut output = Vec::new();
        ansi_parser_inner_csi_finished_set_position_l(b"2", &mut output).unwrap();
        assert_eq!(output, vec![TerminalOutput::InsertLines(2)]);
    }
}
