// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::ansi::{parse_param_as, ParserInner, TerminalOutput};
use crate::error::ParserFailures;
use anyhow::Result;

/// Insert Blank Character(s)
///
/// ICH inserts blank characters at the cursor position.
///
/// Values for param:
/// 0 - Insert one blank character (default)
/// n - Insert n blank characters
///
/// ESC [ Pn @
/// # Errors
/// Will return an error if the parameter is not a valid number
pub fn ansi_parser_inner_csi_finished_ich(
    params: &[u8],
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>> {
    let Ok(param) = parse_param_as::<usize>(params) else {
        warn!("Invalid ich command");
        output.push(TerminalOutput::Invalid);

        return Err(ParserFailures::UnhandledICHCommand(format!("{params:?}")).into());
    };

    // ecma-48 8.3.64
    output.push(TerminalOutput::InsertSpaces(param.unwrap_or(1)));

    Ok(Some(ParserInner::Empty))
}
