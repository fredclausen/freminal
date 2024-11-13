// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::ansi::{parse_param_as, ParserInner, TerminalOutput};
use crate::error::ParserFailures;
use anyhow::Result;
/// Insert Lines
///
/// IL inserts a specified number of lines at the cursor position.
///
/// ESC [ Pn L
/// # Errors
/// Will return an error if the parameter is not a valid number

pub fn ansi_parser_inner_csi_finished_set_position_l(
    params: &[u8],
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>> {
    let Ok(param) = parse_param_as::<usize>(params) else {
        warn!("Invalid il command");
        output.push(TerminalOutput::Invalid);

        return Err(ParserFailures::UnhandledILCommand(format!("{params:?}")).into());
    };

    output.push(TerminalOutput::InsertLines(param.unwrap_or(1)));

    Ok(Some(ParserInner::Empty))
}
