// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::terminal_emulator::ansi::{parse_param_as, ParserInner, TerminalOutput};
use crate::terminal_emulator::error::ParserFailures;
use anyhow::Result;

/// Delete Character(s)
///
/// DCH deletes characters from the cursor position to the right.
///
/// Values for param:
/// 0 - Delete one character (default)
/// n - Delete n characters
///
/// ESC [ Pn P
/// # Errors
/// Will return an error if the parameter is not a valid number

pub fn ansi_parser_inner_csi_finished_set_position_p(
    params: &[u8],
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>> {
    let Ok(param) = parse_param_as::<usize>(params) else {
        warn!("Invalid del command");
        output.push(TerminalOutput::Invalid);

        return Err(ParserFailures::UnhandledDCHCommand(format!("{params:?}")).into());
    };

    output.push(TerminalOutput::Delete(param.unwrap_or(1)));

    Ok(Some(ParserInner::Empty))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ansi_parser_inner_csi_finished_set_position_p() {
        let mut output = Vec::new();

        ansi_parser_inner_csi_finished_set_position_p(&[], &mut output).unwrap();
        assert_eq!(output, vec![TerminalOutput::Delete(1)]);

        output.clear();
        ansi_parser_inner_csi_finished_set_position_p(b"0", &mut output).unwrap();
        assert_eq!(output, vec![TerminalOutput::Delete(0)]);

        output.clear();
        ansi_parser_inner_csi_finished_set_position_p(b"1", &mut output).unwrap();
        assert_eq!(output, vec![TerminalOutput::Delete(1)]);

        output.clear();
        ansi_parser_inner_csi_finished_set_position_p(b"2", &mut output).unwrap();
        assert_eq!(output, vec![TerminalOutput::Delete(2)]);
    }
}
