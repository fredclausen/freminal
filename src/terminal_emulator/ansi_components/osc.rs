use crate::terminal_emulator::ansi::{AnsiParserInner, TerminalOutput};

#[derive(Eq, PartialEq, Debug)]
pub enum OscParserState {
    Params,
    Intermediates,
    Finished(u8),
    Invalid,
    InvalidFinished,
}

#[derive(Eq, PartialEq, Debug)]
pub struct OscParser {
    pub(crate) state: OscParserState,
    pub(crate) params: Vec<u8>,
    pub(crate) intermediates: Vec<u8>,
}

// OSC Sequence looks like this:
// ESC ] ... ST

impl OscParser {
    pub fn new() -> Self {
        Self {
            state: OscParserState::Params,
            params: Vec::new(),
            intermediates: Vec::new(),
        }
    }

    pub fn push(&mut self, b: u8) {
        if let OscParserState::Finished(_) | OscParserState::InvalidFinished = &self.state {
            panic!("CsiParser should not be pushed to once finished");
        }
    }

    // pub fn ansiparser_inner_csi(
    //     &mut self,
    //     b: u8,
    //     output: &mut Vec<TerminalOutput>,
    // ) -> Result<Option<AnsiParserInner>, ()> {
    //     self.push(b);
    // }
}

// the terminator of the OSC sequence is a ST (0x5C) or BEL (0x07)
fn is_osc_terminator(b: u8) -> bool {
    b == b'\x5C' || b == b'\x07'
}

fn is_valid_osc_param(b: u8) -> bool {
    (0x30..=0x3F).contains(&b)
}
