use crate::terminal_emulator::ansi::{parse_param_as, ParserInner, TerminalOutput};

use super::{
    csi_commands::{
        cha::ansi_parser_inner_csi_finished_set_position_g,
        cub::ansi_parser_inner_csi_finished_move_left,
        cud::ansi_parser_inner_csi_finished_move_down,
        cuf::ansi_parser_inner_csi_finished_move_right,
        cup::ansi_parser_inner_csi_finished_set_position_h,
        cuu::ansi_parser_inner_csi_finished_move_up,
        dch::ansi_parser_inner_csi_finished_set_position_p,
        ed::ansi_parser_inner_csi_finished_set_position_j,
        el::ansi_parser_inner_csi_finished_set_position_k,
        il::ansi_parser_inner_csi_finished_set_position_l,
        sgr::ansi_parser_inner_csi_finished_sgr_ansi,
    },
    mode::terminal_mode_from_params,
};

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
                ansi_parser_inner_csi_finished_move_up(&self.params, output)
            }
            AnsiCsiParserState::Finished(b'B') => {
                ansi_parser_inner_csi_finished_move_down(&self.params, output)
            }
            AnsiCsiParserState::Finished(b'C') => {
                ansi_parser_inner_csi_finished_move_right(&self.params, output)
            }
            AnsiCsiParserState::Finished(b'D') => {
                ansi_parser_inner_csi_finished_move_left(&self.params, output)
            }
            AnsiCsiParserState::Finished(b'H') => {
                ansi_parser_inner_csi_finished_set_position_h(&self.params, output)
            }
            AnsiCsiParserState::Finished(b'G') => {
                ansi_parser_inner_csi_finished_set_position_g(&self.params, output)
            }
            AnsiCsiParserState::Finished(b'J') => {
                ansi_parser_inner_csi_finished_set_position_j(&self.params, output)
            }
            AnsiCsiParserState::Finished(b'K') => {
                ansi_parser_inner_csi_finished_set_position_k(&self.params, output)
            }
            AnsiCsiParserState::Finished(b'L') => {
                ansi_parser_inner_csi_finished_set_position_l(&self.params, output)
            }
            AnsiCsiParserState::Finished(b'P') => {
                ansi_parser_inner_csi_finished_set_position_p(&self.params, output)
            }
            AnsiCsiParserState::Finished(b'm') => {
                ansi_parser_inner_csi_finished_sgr_ansi(&self.params, output)
            }
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
