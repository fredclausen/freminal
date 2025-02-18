// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::ansi::{parse_param_as, ParserInner, TerminalOutput};
use crate::error::ParserFailures;
use anyhow::Result;

/// Move cursor to indicated column in current row
///
/// CHA moves the cursor to the specified column in the current row. If the cursor is already at the specified position, no action occurs.
///
/// ESC [ Pn G
///
/// # Errors
/// Will return an error if the parameter is not a valid number
pub fn ansi_parser_inner_csi_finished_set_cursor_position_g(
    params: &[u8],
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>> {
    let Ok(param) = parse_param_as::<usize>(params) else {
        output.push(TerminalOutput::Invalid);
        return Err(ParserFailures::UnhandledCHACommand(
            String::from_utf8_lossy(params).to_string(),
        )
        .into());
    };

    let x_pos = match param {
        Some(0 | 1) | None => 1,
        Some(n) => n,
    };

    output.push(TerminalOutput::SetCursorPos {
        x: Some(x_pos),
        y: None,
    });

    Ok(Some(ParserInner::Empty))
}
