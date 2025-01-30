// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::ansi::{parse_param_as, ParserInner, TerminalOutput};
use crate::error::ParserFailures;
use anyhow::Result;

/// Request Device Attributes
///
/// Supported formats:
/// - Set ESC [ Ps c
///
/// # Errors
/// Will return an error if the parameter is not a valid number
pub fn ansi_parser_inner_csi_finished_send_da(
    params: &[u8],
    intermediates: &[u8],
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>> {
    // ensure intermediates are empty
    if !intermediates.is_empty() {
        warn!("Invalid send device attributes command");
        output.push(TerminalOutput::Invalid);

        return Err(ParserFailures::UnhandledDACommand(format!(
            "Invalid intermediates for Send DA: {params:?}"
        ))
        .into());
    }

    let Ok(param) = parse_param_as::<usize>(params) else {
        output.push(TerminalOutput::Invalid);
        return Err(ParserFailures::UnhandledDACommand(
            String::from_utf8_lossy(params).to_string(),
        )
        .into());
    };

    let param = param.unwrap_or(0);

    if param != 0 {
        warn!("Invalid send device attributes command");
        output.push(TerminalOutput::Invalid);

        return Err(ParserFailures::UnhandledDACommand(format!(
            "Invalid parameters for Send DA: {params:?}"
        ))
        .into());
    }

    output.push(TerminalOutput::RequestDeviceAttributes);
    Ok(Some(ParserInner::Empty))
}
