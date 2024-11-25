// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::ansi::{
    extract_param, split_params_into_semicolon_delimited_usize, ParserInner, TerminalOutput,
};
use crate::error::ParserFailures;
use anyhow::Result;

/// Cursor Position
///
/// CUP moves the cursor to the specified position. If the cursor is already at the specified position, no action occurs.
///
/// ESC [ Pn ; Pn H
/// # Errors
/// Will return an error if the parameter is not a valid number

pub fn ansi_parser_inner_csi_finished_set_position_h(
    params: &[u8],
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>> {
    let params_parsed = split_params_into_semicolon_delimited_usize(params);

    let Ok(params) = params_parsed else {
        warn!("Invalid cursor set position sequence");
        output.push(TerminalOutput::Invalid);
        return Err(ParserFailures::UnhandledCUPCommand(params.to_vec()).into());
    };

    output.push(TerminalOutput::SetCursorPos {
        x: Some(extract_param(1, &params).unwrap_or(1)),
        y: Some(extract_param(0, &params).unwrap_or(1)),
    });

    Ok(Some(ParserInner::Empty))
}
