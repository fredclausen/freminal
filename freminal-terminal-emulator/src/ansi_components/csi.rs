// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use super::{
    csi_commands::{
        cha::ansi_parser_inner_csi_finished_set_position_g,
        cub::ansi_parser_inner_csi_finished_move_left,
        cud::ansi_parser_inner_csi_finished_move_down,
        cuf::ansi_parser_inner_csi_finished_move_right,
        cup::ansi_parser_inner_csi_finished_set_position_h,
        cuu::ansi_parser_inner_csi_finished_move_up,
        dch::ansi_parser_inner_csi_finished_set_position_p,
        decrqm::ansi_parser_inner_csi_finished_decrqm,
        decscusr::ansi_parser_inner_csi_finished_set_position_q,
        decslpp::ansi_parser_inner_csi_finished_set_position_t,
        decstbm::ansi_parser_inner_csi_set_top_and_bottom_margins,
        ech::ansi_parser_inner_csi_finished_set_position_x,
        ed::ansi_parser_inner_csi_finished_set_position_j,
        el::ansi_parser_inner_csi_finished_set_position_k, ict::ansi_parser_inner_csi_finished_ich,
        il::ansi_parser_inner_csi_finished_set_position_l,
        send_device_attributes::ansi_parser_inner_csi_finished_send_da,
        sgr::ansi_parser_inner_csi_finished_sgr_ansi,
    },
    mode::{terminal_mode_from_params, SetMode},
};
use crate::ansi::{ParserInner, TerminalOutput};
use crate::error::ParserFailures;
use anyhow::Result;

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
    pub state: AnsiCsiParserState,
    pub params: Vec<u8>,
    pub intermediates: Vec<u8>,
    pub sequence: Vec<u8>,
}

impl Default for AnsiCsiParser {
    fn default() -> Self {
        Self::new()
    }
}

impl AnsiCsiParser {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: AnsiCsiParserState::Params,
            params: Vec::new(),
            intermediates: Vec::new(),
            sequence: Vec::new(),
        }
    }

    /// Push a byte into the parser
    ///
    /// # Errors
    /// Will return an error if the parser is in a finished state
    pub fn push(&mut self, b: u8) -> Result<()> {
        if let AnsiCsiParserState::Finished(_) | AnsiCsiParserState::InvalidFinished = &self.state {
            return Err(ParserFailures::ParsedPushedToOnceFinished.into());
        }

        self.sequence.push(b);

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

        Ok(())
    }

    /// Push a byte into the parser and return the next state
    ///
    /// # Errors
    /// Will return an error if the parser encounters an invalid state
    #[allow(clippy::too_many_lines)]
    pub fn ansiparser_inner_csi(
        &mut self,
        b: u8,
        output: &mut Vec<TerminalOutput>,
    ) -> Result<Option<ParserInner>> {
        self.push(b)?;

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
            AnsiCsiParserState::Finished(b'X') => {
                ansi_parser_inner_csi_finished_set_position_x(&self.params, output)
            }
            AnsiCsiParserState::Finished(b'm') => {
                ansi_parser_inner_csi_finished_sgr_ansi(&self.params, output)
            }
            AnsiCsiParserState::Finished(b'h') => {
                output.push(TerminalOutput::Mode(terminal_mode_from_params(
                    &self.params,
                    &SetMode::DecSet,
                )));
                Ok(Some(ParserInner::Empty))
            }
            AnsiCsiParserState::Finished(b'l') => {
                output.push(TerminalOutput::Mode(terminal_mode_from_params(
                    &self.params,
                    &SetMode::DecRst,
                )));
                Ok(Some(ParserInner::Empty))
            }
            AnsiCsiParserState::Finished(b'@') => {
                ansi_parser_inner_csi_finished_ich(&self.params, output)
            }
            AnsiCsiParserState::Finished(b'n') => {
                output.push(TerminalOutput::CursorReport);
                Ok(Some(ParserInner::Empty))
            }
            AnsiCsiParserState::Finished(b't') => {
                ansi_parser_inner_csi_finished_set_position_t(&self.params, output)
            }
            AnsiCsiParserState::Finished(b'p') => {
                ansi_parser_inner_csi_finished_decrqm(&self.params, &self.intermediates, b, output)
            }
            AnsiCsiParserState::Finished(b'q') => {
                ansi_parser_inner_csi_finished_set_position_q(&self.params, output)
            }
            AnsiCsiParserState::Finished(b'r') => {
                ansi_parser_inner_csi_set_top_and_bottom_margins(&self.params, output)
            }
            AnsiCsiParserState::Finished(b'c') => {
                ansi_parser_inner_csi_finished_send_da(&self.params, &self.intermediates, output)
            }
            AnsiCsiParserState::Finished(b'u') => {
                format_error_output(&self.sequence);
                output.push(TerminalOutput::Skipped);
                Ok(Some(ParserInner::Empty))
            }
            AnsiCsiParserState::Finished(_esc) => {
                format_error_output(&self.sequence);
                output.push(TerminalOutput::Invalid);

                Ok(Some(ParserInner::Empty))
            }
            AnsiCsiParserState::Invalid => {
                format_error_output(&self.sequence);
                output.push(TerminalOutput::Invalid);

                Ok(Some(ParserInner::Empty))
            }
            _ => Ok(None),
        }
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

fn format_error_output(sequence: &[u8]) {
    let params = String::from_utf8(sequence.to_vec())
        .unwrap_or_else(|_| "Unable To Parse Params".to_string());
    warn!("Unhandled CSI sequence: [{params}");
}
