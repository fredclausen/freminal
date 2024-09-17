use crate::terminal_emulator::ansi::{
    extract_param, split_params_into_semicolon_delimited_usize, AnsiParserInner, TerminalOutput,
};

#[derive(Eq, PartialEq, Debug)]
pub enum OscType {
    RequestColorSetResponse(String),
    UnknownType(Vec<u8>),
}

impl From<Vec<u8>> for OscType {
    fn from(value: Vec<u8>) -> Self {
        let string_of_numbers = String::from_utf8(value.clone()).unwrap();

        match string_of_numbers.as_str() {
            "11" => OscType::RequestColorSetResponse("11".to_string()),
            _ => OscType::UnknownType(value),
        }
    }
}

impl std::fmt::Display for OscType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OscType::RequestColorSetResponse(value) => {
                write!(f, "RequestColorSetResponse({:?})", value)
            }
            OscType::UnknownType(value) => write!(f, "UnknownType({:?})", value),
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
pub enum OscParserState {
    Params,
    Intermediates,
    Finished,
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
// 1b]11;?1b\

impl OscParser {
    pub fn new() -> Self {
        Self {
            state: OscParserState::Params,
            params: Vec::new(),
            intermediates: Vec::new(),
        }
    }

    pub fn push(&mut self, b: u8) {
        if let OscParserState::Finished | OscParserState::InvalidFinished = &self.state {
            panic!("CsiParser should not be pushed to once finished");
        }

        match self.state {
            OscParserState::Params => {
                if is_valid_osc_param(b) {
                    self.params.push(b);
                } else if is_osc_terminator(b) {
                    self.state = OscParserState::Finished;
                } else {
                    self.state = OscParserState::Invalid;
                }
            }
            OscParserState::Intermediates => {
                panic!("OscParser should not be in intermediates state");
            }
            OscParserState::Finished | OscParserState::InvalidFinished => {
                unreachable!()
            }
            OscParserState::Invalid => {
                if is_osc_terminator(b) {
                    self.state = OscParserState::InvalidFinished;
                }
            }
        }
    }

    pub fn ansiparser_inner_osc(
        &mut self,
        b: u8,
        output: &mut Vec<TerminalOutput>,
    ) -> Result<Option<AnsiParserInner>, ()> {
        self.push(b);

        let return_value = match self.state {
            OscParserState::Finished => {
                match split_params_into_semicolon_delimited_usize(&self.params) {
                    Ok(params) => {
                        let type_number = extract_param(0, &params).unwrap();

                        match OscType::from(vec![type_number as u8]) {
                            OscType::RequestColorSetResponse(value) => {
                                match extract_param(1, &params).unwrap() as u8 {
                                    b'?' => {
                                        output.push(TerminalOutput::OscResponse(
                                            OscType::RequestColorSetResponse(value),
                                        ));
                                    }
                                    _ => {
                                        output.push(TerminalOutput::Invalid);
                                    }
                                }
                            }
                            OscType::UnknownType(_) => {
                                output.push(TerminalOutput::Invalid);
                            }
                        }
                    }
                    Err(_) => {
                        output.push(TerminalOutput::Invalid);
                    }
                };

                Ok(Some(AnsiParserInner::Empty))
            }
            OscParserState::Invalid => {
                output.push(TerminalOutput::Invalid);
                Ok(Some(AnsiParserInner::Empty))
            }
            _ => return Ok(None),
        };

        return_value
    }
}

// the terminator of the OSC sequence is a ST (0x5C) or BEL (0x07)
fn is_osc_terminator(b: u8) -> bool {
    b == b'\x5C' || b == b'\x07'
}

fn is_valid_osc_param(b: u8) -> bool {
    (0x30..=0x3F).contains(&b)
}
