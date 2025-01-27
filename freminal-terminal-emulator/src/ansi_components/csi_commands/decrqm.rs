// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::ansi::{ParserInner, TerminalOutput};
use crate::ansi_components::mode::{terminal_mode_from_params, SetMode};
use crate::error::ParserFailures;
use anyhow::Result;

/// DEC Private Mode Set
///
/// Supported formats:
/// - Set ESC [ ? Pn h
/// - Reset ESC [ ? Pn l
/// - Query ESC [ ? Pn $ h
/// # Errors
/// Will return an error if the parameter is not a valid number
pub fn ansi_parser_inner_csi_finished_decrqm(
    params: &[u8],
    intermediates: &[u8],
    terminator: u8,
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>> {
    // if intermediates contains '$' then we are querying
    if intermediates.contains(&b'$') {
        output.push(TerminalOutput::Mode(terminal_mode_from_params(
            params,
            &SetMode::DecQuery,
        )));
    } else if terminator == b'h' {
        output.push(TerminalOutput::Mode(terminal_mode_from_params(
            params,
            &SetMode::DecSet,
        )));
    } else if terminator == b'l' {
        output.push(TerminalOutput::Mode(terminal_mode_from_params(
            params,
            &SetMode::DecRst,
        )));
    } else {
        warn!("Invalid DEC Private Mode Set sequence");
        output.push(TerminalOutput::Invalid);
        return Err(ParserFailures::UnhandledDECRQMCommand(params.to_vec()).into());
    }

    Ok(Some(ParserInner::Empty))
}
