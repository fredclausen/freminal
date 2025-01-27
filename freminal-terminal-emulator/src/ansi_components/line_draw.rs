// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::{
    ansi::{ParserInner, TerminalOutput},
    error::ParserFailures,
};
use anyhow::Result;

#[derive(Eq, PartialEq, Debug, Default)]
pub enum DecSpecialGraphics {
    Replace,
    #[default]
    DontReplace,
}

#[derive(Eq, PartialEq, Debug)]
pub enum DecSpecialGraphicsState {
    Waiting,
    Finished(DecSpecialGraphics),
    InvalidFinished,
}

#[derive(Eq, PartialEq, Debug)]
pub struct DecSpecialGraphicsParser {
    pub state: DecSpecialGraphicsState,
}

impl Default for DecSpecialGraphicsParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DecSpecialGraphicsParser {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: DecSpecialGraphicsState::Waiting,
        }
    }

    pub fn ansi_parser_inner_line_draw(
        &mut self,
        b: u8,
        output: &mut Vec<TerminalOutput>,
    ) -> Option<ParserInner> {
        match self.push(b) {
            Ok(()) => {}
            Err(e) => {
                error!("Error: {:?}", e);
                output.push(TerminalOutput::Invalid);
                return Some(ParserInner::Empty);
            }
        }

        match self.state {
            DecSpecialGraphicsState::Finished(DecSpecialGraphics::DontReplace) => {
                output.push(TerminalOutput::DecSpecialGraphics(
                    DecSpecialGraphics::DontReplace,
                ));
                Some(ParserInner::Empty)
            }
            DecSpecialGraphicsState::Finished(DecSpecialGraphics::Replace) => {
                output.push(TerminalOutput::DecSpecialGraphics(
                    DecSpecialGraphics::Replace,
                ));
                Some(ParserInner::Empty)
            }
            DecSpecialGraphicsState::InvalidFinished => {
                output.push(TerminalOutput::Invalid);
                Some(ParserInner::Empty)
            }
            DecSpecialGraphicsState::Waiting => unreachable!(),
        }
    }

    /// Push a byte into the parser
    ///
    /// # Errors
    /// Will return an error if the parser is in an invalid state
    pub fn push(&mut self, byte: u8) -> Result<()> {
        if let DecSpecialGraphicsState::InvalidFinished | DecSpecialGraphicsState::Finished(_) =
            self.state
        {
            return Err(ParserFailures::ParsedPushedToOnceFinished.into());
        }

        match self.state {
            DecSpecialGraphicsState::Waiting => match byte {
                b'0' => {
                    self.state = DecSpecialGraphicsState::Finished(DecSpecialGraphics::Replace);
                }
                b'B' => {
                    self.state = DecSpecialGraphicsState::Finished(DecSpecialGraphics::DontReplace);
                }
                _ => {
                    self.state = DecSpecialGraphicsState::InvalidFinished;
                }
            },
            DecSpecialGraphicsState::Finished(_) | DecSpecialGraphicsState::InvalidFinished => {
                unreachable!()
            }
        }

        Ok(())
    }
}
