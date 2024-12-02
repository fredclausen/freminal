// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::ansi::{parse_param_as, ParserInner, TerminalOutput};
use crate::error::ParserFailures;
use anyhow::Result;

/// Cursor Backward
///
/// CUB moves the cursor backward by a specified number of lines without changing columns.
///
/// ESC [ Pn D
/// # Errors
/// Will return an error if the parameter is not a valid number
pub fn ansi_parser_inner_csi_finished_move_left(
    params: &[u8],
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>> {
    let Ok(param) = parse_param_as::<i32>(params) else {
        warn!("Invalid cursor move left distance");
        output.push(TerminalOutput::Invalid);
        return Err(ParserFailures::UnhandledCUBCommand(
            String::from_utf8_lossy(params).to_string(),
        )
        .into());
    };

    output.push(TerminalOutput::SetCursorPosRel {
        x: Some(-param.unwrap_or(1)),
        y: None,
    });

    Ok(Some(ParserInner::Empty))
}
