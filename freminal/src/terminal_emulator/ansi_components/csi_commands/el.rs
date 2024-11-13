// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::terminal_emulator::ansi::{parse_param_as, ParserInner, TerminalOutput};
use crate::terminal_emulator::error::ParserFailures;
use anyhow::Result;

/// Erase in Line
///
/// EL clears part or all of the line.
///
/// Values for param:
/// 0 - Erase from the cursor to the end of the line (default)
/// 1 - Erase from the cursor to the start of the line to cursor
/// 2 - Erase the whole line
///
/// ESC [ Pn K
///
/// # Errors
/// Will return an error if the parameter is not a valid number
pub fn ansi_parser_inner_csi_finished_set_position_k(
    params: &[u8],
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>> {
    let Ok(param) = parse_param_as::<usize>(params) else {
        warn!("Invalid erase in line command");
        output.push(TerminalOutput::Invalid);

        return Err(ParserFailures::UnhandledELCommand(format!("{params:?}")).into());
    };

    // ECMA-48 8.3.39
    match param.unwrap_or(0) {
        0 => output.push(TerminalOutput::ClearLineForwards),
        1 => output.push(TerminalOutput::ClearLineBackwards),
        2 => output.push(TerminalOutput::ClearLine),
        v => {
            warn!("Unsupported erase in line command ({v})");
            output.push(TerminalOutput::Invalid);
        }
    }

    Ok(Some(ParserInner::Empty))
}
