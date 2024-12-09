// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::ansi::{parse_param_as, ParserInner, TerminalOutput};
use crate::error::ParserFailures;
use anyhow::Result;

/// DECSCUSR—Set Cursor Style
///
/// Select the style of the cursor on the screen.
/// 0, 1, or none: Blink Block (default)
/// 2: Steady Block
/// 3: Blink Underline
/// 4: Steady Underline
/// 5: Vertical line cursor / Blink
/// 6: Vertical line cursor / Steady
///
/// ESC [ Pn SP q
/// # Errors
/// Will return an error if the parameter is not a valid number
pub fn ansi_parser_inner_csi_finished_set_position_q(
    params: &[u8],
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>> {
    let Ok(param) = parse_param_as::<usize>(params) else {
        warn!("Invalid decscusr command");
        output.push(TerminalOutput::Invalid);

        return Err(ParserFailures::UnhandledDECSCUSRCommand(format!("{params:?}")).into());
    };

    output.push(TerminalOutput::CursorVisualStyle(
        param.unwrap_or_default().into(),
    ));

    Ok(Some(ParserInner::Empty))
}
