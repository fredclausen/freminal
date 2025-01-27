// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::ansi::{parse_param_as, ParserInner, TerminalOutput};
use crate::error::ParserFailures;
use anyhow::Result;

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
/// # Errors
/// Will return an error if the parameter is not a valid number
pub fn ansi_parser_inner_csi_finished_set_position_j(
    params: &[u8],
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>> {
    let Ok(param) = parse_param_as::<usize>(params) else {
        warn!("Invalid clear command");
        output.push(TerminalOutput::Invalid);

        return Err(ParserFailures::UnhandledEDCommand(format!("{params:?}")).into());
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
