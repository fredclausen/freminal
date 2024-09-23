use std::str::FromStr;

use eframe::egui::Color32;

use crate::terminal_emulator::ansi::{AnsiParserInner, TerminalOutput};

#[derive(Eq, PartialEq, Debug)]
pub enum OscInternalType {
    Query,
    SetColor(Color32),
    String(String),
    Unknown(Option<OscToken>),
}

// to string for OscInternalType

impl std::fmt::Display for OscInternalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Query => write!(f, "Query"),
            Self::SetColor(value) => write!(f, "SetColor({value:?})"),
            Self::String(value) => write!(f, "{value}"),
            Self::Unknown(value) => write!(f, "Unknown({value:?})"),
        }
    }
}

impl From<Vec<Option<OscToken>>> for OscInternalType {
    fn from(value: Vec<Option<OscToken>>) -> Self {
        // The first value is the type of the OSC sequence
        // if the first value is b'?', then it is a query
        // otherwise, it is a set but we'll leave that as unknown for now

        match value.get(1) {
            Some(value) => match value {
                Some(OscToken::String(value)) => {
                    if value == &"?".to_string() {
                        Self::Query
                    } else {
                        Self::String(value.clone())
                    }
                }
                Some(value) => Self::Unknown(Some(value.clone())),
                None => Self::Unknown(None),
            },
            None => Self::Unknown(None),
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
enum OscTarget {
    TitleBar,
    Background,
    Foreground,
    // https://iterm2.com/documentation-escape-codes.html
    Ftcs,
    Unknown,
}

impl From<OscToken> for OscTarget {
    fn from(value: OscToken) -> Self {
        match value {
            OscToken::U8(0) => Self::TitleBar,
            OscToken::U8(11) => Self::Background,
            OscToken::U8(10) => Self::Foreground,
            OscToken::U8(133) => Self::Ftcs,
            _ => Self::Unknown,
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
pub enum OscType {
    RequestColorQueryBackground(OscInternalType),
    RequestColorQueryForeground(OscInternalType),
    Ftcs(String),
    SetTitleBar(String),
}

impl std::fmt::Display for OscType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RequestColorQueryBackground(value) => {
                write!(f, "RequestColorQueryBackground({value:?})")
            }
            Self::RequestColorQueryForeground(value) => {
                write!(f, "RequestColorQueryForeground({value:?})")
            }
            Self::SetTitleBar(value) => write!(f, "SetTitleBar({value:?})"),
            Self::Ftcs(value) => write!(f, "Ftcs ({value:?})"),
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
    pub const fn new() -> Self {
        Self {
            state: OscParserState::Params,
            params: Vec::new(),
            intermediates: Vec::new(),
        }
    }

    pub fn push(&mut self, b: u8) {
        if let OscParserState::Finished | OscParserState::InvalidFinished = &self.state {
            panic!("OscParser should not be pushed to once finished");
        }

        match self.state {
            OscParserState::Params => {
                if is_valid_osc_param(b) {
                    self.params.push(b);
                } else {
                    warn!("Invalid OSC param: {:x}", b);
                    self.state = OscParserState::Invalid;
                }

                if is_osc_terminator(&self.params) {
                    self.state = OscParserState::Finished;

                    while is_final_character_osc_terminator(self.params[self.params.len() - 1]) {
                        self.params.pop();
                    }
                }
            }
            OscParserState::Intermediates => {
                panic!("OscParser should not be in intermediates state");
            }
            OscParserState::Finished | OscParserState::InvalidFinished => {
                unreachable!()
            }
            OscParserState::Invalid => {
                if is_osc_terminator(&self.params) {
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

        

        match self.state {
            OscParserState::Finished => {
                if let Ok(params) = split_params_into_semicolon_delimited_usize(&self.params) {
                    let type_number = extract_param(0, &params).unwrap();

                    let osc_target = OscTarget::from(type_number.clone());
                    let osc_internal_type = OscInternalType::from(params);

                    match osc_target {
                        OscTarget::Background => {
                            output.push(TerminalOutput::OscResponse(
                                OscType::RequestColorQueryBackground(osc_internal_type),
                            ));
                        }
                        OscTarget::Foreground => {
                            output.push(TerminalOutput::OscResponse(
                                OscType::RequestColorQueryForeground(osc_internal_type),
                            ));
                        }
                        OscTarget::Unknown => {
                            warn!("Unknown OSC target: {:?}", type_number);
                            output.push(TerminalOutput::Invalid);
                        }
                        OscTarget::TitleBar => {
                            warn!("TitleBar is not supported");
                            output.push(TerminalOutput::OscResponse(OscType::SetTitleBar(
                                osc_internal_type.to_string(),
                            )));
                        }
                        OscTarget::Ftcs => {
                            warn!("Ftcs is not supported");
                            output.push(TerminalOutput::OscResponse(OscType::Ftcs(
                                osc_internal_type.to_string(),
                            )));
                        }
                    }
                } else {
                    warn!("Invalid OSC params: {:?}", self.params);
                    output.push(TerminalOutput::Invalid);
                };

                Ok(Some(AnsiParserInner::Empty))
            }
            OscParserState::Invalid => {
                output.push(TerminalOutput::Invalid);
                Ok(Some(AnsiParserInner::Empty))
            }
            _ => Ok(None),
        }
    }
}

// the terminator of the OSC sequence is a ST (0x5C) or BEL (0x07)
const fn is_osc_terminator(b: &[u8]) -> bool {
    // the array has to be at least 4 bytes long, and the last two characters need to be 0x1b and 0x5c

    if b.len() < 2 {
        return false;
    }

    b[b.len() - 2] == 0x1b && b[b.len() - 1] == 0x5c || b[b.len() - 1] == 0x07
}

const fn is_final_character_osc_terminator(b: u8) -> bool {
    b == 0x5c || b == 0x07 || b == 0x1b
}

fn is_valid_osc_param(b: u8) -> bool {
    // if the character is a printable character, or is 0x1b or 0x5c then it is valid
    (0x20..=0x7E).contains(&b) || (0x80..=0xff).contains(&b) || b == 0x1b || b == 0x07
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OscToken {
    U8(u8),
    String(String),
}

impl FromStr for OscToken {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(value) = s.parse::<u8>() {
            Ok(Self::U8(value))
        } else {
            Ok(Self::String(s.to_string()))
        }
    }
}

pub fn split_params_into_semicolon_delimited_usize(
    params: &[u8],
) -> Result<Vec<Option<OscToken>>, ()> {
    let params = params
        .split(|b| *b == b';')
        .map(parse_param_as::<OscToken>)
        .collect::<Result<Vec<Option<OscToken>>, ()>>();

    params
}

pub fn parse_param_as<T: std::str::FromStr>(param_bytes: &[u8]) -> Result<Option<T>, ()> {
    let param_str =
        std::str::from_utf8(param_bytes).expect("parameter should always be valid utf8");
    if param_str.is_empty() {
        return Ok(None);
    }
    if let Ok(value) = param_str.parse().map_err(|_| ()) { Ok(Some(value)) } else {
        warn!(
            "Failed to parse parameter ({:?}) as {:?}",
            param_bytes,
            std::any::type_name::<T>()
        );
        Err(())
    }
}

pub fn extract_param(idx: usize, params: &[Option<OscToken>]) -> Option<OscToken> {
    // get the parameter at the index

    if let Some(value) = params.get(idx) {
        value.clone()
    } else {
        None
    }
}
