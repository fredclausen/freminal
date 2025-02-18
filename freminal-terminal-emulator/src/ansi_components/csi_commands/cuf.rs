// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::ansi::{parse_param_as, ParserInner, TerminalOutput};
use crate::error::ParserFailures;
use anyhow::Result;

/// Cursor Right
///
/// CUF moves the cursor right by a specified number of columns without changing lines.
///
/// ESC [ Pn C
/// # Errors
/// Will return an error if the parameter is not a valid number
pub fn ansi_parser_inner_csi_finished_move_right(
    params: &[u8],
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>> {
    let Ok(param) = parse_param_as::<i32>(params) else {
        warn!("Invalid cursor move right distance");
        output.push(TerminalOutput::Invalid);
        return Err(ParserFailures::UnhandledCUFCommand(
            String::from_utf8_lossy(params).to_string(),
        )
        .into());
    };

    let param = match param {
        Some(0 | 1) | None => 1,
        Some(n) => n,
    };

    output.push(TerminalOutput::SetCursorPosRel {
        x: Some(param),
        y: None,
    });

    Ok(Some(ParserInner::Empty))
}
