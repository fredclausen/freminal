// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::{
    gui::colors::{lookup_256_color_by_index, TerminalColor},
    terminal_emulator::{
        ansi::{split_params_into_semicolon_delimited_usize, ParserInner, TerminalOutput},
        ansi_components::sgr::SelectGraphicRendition,
    },
};

#[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
pub fn ansi_parser_inner_csi_finished_sgr_ansi(
    params: &[u8],
    output: &mut Vec<TerminalOutput>,
) -> Result<Option<ParserInner>, ()> {
    let params = split_params_into_semicolon_delimited_usize(params);

    let Ok(mut params) = params else {
        warn!("Invalid SGR sequence");
        output.push(TerminalOutput::Invalid);

        return Err(());
    };

    if params.is_empty() {
        params.push(Some(0));
    }

    if params.len() == 1 && params[0].is_none() {
        params[0] = Some(0);
    }

    let mut param_iter = params.into_iter();
    loop {
        let param = param_iter.next();
        let Some(mut param) = param.unwrap_or(None) else {
            break;
        };

        // if control code is 38 or 48, we need to read the next param
        // otherwise, store the param as is

        if param == 38 || param == 48 || param == 58 {
            let custom_color_control_code = param;
            let custom_color_r: usize;
            let custom_color_g: usize;
            let custom_color_b: usize;

            param = if let Some(Some(param)) = param_iter.next() {
                param
            } else {
                // FIXME: we'll treat '\e[38m' or '\e[48m' as a color reset.
                // I can't find documentation for this, but it seems that other terminals handle it this way
                warn!(
                    "SGR {} received with no color input. Resetting pallate",
                    param
                );
                output.push(match custom_color_control_code {
                    38 => TerminalOutput::Sgr(SelectGraphicRendition::Foreground(
                        TerminalColor::Default,
                    )),
                    48 => TerminalOutput::Sgr(SelectGraphicRendition::Background(
                        TerminalColor::DefaultBackground,
                    )),
                    58 => TerminalOutput::Sgr(SelectGraphicRendition::UnderlineColor(
                        TerminalColor::DefaultUnderlineColor,
                    )),
                    _ => unreachable!(),
                });
                continue;
            };

            match param {
                2 => {
                    custom_color_r = if let Some(Some(param)) = param_iter.next() {
                        param
                    } else {
                        warn!("Invalid SGR sequence: {}", param);
                        output.push(TerminalOutput::Invalid);
                        continue;
                    };
                    custom_color_g = if let Some(Some(param)) = param_iter.next() {
                        param
                    } else {
                        warn!("Invalid SGR sequence: {}", param);
                        output.push(TerminalOutput::Invalid);
                        continue;
                    };
                    custom_color_b = if let Some(Some(param)) = param_iter.next() {
                        param
                    } else {
                        warn!("Invalid SGR sequence: {}", param);
                        output.push(TerminalOutput::Invalid);
                        continue;
                    };
                }
                5 => {
                    let Some(Some(lookup)) = param_iter.next() else {
                        warn!("Invalid SGR sequence: {}", param);
                        output.push(TerminalOutput::Invalid);
                        continue;
                    };

                    // lets make sure the iterator is empty now. Otherwise, it's an invalid sequence
                    if param_iter.next().is_some() {
                        warn!("Invalid SGR sequence: {}", param);
                        output.push(TerminalOutput::Invalid);
                        continue;
                    }

                    // look up the rgb

                    (custom_color_r, custom_color_g, custom_color_b) =
                        lookup_256_color_by_index(lookup);
                }
                _ => {
                    warn!("Invalid SGR sequence: {}", param);
                    output.push(TerminalOutput::Invalid);
                    continue;
                }
            }

            output.push(TerminalOutput::Sgr(
                SelectGraphicRendition::from_usize_color(
                    custom_color_control_code,
                    custom_color_r,
                    custom_color_g,
                    custom_color_b,
                ),
            ));
            continue;
        }

        output.push(TerminalOutput::Sgr(SelectGraphicRendition::from_usize(
            param,
        )));
    }

    Ok(Some(ParserInner::Empty))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal_emulator::ansi::ParserInner;

    #[test]
    fn test_ansi_parser_inner_csi_finished_sgr_ansi() {
        for i in 0..=107usize {
            let mut output = Vec::new();
            let i_string = i.to_string();
            let params = i_string.as_bytes();
            let result = ansi_parser_inner_csi_finished_sgr_ansi(params, &mut output);
            assert_eq!(result, Ok(Some(ParserInner::Empty)), "Failed for {i}");
            assert_eq!(
                output,
                vec![TerminalOutput::Sgr(SelectGraphicRendition::from_usize(i))],
                "Failed for {i}"
            );
        }

        // now test SGR 38 and 48

        let mut output = Vec::new();
        let params = b"38;2;255;255;255";
        let result = ansi_parser_inner_csi_finished_sgr_ansi(params, &mut output);
        assert_eq!(result, Ok(Some(ParserInner::Empty)));
        assert_eq!(
            output,
            vec![TerminalOutput::Sgr(SelectGraphicRendition::Foreground(
                TerminalColor::Custom(255, 255, 255)
            ))]
        );

        let mut output = Vec::new();
        let params = b"48;5;255";
        let result = ansi_parser_inner_csi_finished_sgr_ansi(params, &mut output);
        assert_eq!(result, Ok(Some(ParserInner::Empty)));
    }
}
