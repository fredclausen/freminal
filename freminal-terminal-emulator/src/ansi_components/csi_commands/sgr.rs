// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::vec::IntoIter;

use crate::ansi::split_params_into_colon_delimited_usize;
use crate::error::ParserFailures;
use crate::{
    ansi::{split_params_into_semicolon_delimited_usize, ParserInner, TerminalOutput},
    ansi_components::sgr::SelectGraphicRendition,
};
use anyhow::Result;
use freminal_common::colors::{lookup_256_color_by_index, TerminalColor};

/// Select Graphic Rendition
///
/// SGR sets the text attributes for the following characters. Several attributes can be combined by separating them with a semicolon.
///
/// Values for param are defined in the `SelectGraphicRendition` enum
///
/// ESC [ params m
/// # Errors
/// Will return an error if the parameter is not a valid number
pub fn ansi_parser_inner_csi_finished_sgr_ansi(
    params: &[u8],
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>> {
    let (params, split_by_colon) = if params.contains(&b':') {
        (split_params_into_colon_delimited_usize(params), true)
    } else {
        (split_params_into_semicolon_delimited_usize(params), false)
    };

    let Ok(mut params) = params else {
        warn!("Invalid SGR sequence");
        output.push(TerminalOutput::Invalid);

        return Err(ParserFailures::UnhandledSGRCommand(format!("{params:?}")).into());
    };

    if params.is_empty() {
        params.push(Some(0));
    }

    if params.len() == 1 && params[0].is_none() {
        params[0] = Some(0);
    }

    let mut param_iter: IntoIter<Option<usize>> = params.into_iter();
    loop {
        let param = param_iter.next();
        let Some(param) = param.unwrap_or(None) else {
            break;
        };

        if param == 38 || param == 48 || param == 58 {
            handle_custom_color(output, &mut param_iter, param, split_by_colon);
            continue;
        }

        output.push(TerminalOutput::Sgr(SelectGraphicRendition::from_usize(
            param,
        )));
    }

    Ok(Some(ParserInner::Empty))
}

fn default_color(output: &mut Vec<TerminalOutput>, param: usize, custom_color_control_code: usize) {
    // FIXME: we'll treat '\e[38m' or '\e[48m' as a color reset.
    // I can't find documentation for this, but it seems that other terminals handle it this way
    debug!(
        "SGR {} received with no color input. Resetting pallate",
        param
    );
    output.push(match custom_color_control_code {
        38 => TerminalOutput::Sgr(SelectGraphicRendition::Foreground(TerminalColor::Default)),
        48 => TerminalOutput::Sgr(SelectGraphicRendition::Background(
            TerminalColor::DefaultBackground,
        )),
        // instead of matching directly on 58, we'll match on a wildcard. This helps with codecov because it thought
        // we were testing `_` in the match statement when it's impossible to end up here with a value other than 58
        _ => TerminalOutput::Sgr(SelectGraphicRendition::UnderlineColor(
            TerminalColor::DefaultUnderlineColor,
        )),
    });
}

fn handle_custom_color(
    output: &mut Vec<TerminalOutput>,
    param_iter: &mut IntoIter<Option<usize>>,
    param: usize,
    split_by_colon: bool,
) {
    // if control code is 38, 48 or 58 we need to read the next param
    // otherwise, store the param as is

    let mut param = param;
    let custom_color_control_code = param;
    let custom_color_r: usize;
    let custom_color_g: usize;
    let custom_color_b: usize;

    param = if let Some(Some(param)) = param_iter.next() {
        param
    } else {
        default_color(output, param, custom_color_control_code);
        return;
    };

    match param {
        2 => {
            if param_iter.len() > 3 && split_by_colon {
                debug!(
                    "Found custom color with color space. Ignoring Color Space: {:?}",
                    param_iter
                );
                param_iter.next();
            }

            custom_color_r = if let Some(Some(param)) = param_iter.next() {
                param
            } else {
                0
            };
            custom_color_g = if let Some(Some(param)) = param_iter.next() {
                param
            } else {
                0
            };
            custom_color_b = if let Some(Some(param)) = param_iter.next() {
                param
            } else {
                0
            };
        }
        5 => {
            let lookup = match param_iter.next() {
                Some(Some(lookup)) => lookup,
                _ => 0,
            };

            // look up the rgb

            (custom_color_r, custom_color_g, custom_color_b) = lookup_256_color_by_index(lookup);
        }
        _ => {
            warn!("Invalid SGR sequence: {}", param);
            output.push(TerminalOutput::Invalid);
            return;
        }
    }

    match SelectGraphicRendition::from_usize_color(
        custom_color_control_code,
        custom_color_r,
        custom_color_g,
        custom_color_b,
    ) {
        Ok(sgr) => output.push(TerminalOutput::Sgr(sgr)),
        Err(e) => {
            warn!("Invalid SGR sequence: {}", e);
            output.push(TerminalOutput::Invalid);
        }
    }
}
