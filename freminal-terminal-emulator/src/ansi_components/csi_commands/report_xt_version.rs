// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::ansi::{parse_param_as, ParserInner, TerminalOutput};
use crate::error::ParserFailures;
use anyhow::Result;

/// Request device name and version
///
/// ESC [ > Ps q
///
/// # Errors
/// Will return an error if the parameter is not a valid number
pub fn ansi_parser_inner_csi_finished_report_version_q(
    params: &[u8],
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>> {
    let Ok(param) = parse_param_as::<usize>(&[*params.get(1).unwrap_or(&b'0')]) else {
        output.push(TerminalOutput::Invalid);
        return Err(ParserFailures::UnhandledXTVERSIONCommand(
            String::from_utf8_lossy(params).to_string(),
        )
        .into());
    };

    let request = param.unwrap_or(0);

    if request == 0 {
        output.push(TerminalOutput::RequestDeviceNameandVersion);
    } else {
        output.push(TerminalOutput::Invalid);
        return Err(ParserFailures::UnhandledXTVERSIONCommand(
            String::from_utf8_lossy(params).to_string(),
        )
        .into());
    }

    Ok(Some(ParserInner::Empty))
}
