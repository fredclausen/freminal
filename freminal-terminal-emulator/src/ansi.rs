// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::{
    ansi_components::{
        csi::AnsiCsiParser,
        line_draw::{DecSpecialGraphics, DecSpecialGraphicsParser},
        mode::Mode,
        osc::{AnsiOscParser, AnsiOscType},
        sgr::SelectGraphicRendition,
    },
    error::ParserFailures,
};

use anyhow::Result;
use freminal_common::{cursor::CursorVisualStyle, window_manipulation::WindowManipulation};

#[derive(Debug, Eq, PartialEq)]
pub enum TerminalOutput {
    SetCursorPos {
        x: Option<usize>,
        y: Option<usize>,
    },
    SetCursorPosRel {
        x: Option<i32>,
        y: Option<i32>,
    },
    ClearDisplayfromCursortoEndofDisplay,
    ClearDiplayfromStartofDisplaytoCursor,
    ClearScrollbackandDisplay,
    ClearDisplay,
    CarriageReturn,
    ClearLineForwards,
    ClearLineBackwards,
    ClearLine,
    Newline,
    Backspace,
    Bell,
    ApplicationKeypadMode,
    NormalKeypadMode,
    InsertLines(usize),
    Delete(usize),
    Erase(usize),
    Sgr(SelectGraphicRendition),
    Data(Vec<u8>),
    Mode(Mode),
    // ich (8.3.64 of ecma-48)
    InsertSpaces(usize),
    OscResponse(AnsiOscType),
    CursorReport,
    Invalid,
    Skipped,
    DecSpecialGraphics(DecSpecialGraphics),
    CursorVisualStyle(CursorVisualStyle),
    WindowManipulation(WindowManipulation),
    RequestDeviceAttributes,
    SetTopAndBottomMargins {
        top_margin: usize,
        bottom_margin: usize,
    },
}

// impl format display for TerminalOutput

impl std::fmt::Display for TerminalOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SetCursorPos { x, y } => {
                write!(f, "SetCursorPos: x: {x:?}, y: {y:?}")
            }
            Self::SetCursorPosRel { x, y } => {
                write!(f, "SetCursorPosRel: x: {x:?}, y: {y:?}")
            }
            Self::ClearDisplayfromCursortoEndofDisplay => write!(f, "ClearForwards"),
            Self::ClearScrollbackandDisplay => write!(f, "ClearAll"),
            Self::ClearDiplayfromStartofDisplaytoCursor => write!(f, "ClearBackwards"),
            Self::ClearDisplay => write!(f, "ClearDisplay"),
            Self::CarriageReturn => write!(f, "CarriageReturn"),
            Self::ClearLineForwards => write!(f, "ClearLineForwards"),
            Self::ClearLineBackwards => write!(f, "ClearLineBackwards"),
            Self::ClearLine => write!(f, "ClearLine"),
            Self::Newline => write!(f, "Newline"),
            Self::Backspace => write!(f, "Backspace"),
            Self::Bell => write!(f, "Bell"),
            Self::InsertLines(n) => write!(f, "InsertLines({n})"),
            Self::Delete(n) => write!(f, "Delete({n})"),
            Self::Erase(n) => write!(f, "Erase({n})"),
            Self::Sgr(sgr) => write!(f, "Sgr({sgr:?})"),
            Self::Data(data) => {
                write!(f, "Data({})", String::from_utf8_lossy(data))
            }
            Self::Mode(mode) => write!(f, "SetMode({mode})"),
            Self::InsertSpaces(n) => write!(f, "InsertSpaces({n})"),
            Self::OscResponse(n) => write!(f, "OscResponse({n})"),
            Self::DecSpecialGraphics(dec_special_graphics) => {
                write!(f, "DecSpecialGraphics({dec_special_graphics:?})")
            }
            Self::Invalid => write!(f, "Invalid"),
            Self::CursorReport => write!(f, "CursorReport"),
            Self::Skipped => write!(f, "Skipped"),
            Self::ApplicationKeypadMode => write!(f, "ApplicationKeypadMode"),
            Self::NormalKeypadMode => write!(f, "NormalKeypadMode"),
            Self::CursorVisualStyle(cursor_visual_style) => {
                write!(f, "CursorVisualStyle({cursor_visual_style:?})")
            }
            Self::WindowManipulation(window_manipulation) => {
                write!(f, "WindowManipulation({window_manipulation:?})")
            }
            Self::SetTopAndBottomMargins {
                top_margin,
                bottom_margin,
            } => {
                write!(f, "SetTopAndBottomMargins({top_margin}, {bottom_margin})")
            }
            Self::RequestDeviceAttributes => write!(f, "RequestDeviceAttributes"),
        }
    }
}

#[must_use]
pub fn extract_param(idx: usize, params: &[Option<usize>]) -> Option<usize> {
    params.get(idx).copied().flatten()
}

/// # Errors
/// Will return an error if the parameter is not a valid number
pub fn split_params_into_semicolon_delimited_usize(params: &[u8]) -> Result<Vec<Option<usize>>> {
    params
        .split(|b| *b == b';')
        .map(parse_param_as::<usize>)
        .collect::<Result<Vec<Option<usize>>>>()
}

/// # Errors
/// Will return an error if the parameter is not a valid number
pub fn split_params_into_colon_delimited_usize(params: &[u8]) -> Result<Vec<Option<usize>>> {
    params
        .split(|b| *b == b':')
        .map(parse_param_as::<usize>)
        .collect::<Result<Vec<Option<usize>>>>()
}

/// # Errors
/// Will return an error if the parameter is not a valid number
pub fn parse_param_as<T: std::str::FromStr>(param_bytes: &[u8]) -> Result<Option<T>> {
    let param_str = std::str::from_utf8(param_bytes)?;

    if param_str.is_empty() {
        return Ok(None);
    }

    param_str
        .parse()
        .map_err(|_| anyhow::Error::msg("Parse error"))
        .map(Some)
}

fn push_data_if_non_empty(data: &mut Vec<u8>, output: &mut Vec<TerminalOutput>) {
    if !data.is_empty() {
        output.push(TerminalOutput::Data(std::mem::take(data)));
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ParserInner {
    Empty,
    Escape,
    Csi(AnsiCsiParser),
    Osc(AnsiOscParser),
    DecSpecialGraphics(DecSpecialGraphicsParser),
}

#[derive(Debug, Eq, PartialEq)]
pub struct FreminalAnsiParser {
    pub inner: ParserInner,
}

impl Default for FreminalAnsiParser {
    fn default() -> Self {
        Self::new()
    }
}

impl FreminalAnsiParser {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            inner: ParserInner::Empty,
        }
    }

    fn ansi_parser_inner_empty(
        &mut self,
        b: u8,
        data_output: &mut Vec<u8>,
        output: &mut Vec<TerminalOutput>,
    ) -> Result<(), ()> {
        if b == b'\x1b' {
            self.inner = ParserInner::Escape;
            return Err(());
        }

        if b == b'\r' {
            push_data_if_non_empty(data_output, output);
            output.push(TerminalOutput::CarriageReturn);
            return Err(());
        }

        if b == b'\n' {
            push_data_if_non_empty(data_output, output);
            output.push(TerminalOutput::Newline);
            return Err(());
        }

        if b == 0x08 {
            push_data_if_non_empty(data_output, output);
            output.push(TerminalOutput::Backspace);
            return Err(());
        }

        if b == 0x07 {
            push_data_if_non_empty(data_output, output);
            output.push(TerminalOutput::Bell);
            return Err(());
        }

        Ok(())
    }

    fn ansiparser_inner_escape(
        &mut self,
        b: u8,
        data_output: &mut Vec<u8>,
        output: &mut Vec<TerminalOutput>,
    ) -> Result<()> {
        push_data_if_non_empty(data_output, output);

        match b {
            b'[' => {
                self.inner = ParserInner::Csi(AnsiCsiParser::new());
            }
            b']' => {
                self.inner = ParserInner::Osc(AnsiOscParser::new());
            }
            b'(' => {
                self.inner = ParserInner::DecSpecialGraphics(DecSpecialGraphicsParser::new());
            }
            b'=' => {
                self.inner = ParserInner::Empty;
                output.push(TerminalOutput::ApplicationKeypadMode);
            }
            b'>' => {
                self.inner = ParserInner::Empty;
                output.push(TerminalOutput::NormalKeypadMode);
            }
            b'M' => {
                self.inner = ParserInner::Empty;
                output.push(TerminalOutput::SetCursorPosRel {
                    x: None,
                    y: Some(-1),
                });
            }
            b'D' => {
                self.inner = ParserInner::Empty;
                output.push(TerminalOutput::SetCursorPosRel {
                    x: None,
                    y: Some(1),
                });
            }
            b'E' => {
                self.inner = ParserInner::Empty;
                output.push(TerminalOutput::SetCursorPosRel {
                    x: Some(1),
                    y: Some(1),
                });
            }
            _ => {
                let char_decoded = b as char;
                error!("Unhandled escape sequence. Hex: {b:x}/Int: {b}/ASCII: {char_decoded}",);
                self.inner = ParserInner::Empty;

                return Err(ParserFailures::UnhandledInnerEscape(format!("{b:x}")).into());
            }
        }

        Ok(())
    }

    pub fn push(&mut self, incoming: &[u8]) -> Vec<TerminalOutput> {
        let mut output = Vec::new();
        let mut data_output = Vec::new();
        let mut output_string_sequence = String::new();

        for b in incoming {
            match &mut self.inner {
                ParserInner::Empty => {
                    if !output_string_sequence.is_empty() {
                        output_string_sequence.clear();
                    }

                    if self.ansi_parser_inner_empty(*b, &mut data_output, &mut output) == Err(()) {
                        continue;
                    }

                    data_output.push(*b);
                }
                ParserInner::Escape => {
                    if let Err(e) = self.ansiparser_inner_escape(*b, &mut data_output, &mut output)
                    {
                        error!("Parser Error: {e}");
                        error!("Escape Sequence that threw an error: {output_string_sequence}");
                        self.inner = ParserInner::Empty;
                    }
                }
                ParserInner::Csi(parser) => {
                    output_string_sequence.push(*b as char);
                    match parser.ansiparser_inner_csi(*b, &mut output) {
                        Ok(value) => match value {
                            Some(return_value) => {
                                self.inner = return_value;

                                // if the last value pushed to output is terminal Invalid, print out the sequence of characters that caused the error

                                if output.last() == Some(&TerminalOutput::Invalid) {
                                    error!(
                                        "CSI Sequence that threw an error: {}",
                                        output_string_sequence
                                    );
                                }
                            }
                            None => continue,
                        },
                        Err(e) => {
                            error!("Parser Error: {e}");
                            error!("CSI Sequence that threw an error: {output_string_sequence}");
                            self.inner = ParserInner::Empty;
                        }
                    }
                }
                ParserInner::Osc(parser) => {
                    output_string_sequence.push(*b as char);
                    match parser.ansiparser_inner_osc(*b, &mut output) {
                        Ok(Some(value)) => {
                            self.inner = value;

                            // if the last value pushed to output is terminal Invalid, print out the sequence of characters that caused the error

                            if output.last() == Some(&TerminalOutput::Invalid) {
                                error!(
                                    "OSC Sequence that threw an error: {output_string_sequence}",
                                );
                            }
                        }
                        Ok(None) => continue,
                        Err(e) => {
                            error!("Parser Error: {e}");
                            error!("OSC Sequence that threw an error: {output_string_sequence}");
                            self.inner = ParserInner::Empty;
                        }
                    }
                }
                ParserInner::DecSpecialGraphics(parser) => {
                    output_string_sequence.push(*b as char);
                    match parser.ansi_parser_inner_line_draw(*b, &mut output) {
                        Some(value) => {
                            self.inner = value;

                            // if the last value pushed to output is terminal Invalid, print out the sequence of characters that caused the error

                            if output.last() == Some(&TerminalOutput::Invalid) {
                                error!(
                                    "DecSpecialGraphics Sequence that threw an error: {output_string_sequence}",
                                );
                            }
                        }
                        None => continue,
                    }
                }
            }
        }

        if !data_output.is_empty() {
            output.push(TerminalOutput::Data(data_output));
        }

        output
    }
}
