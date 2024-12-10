// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::ansi::{split_params_into_semicolon_delimited_usize, ParserInner, TerminalOutput};
use crate::error::ParserFailures;
use anyhow::Result;

/// Set Top and Bottom Margins
///
/// DECSTBM: This control function sets the top and bottom margins for the current page.
/// You cannot perform scrolling outside the margins.
///
/// Values for param:
/// Pt - Line number for top margin
/// Pb - Line number for bottom margin
///
/// Notes on DECSTBM
/// The value of the top margin (Pt) must be less than the bottom margin (Pb).
/// The maximum size of the scrolling region is the page size.
/// DECSTBM moves the cursor to column 1, line 1 of the page.
///
/// ESC [ Pt ; Pb r
///
/// Internally, we will use `usize::MAX` to flag that the value should be default
/// Default for Pt is 1
/// Default for Pb is the page size
///
/// # Errors
/// Will return an error if the parameter is not a valid number
pub fn ansi_parser_inner_csi_set_top_and_bottom_margins(
    params: &[u8],
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>> {
    if params.is_empty() {
        debug!("DECSTBM command with no params. Using defaults");
        output.push(TerminalOutput::SetTopAndBottomMargins {
            top_margin: 0,
            bottom_margin: usize::MAX,
        });

        return Ok(Some(ParserInner::Empty));
    }

    let Ok(param) = split_params_into_semicolon_delimited_usize(params) else {
        warn!("Invalid DECSTBM command");
        output.push(TerminalOutput::Invalid);

        return Err(ParserFailures::UnhandledDECSTBMCommand(format!(
            "Failed to parse in to {params:?}"
        ))
        .into());
    };

    if param.len() != 2 {
        return Err(ParserFailures::UnhandledDECSTBMCommand(format!("{param:?}")).into());
    }

    let pt = param[0].unwrap_or(1);
    let pb = param[1].unwrap_or(usize::MAX);

    if pt >= pb || pt == 0 || pb == 0 {
        return Err(ParserFailures::UnhandledDECSTBMCommand(format!("{param:?}")).into());
    }

    output.push(TerminalOutput::SetTopAndBottomMargins {
        top_margin: pt,
        bottom_margin: pb,
    });

    Ok(Some(ParserInner::Empty))
}
