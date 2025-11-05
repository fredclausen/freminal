// Copyright (C) 2024-2025 Fred Clausen
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
#[inline]
fn param_or(params: &[Option<usize>], idx: usize, default: usize) -> usize {
    params.get(idx).and_then(|opt| *opt).unwrap_or(default)
}

/// # Errors
/// Will return an error if the parameter is not a valid number
pub fn ansi_parser_inner_csi_set_top_and_bottom_margins(
    params: &[u8],
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>> {
    if params.is_empty() {
        debug!("DECSTBM command with no params. Using defaults");
        output.push(TerminalOutput::SetTopAndBottomMargins {
            top_margin: 1,
            bottom_margin: usize::MAX,
        });

        return Ok(Some(ParserInner::Empty));
    }

    let params = split_params_into_semicolon_delimited_usize(params);

    let Ok(params) = params else {
        warn!("Invalid DECSTBM commandz");
        output.push(TerminalOutput::Invalid);

        return Err(ParserFailures::UnhandledDECSTBMCommand(format!(
            "Failed to parse in to {params:?}"
        ))
        .into());
    };

    if params.len() != 2 {
        warn!("DECSTBM command with invalid number of params. Expected 2, got {params:?}");
        output.push(TerminalOutput::Invalid);
        return Err(ParserFailures::UnhandledDECSTBMCommand(format!("{params:?}")).into());
    }

    let pt = match params.first() {
        Some(Some(0 | 1) | None) | None => 1,
        Some(Some(n)) => *n,
    };

    let pb = param_or(&params, 1, usize::MAX);

    if pt >= pb || pt == 0 || pb == 0 {
        warn!("Invalid DECSTBM command with out of bounds params. pt: {pt}, pb: {pb}");
        output.push(TerminalOutput::Invalid);
        return Err(ParserFailures::UnhandledDECSTBMCommand(format!("{params:?}")).into());
    }

    output.push(TerminalOutput::SetTopAndBottomMargins {
        top_margin: pt,
        bottom_margin: pb,
    });

    Ok(Some(ParserInner::Empty))
}
