use crate::{
    gui::colors::{lookup_256_color_by_index, TerminalColor},
    terminal_emulator::ansi::{
        extract_param, parse_param_as, split_params_into_semicolon_delimited_usize, ParserInner,
        TerminalOutput,
    },
};

use super::{mode::terminal_mode_from_params, sgr::SelectGraphicRendition};

#[derive(Eq, PartialEq, Debug)]
pub enum AnsiCsiParserState {
    Params,
    Intermediates,
    Finished(u8),
    Invalid,
    InvalidFinished,
}

#[derive(Eq, PartialEq, Debug)]
pub struct AnsiCsiParser {
    pub(crate) state: AnsiCsiParserState,
    pub(crate) params: Vec<u8>,
    pub(crate) intermediates: Vec<u8>,
}

impl AnsiCsiParser {
    pub const fn new() -> Self {
        Self {
            state: AnsiCsiParserState::Params,
            params: Vec::new(),
            intermediates: Vec::new(),
        }
    }

    pub fn push(&mut self, b: u8) {
        if let AnsiCsiParserState::Finished(_) | AnsiCsiParserState::InvalidFinished = &self.state {
            panic!("CsiParser should not be pushed to once finished");
        }

        match &mut self.state {
            AnsiCsiParserState::Params => {
                if is_csi_param(b) {
                    self.params.push(b);
                } else if is_csi_intermediate(b) {
                    self.intermediates.push(b);
                    self.state = AnsiCsiParserState::Intermediates;
                } else if is_csi_terminator(b) {
                    self.state = AnsiCsiParserState::Finished(b);
                } else {
                    self.state = AnsiCsiParserState::Invalid;
                }
            }
            AnsiCsiParserState::Intermediates => {
                if is_csi_param(b) {
                    self.state = AnsiCsiParserState::Invalid;
                } else if is_csi_intermediate(b) {
                    self.intermediates.push(b);
                } else if is_csi_terminator(b) {
                    self.state = AnsiCsiParserState::Finished(b);
                } else {
                    self.state = AnsiCsiParserState::Invalid;
                }
            }
            AnsiCsiParserState::Invalid => {
                if is_csi_terminator(b) {
                    self.state = AnsiCsiParserState::InvalidFinished;
                }
            }
            AnsiCsiParserState::Finished(_) | AnsiCsiParserState::InvalidFinished => {
                unreachable!();
            }
        }
    }

    pub fn ansiparser_inner_csi(
        &mut self,
        b: u8,
        output: &mut Vec<TerminalOutput>,
    ) -> Result<Option<ParserInner>, ()> {
        self.push(b);

        match self.state {
            AnsiCsiParserState::Finished(b'A') => {
                self.ansi_parser_inner_csi_finished_move_up(output)
            }
            AnsiCsiParserState::Finished(b'B') => {
                self.ansi_parser_inner_csi_finished_move_down(output)
            }
            AnsiCsiParserState::Finished(b'C') => {
                self.ansi_parser_inner_csi_finished_move_right(output)
            }
            AnsiCsiParserState::Finished(b'D') => {
                self.ansi_parser_inner_csi_finished_move_left(output)
            }
            AnsiCsiParserState::Finished(b'H') => {
                self.ansi_parser_inner_csi_finished_set_position_h(output)
            }
            AnsiCsiParserState::Finished(b'G') => {
                self.ansi_parser_inner_csi_finished_set_position_g(output)
            }
            AnsiCsiParserState::Finished(b'J') => {
                self.ansi_parser_inner_csi_finished_set_position_j(output)
            }
            AnsiCsiParserState::Finished(b'K') => {
                self.ansi_parser_inner_csi_finished_set_position_k(output)
            }
            AnsiCsiParserState::Finished(b'L') => {
                self.ansi_parser_inner_csi_finished_set_position_l(output)
            }
            AnsiCsiParserState::Finished(b'P') => {
                self.ansi_parser_inner_csi_finished_set_position_p(output)
            }
            AnsiCsiParserState::Finished(b'm') => self.ansi_parser_inner_csi_finished_sgr(output),
            AnsiCsiParserState::Finished(b'h') => {
                output.push(TerminalOutput::SetMode(terminal_mode_from_params(
                    &self.params,
                )));
                Ok(Some(ParserInner::Empty))
            }
            AnsiCsiParserState::Finished(b'l') => {
                output.push(TerminalOutput::ResetMode(terminal_mode_from_params(
                    &self.params,
                )));
                Ok(Some(ParserInner::Empty))
            }
            AnsiCsiParserState::Finished(b'@') => self.ansi_parser_inner_csi_finished_ich(output),
            AnsiCsiParserState::Finished(b'n') => {
                output.push(TerminalOutput::CursorReport);
                Ok(Some(ParserInner::Empty))
            }
            AnsiCsiParserState::Finished(esc) => {
                warn!(
                    "Unhandled csi code: {:?} {esc:x} {}/{}",
                    std::char::from_u32(u32::from(esc)),
                    esc >> 4,
                    esc & 0xf,
                );
                output.push(TerminalOutput::Invalid);

                Ok(Some(ParserInner::Empty))
            }
            AnsiCsiParserState::Invalid => {
                warn!("Invalid CSI sequence");
                output.push(TerminalOutput::Invalid);

                Ok(Some(ParserInner::Empty))
            }
            _ => Ok(None),
        }
    }

    fn ansi_parser_inner_csi_finished_move_up(
        &self,
        output: &mut Vec<TerminalOutput>,
    ) -> Result<Option<ParserInner>, ()> {
        let Ok(param) = parse_param_as::<i32>(&self.params) else {
            warn!("Invalid cursor move up distance");
            output.push(TerminalOutput::Invalid);
            return Err(());
        };

        output.push(TerminalOutput::SetCursorPosRel {
            x: None,
            y: Some(-param.unwrap_or(1)),
        });

        Ok(Some(ParserInner::Empty))
    }

    fn ansi_parser_inner_csi_finished_move_down(
        &self,
        output: &mut Vec<TerminalOutput>,
    ) -> Result<Option<ParserInner>, ()> {
        let Ok(param) = parse_param_as::<i32>(&self.params) else {
            warn!("Invalid cursor move down distance");
            output.push(TerminalOutput::Invalid);
            return Err(());
        };

        output.push(TerminalOutput::SetCursorPosRel {
            x: None,
            y: Some(param.unwrap_or(1)),
        });

        Ok(Some(ParserInner::Empty))
    }

    fn ansi_parser_inner_csi_finished_move_right(
        &self,
        output: &mut Vec<TerminalOutput>,
    ) -> Result<Option<ParserInner>, ()> {
        let Ok(param) = parse_param_as::<i32>(&self.params) else {
            warn!("Invalid cursor move right distance");
            output.push(TerminalOutput::Invalid);
            return Err(());
        };

        output.push(TerminalOutput::SetCursorPosRel {
            x: Some(param.unwrap_or(1)),
            y: None,
        });

        Ok(Some(ParserInner::Empty))
    }

    fn ansi_parser_inner_csi_finished_move_left(
        &self,
        output: &mut Vec<TerminalOutput>,
    ) -> Result<Option<ParserInner>, ()> {
        let Ok(param) = parse_param_as::<i32>(&self.params) else {
            warn!("Invalid cursor move left distance");
            output.push(TerminalOutput::Invalid);
            return Err(());
        };

        output.push(TerminalOutput::SetCursorPosRel {
            x: Some(-param.unwrap_or(1)),
            y: None,
        });

        Ok(Some(ParserInner::Empty))
    }

    fn ansi_parser_inner_csi_finished_set_position_h(
        &self,
        output: &mut Vec<TerminalOutput>,
    ) -> Result<Option<ParserInner>, ()> {
        let params = split_params_into_semicolon_delimited_usize(&self.params);

        let Ok(params) = params else {
            warn!("Invalid cursor set position sequence");
            output.push(TerminalOutput::Invalid);
            return Err(());
        };

        output.push(TerminalOutput::SetCursorPos {
            x: Some(extract_param(1, &params).unwrap_or(1)),
            y: Some(extract_param(0, &params).unwrap_or(1)),
        });

        Ok(Some(ParserInner::Empty))
    }

    fn ansi_parser_inner_csi_finished_set_position_g(
        &self,
        output: &mut Vec<TerminalOutput>,
    ) -> Result<Option<ParserInner>, ()> {
        let Ok(param) = parse_param_as::<usize>(&self.params) else {
            warn!("Invalid cursor set position sequence");
            output.push(TerminalOutput::Invalid);
            return Err(());
        };

        let x_pos = param.unwrap_or(1);

        output.push(TerminalOutput::SetCursorPos {
            x: Some(x_pos),
            y: None,
        });

        Ok(Some(ParserInner::Empty))
    }

    fn ansi_parser_inner_csi_finished_set_position_j(
        &self,
        output: &mut Vec<TerminalOutput>,
    ) -> Result<Option<ParserInner>, ()> {
        let Ok(param) = parse_param_as::<usize>(&self.params) else {
            warn!("Invalid clear command");
            output.push(TerminalOutput::Invalid);

            return Err(());
        };

        let ret = match param.unwrap_or(0) {
            0 => TerminalOutput::ClearForwards,
            2 | 3 => TerminalOutput::ClearAll,
            _ => TerminalOutput::Invalid,
        };
        output.push(ret);

        Ok(Some(ParserInner::Empty))
    }

    fn ansi_parser_inner_csi_finished_set_position_k(
        &self,
        output: &mut Vec<TerminalOutput>,
    ) -> Result<Option<ParserInner>, ()> {
        let Ok(param) = parse_param_as::<usize>(&self.params) else {
            warn!("Invalid erase in line command");
            output.push(TerminalOutput::Invalid);

            return Err(());
        };

        // ECMA-48 8.3.39
        match param.unwrap_or(0) {
            0 => output.push(TerminalOutput::ClearLineForwards),
            v => {
                warn!("Unsupported erase in line command ({v})");
                output.push(TerminalOutput::Invalid);
            }
        }

        Ok(Some(ParserInner::Empty))
    }

    fn ansi_parser_inner_csi_finished_set_position_l(
        &self,
        output: &mut Vec<TerminalOutput>,
    ) -> Result<Option<ParserInner>, ()> {
        let Ok(param) = parse_param_as::<usize>(&self.params) else {
            warn!("Invalid il command");
            output.push(TerminalOutput::Invalid);

            return Err(());
        };

        output.push(TerminalOutput::InsertLines(param.unwrap_or(1)));

        Ok(Some(ParserInner::Empty))
    }

    fn ansi_parser_inner_csi_finished_set_position_p(
        &self,
        output: &mut Vec<TerminalOutput>,
    ) -> Result<Option<ParserInner>, ()> {
        let Ok(param) = parse_param_as::<usize>(&self.params) else {
            warn!("Invalid del command");
            output.push(TerminalOutput::Invalid);

            return Err(());
        };

        output.push(TerminalOutput::Delete(param.unwrap_or(1)));

        Ok(Some(ParserInner::Empty))
    }

    #[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
    fn ansi_parser_inner_csi_finished_sgr(
        &self,
        output: &mut Vec<TerminalOutput>,
    ) -> Result<Option<ParserInner>, ()> {
        let params = split_params_into_semicolon_delimited_usize(&self.params);

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
                    output.push(if custom_color_control_code == 38 {
                        TerminalOutput::Sgr(SelectGraphicRendition::Foreground(
                            TerminalColor::Default,
                        ))
                    } else if custom_color_control_code == 48 {
                        TerminalOutput::Sgr(SelectGraphicRendition::Background(
                            TerminalColor::DefaultBackground,
                        ))
                    } else {
                        TerminalOutput::Sgr(SelectGraphicRendition::UnderlineColor(
                            TerminalColor::DefaultUnderlineColor,
                        ))
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

    fn ansi_parser_inner_csi_finished_ich(
        &self,
        output: &mut Vec<TerminalOutput>,
    ) -> Result<Option<ParserInner>, ()> {
        let Ok(param) = parse_param_as::<usize>(&self.params) else {
            warn!("Invalid ich command");
            output.push(TerminalOutput::Invalid);

            return Err(());
        };

        // ecma-48 8.3.64
        output.push(TerminalOutput::InsertSpaces(param.unwrap_or(1)));

        Ok(Some(ParserInner::Empty))
    }
}

fn is_csi_param(b: u8) -> bool {
    (0x30..=0x3f).contains(&b)
}

fn is_csi_terminator(b: u8) -> bool {
    (0x40..=0x7e).contains(&b)
}
fn is_csi_intermediate(b: u8) -> bool {
    (0x20..=0x2f).contains(&b)
}
