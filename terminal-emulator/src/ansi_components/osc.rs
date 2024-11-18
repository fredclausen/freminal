use std::fmt;
use std::str::FromStr;

//use eframe::egui::Color32;

use crate::ansi::{ParserInner, TerminalOutput};
use crate::error::ParserFailures;
use anyhow::Result;

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
    RemoteHost,
    Url,
    Unknown,
}

impl From<AnsiOscToken> for OscTarget {
    fn from(value: AnsiOscToken) -> Self {
        match value {
            AnsiOscToken::U8(0 | 2) => Self::TitleBar,
            AnsiOscToken::U8(1) => Self::IconName,
            AnsiOscToken::U8(7) => Self::RemoteHost,
            AnsiOscToken::U8(8) => Self::Url,
            AnsiOscToken::U8(11) => Self::Background,
            AnsiOscToken::U8(10) => Self::Foreground,
            AnsiOscToken::U8(133) => Self::Ftcs,
            _ => Self::Unknown,
        }
    }
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum UrlResponse {
    Url(Url),
    End,
}

impl std::fmt::Display for UrlResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Url(url) => write!(f, "Url({url})"),
            Self::End => write!(f, "End"),
        }
    }
}

impl From<Vec<Option<AnsiOscToken>>> for UrlResponse {
    fn from(value: Vec<Option<AnsiOscToken>>) -> Self {
        // There are two tokens that we care about
        // if BOTH tokens are None, then it is the end of the URL

        // Otherwise, the first token is the ID, and the second token is the URL
        match value.as_slice() {
            [Some(AnsiOscToken::U8(8)), Some(AnsiOscToken::String(id)), Some(AnsiOscToken::String(url))] => {
                Self::Url(Url {
                    id: Some(id.clone()),
                    url: url.clone(),
                })
            }
            [Some(AnsiOscToken::U8(8)), None, Some(AnsiOscToken::String(url))] => Self::Url(Url {
                id: None,
                url: url.clone(),
            }),
            _ => Self::End,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    // Ostensibly, the ID is a key/value pair that is used to identify the URL
    // However, the current spec (https://iterm2.com/documentation-escape-codes.html) only
    // defines the ID as the only valid parameter
    pub id: Option<String>,
    pub url: String,
}

impl fmt::Display for Url {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Url {{ id: {}, url: {} }}",
            self.id.clone().unwrap_or_else(|| "None".to_string()),
            self.url
        )
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
    Url(UrlResponse),
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
            Self::Url(url) => write!(f, "Url({url})"),
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

impl Default for AnsiOscParser {
    fn default() -> Self {
        Self::new()
    }
}

impl AnsiOscParser {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: AnsiOscParserState::Params,
            params: Vec::new(),
            intermediates: Vec::new(),
        }
    }

    /// Push a byte into the parser
    ///
    /// # Errors
    /// Will return an error if the parser is in the `Finished` or `InvalidFinished` state
    pub fn push(&mut self, b: u8) -> Result<()> {
        if let AnsiOscParserState::Finished | AnsiOscParserState::InvalidFinished = &self.state {
            return Err(ParserFailures::ParsedPushedToOnceFinished.into());
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

        Ok(())
    }

    /// Parse the OSC sequence
    ///
    /// # Errors
    /// Will return an error if the parser is in the `Finished` or `InvalidFinished` state
    pub fn ansiparser_inner_osc(
        &mut self,
        b: u8,
        output: &mut Vec<TerminalOutput>,
    ) -> Result<Option<ParserInner>> {
        self.push(b)?;

        match self.state {
            AnsiOscParserState::Finished => {
                if let Ok(params) = split_params_into_semicolon_delimited_usize(&self.params) {
                    let Some(type_number) = extract_param(0, &params) else {
                        warn!("Invalid OSC params: {:?}", self.params);
                        output.push(TerminalOutput::Invalid);
                        return Ok(Some(ParserInner::Empty));
                    };

                    let osc_target = OscTarget::from(type_number.clone());
                    let osc_internal_type = AnsiOscInternalType::from(params.clone());

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
                        OscTarget::RemoteHost => {
                            warn!("RemoteHost is not supported");
                            output.push(TerminalOutput::Skipped);
                        }
                        OscTarget::Url => {
                            let url_response = UrlResponse::from(params);
                            output
                                .push(TerminalOutput::OscResponse(AnsiOscType::Url(url_response)));
                        }
                        OscTarget::Unknown => {
                            warn!("Unknown OSC target: {:?}", type_number);
                            output.push(TerminalOutput::Invalid);
                        }
                    }
                } else {
                    warn!("Invalid OSC params: {:?}", self.params);
                    output.push(TerminalOutput::Invalid);
                };

                Ok(Some(ParserInner::Empty))
            }
            AnsiOscParserState::Invalid => {
                output.push(TerminalOutput::Invalid);
                Ok(Some(ParserInner::Empty))
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

/// # Errors
/// Will return an error if the parameter is not a valid number
pub fn split_params_into_semicolon_delimited_usize(
    params: &[u8],
) -> Result<Vec<Option<AnsiOscToken>>> {
    let params = params
        .split(|b| *b == b';')
        .map(parse_param_as::<AnsiOscToken>)
        .collect::<Result<Vec<Option<AnsiOscToken>>, anyhow::Error>>();

    params
}

/// # Errors
///
/// Will return an error if the parameter is not a valid number
pub fn parse_param_as<T: std::str::FromStr>(param_bytes: &[u8]) -> Result<Option<T>> {
    let param_str = std::str::from_utf8(param_bytes)?;
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
            Err(anyhow::anyhow!("Failed to parse parameter"))
        },
        |value| Ok(Some(value)),
    )
}

pub fn extract_param(idx: usize, params: &[Option<AnsiOscToken>]) -> Option<AnsiOscToken> {
    // get the parameter at the index
    params.get(idx).and_then(std::clone::Clone::clone)
}
