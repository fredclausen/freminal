use std::str::FromStr;

//use eframe::egui::Color32;

use crate::terminal_emulator::ansi::{ParserInner, TerminalOutput};

#[derive(Eq, PartialEq, Debug)]
pub enum AnsiOscInternalType {
    Query,
    //SetColor(Color32),
    String(String),
    Unknown(Option<AnsiOscToken>),
}

// to string for OscInternalType

impl std::fmt::Display for AnsiOscInternalType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Query => write!(f, "Query"),
            //Self::SetColor(value) => write!(f, "SetColor({value:?})"),
            Self::String(value) => write!(f, "{value}"),
            Self::Unknown(value) => write!(f, "Unknown({value:?})"),
        }
    }
}

impl From<Vec<Option<AnsiOscToken>>> for AnsiOscInternalType {
    fn from(value: Vec<Option<AnsiOscToken>>) -> Self {
        // The first value is the type of the OSC sequence
        // if the first value is b'?', then it is a query
        // otherwise, it is a set but we'll leave that as unknown for now

        value
            .get(1)
            .map_or(Self::Unknown(None), |value| match value {
                Some(AnsiOscToken::String(value)) => {
                    if value == &"?".to_string() {
                        Self::Query
                    } else {
                        Self::String(value.clone())
                    }
                }
                Some(value) => Self::Unknown(Some(value.clone())),
                None => Self::Unknown(None),
            })
    }
}

#[derive(Eq, PartialEq, Debug)]
enum OscTarget {
    TitleBar,
    IconName,
    Background,
    Foreground,
    // https://iterm2.com/documentation-escape-codes.html
    Ftcs,
    Unknown,
}

impl From<AnsiOscToken> for OscTarget {
    fn from(value: AnsiOscToken) -> Self {
        match value {
            AnsiOscToken::U8(0 | 2) => Self::TitleBar,
            AnsiOscToken::U8(1) => Self::IconName,
            AnsiOscToken::U8(11) => Self::Background,
            AnsiOscToken::U8(10) => Self::Foreground,
            AnsiOscToken::U8(133) => Self::Ftcs,
            _ => Self::Unknown,
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
pub enum AnsiOscType {
    RequestColorQueryBackground(AnsiOscInternalType),
    RequestColorQueryForeground(AnsiOscInternalType),
    Ftcs(String),
    // FIXME: We're handling 0 and 2 as just title bar for now
    // if we go tabbed, we'll need to handle 2 differently
    SetTitleBar(String),
}

impl std::fmt::Display for AnsiOscType {
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
pub enum AnsiOscParserState {
    Params,
    //Intermediates,
    Finished,
    Invalid,
    InvalidFinished,
}

#[derive(Eq, PartialEq, Debug)]
pub struct AnsiOscParser {
    pub(crate) state: AnsiOscParserState,
    pub(crate) params: Vec<u8>,
    pub(crate) intermediates: Vec<u8>,
}

// OSC Sequence looks like this:
// 1b]11;?1b\

impl AnsiOscParser {
    pub const fn new() -> Self {
        Self {
            state: AnsiOscParserState::Params,
            params: Vec::new(),
            intermediates: Vec::new(),
        }
    }

    pub fn push(&mut self, b: u8) {
        if let AnsiOscParserState::Finished | AnsiOscParserState::InvalidFinished = &self.state {
            panic!("OscParser should not be pushed to once finished");
        }

        match self.state {
            AnsiOscParserState::Params => {
                if is_valid_osc_param(b) {
                    self.params.push(b);
                } else {
                    warn!("Invalid OSC param: {:x}", b);
                    self.state = AnsiOscParserState::Invalid;
                }

                if is_osc_terminator(&self.params) {
                    self.state = AnsiOscParserState::Finished;

                    while is_final_character_osc_terminator(self.params[self.params.len() - 1]) {
                        self.params.pop();
                    }
                }
            }
            // OscParserState::Intermediates => {
            //     panic!("OscParser should not be in intermediates state");
            // }
            AnsiOscParserState::Finished | AnsiOscParserState::InvalidFinished => {
                unreachable!()
            }
            AnsiOscParserState::Invalid => {
                if is_osc_terminator(&self.params) {
                    self.state = AnsiOscParserState::InvalidFinished;
                }
            }
        }
    }

    pub fn ansiparser_inner_osc(
        &mut self,
        b: u8,
        output: &mut Vec<TerminalOutput>,
    ) -> Option<ParserInner> {
        self.push(b);

        match self.state {
            AnsiOscParserState::Finished => {
                if let Ok(params) = split_params_into_semicolon_delimited_usize(&self.params) {
                    let type_number = extract_param(0, &params).unwrap();

                    let osc_target = OscTarget::from(type_number.clone());
                    let osc_internal_type = AnsiOscInternalType::from(params);

                    match osc_target {
                        OscTarget::Background => {
                            output.push(TerminalOutput::OscResponse(
                                AnsiOscType::RequestColorQueryBackground(osc_internal_type),
                            ));
                        }
                        OscTarget::Foreground => {
                            output.push(TerminalOutput::OscResponse(
                                AnsiOscType::RequestColorQueryForeground(osc_internal_type),
                            ));
                        }
                        OscTarget::Unknown => {
                            warn!("Unknown OSC target: {:?}", type_number);
                            output.push(TerminalOutput::Invalid);
                        }
                        OscTarget::TitleBar => {
                            output.push(TerminalOutput::OscResponse(AnsiOscType::SetTitleBar(
                                osc_internal_type.to_string(),
                            )));
                        }
                        OscTarget::Ftcs => {
                            warn!("Ftcs is not supported");
                            output.push(TerminalOutput::OscResponse(AnsiOscType::Ftcs(
                                osc_internal_type.to_string(),
                            )));
                        }
                        OscTarget::IconName => {
                            warn!("IconName is not supported");
                            output.push(TerminalOutput::Skipped);
                        }
                    }
                } else {
                    warn!("Invalid OSC params: {:?}", self.params);
                    output.push(TerminalOutput::Invalid);
                };

                Some(ParserInner::Empty)
            }
            AnsiOscParserState::Invalid => {
                output.push(TerminalOutput::Invalid);
                Some(ParserInner::Empty)
            }
            _ => None,
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
pub enum AnsiOscToken {
    U8(u8),
    String(String),
}

impl FromStr for AnsiOscToken {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u8>().map_or_else(
            |_| Ok(Self::String(s.to_string())),
            |value| Ok(Self::U8(value)),
        )
    }
}

pub fn split_params_into_semicolon_delimited_usize(
    params: &[u8],
) -> Result<Vec<Option<AnsiOscToken>>, ()> {
    let params = params
        .split(|b| *b == b';')
        .map(parse_param_as::<AnsiOscToken>)
        .collect::<Result<Vec<Option<AnsiOscToken>>, ()>>();

    params
}

pub fn parse_param_as<T: std::str::FromStr>(param_bytes: &[u8]) -> Result<Option<T>, ()> {
    let param_str =
        std::str::from_utf8(param_bytes).expect("parameter should always be valid utf8");
    if param_str.is_empty() {
        return Ok(None);
    }
    param_str.parse().map_err(|_| ()).map_or_else(
        |()| {
            warn!(
                "Failed to parse parameter ({:?}) as {:?}",
                param_bytes,
                std::any::type_name::<T>()
            );
            Err(())
        },
        |value| Ok(Some(value)),
    )
}

pub fn extract_param(idx: usize, params: &[Option<AnsiOscToken>]) -> Option<AnsiOscToken> {
    // get the parameter at the index
    params.get(idx).and_then(std::clone::Clone::clone)
}
