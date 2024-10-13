// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use super::{
    ansi_components::{
        csi::AnsiCsiParser,
        osc::{AnsiOscParser, AnsiOscType},
        sgr::SelectGraphicRendition,
    },
    error::ParserFailures,
    Mode,
};

use anyhow::Result;

#[derive(Debug, Eq, PartialEq)]
pub enum TerminalOutput {
    SetCursorPos { x: Option<usize>, y: Option<usize> },
    SetCursorPosRel { x: Option<i32>, y: Option<i32> },
    ClearForwards,
    ClearAll,
    CarriageReturn,
    ClearLineForwards,
    Newline,
    Backspace,
    Bell,
    InsertLines(usize),
    Delete(usize),
    Sgr(SelectGraphicRendition),
    Data(Vec<u8>),
    SetMode(Mode),
    ResetMode(Mode),
    // ich (8.3.64 of ecma-48)
    InsertSpaces(usize),
    OscResponse(AnsiOscType),
    CursorReport,
    Invalid,
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
            Self::ClearForwards => write!(f, "ClearForwards"),
            Self::ClearAll => write!(f, "ClearAll"),
            Self::CarriageReturn => write!(f, "CarriageReturn"),
            Self::ClearLineForwards => write!(f, "ClearLineForwards"),
            Self::Newline => write!(f, "Newline"),
            Self::Backspace => write!(f, "Backspace"),
            Self::Bell => write!(f, "Bell"),
            Self::InsertLines(n) => write!(f, "InsertLines({n})"),
            Self::Delete(n) => write!(f, "Delete({n})"),
            Self::Sgr(sgr) => write!(f, "Sgr({sgr:?})"),
            Self::Data(data) => {
                write!(f, "Data({})", String::from_utf8_lossy(data))
            }
            Self::SetMode(mode) => write!(f, "SetMode({mode:?})"),
            Self::ResetMode(mode) => write!(f, "ResetMode({mode:?})"),
            Self::InsertSpaces(n) => write!(f, "InsertSpaces({n})"),
            Self::OscResponse(n) => write!(f, "OscResponse({n})"),
            Self::Invalid => write!(f, "Invalid"),
            Self::CursorReport => write!(f, "CursorReport"),
        }
    }
}

pub fn extract_param(idx: usize, params: &[Option<usize>]) -> Option<usize> {
    params.get(idx).copied().flatten()
}

pub fn split_params_into_semicolon_delimited_usize(
    params: &[u8],
) -> Result<Vec<Option<usize>>, ()> {
    let params = params
        .split(|b| *b == b';')
        .map(parse_param_as::<usize>)
        .collect::<Result<Vec<Option<usize>>, ()>>();

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
}

pub struct FreminalAnsiParser {
    pub(crate) inner: ParserInner,
}

impl FreminalAnsiParser {
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

        if b == b'[' {
            self.inner = ParserInner::Csi(AnsiCsiParser::new());
        } else if b == b']' {
            self.inner = ParserInner::Osc(AnsiOscParser::new());
        } else {
            error!("Unhandled escape sequence {b:x}");
            self.inner = ParserInner::Empty;

            return Err(ParserFailures::UnhandledInnerEscape(format!("{b:x}")).into());
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
                        Err(()) => continue,
                    }
                }
                ParserInner::Osc(parser) => {
                    output_string_sequence.push(*b as char);
                    match parser.ansiparser_inner_osc(*b, &mut output) {
                        Some(value) => {
                            self.inner = value;

                            // if the last value pushed to output is terminal Invalid, print out the sequence of characters that caused the error

                            if output.last() == Some(&TerminalOutput::Invalid) {
                                error!(
                                    "OSC Sequence that threw an error: {output_string_sequence}",
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

#[cfg(test)]
mod test {
    use crate::terminal_emulator::{
        ansi_components::{csi::AnsiCsiParserState, osc::AnsiOscInternalType},
        TerminalColor,
    };

    use super::*;

    struct ColorCode(u8);

    impl std::fmt::Display for ColorCode {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_fmt(format_args!("\x1b[{}m", self.0))
        }
    }

    #[test]
    fn test_set_cursor_position() {
        let mut output_buffer = FreminalAnsiParser::new();
        let parsed = output_buffer.push(b"\x1b[32;15H");
        assert_eq!(parsed.len(), 1);
        assert!(matches!(
            parsed[0],
            TerminalOutput::SetCursorPos {
                y: Some(32),
                x: Some(15)
            }
        ));

        let parsed = output_buffer.push(b"\x1b[;32H");
        assert_eq!(parsed.len(), 1);
        assert!(matches!(
            parsed[0],
            TerminalOutput::SetCursorPos {
                y: Some(1),
                x: Some(32)
            }
        ));

        let parsed = output_buffer.push(b"\x1b[32H");
        assert_eq!(parsed.len(), 1);
        assert!(matches!(
            parsed[0],
            TerminalOutput::SetCursorPos {
                y: Some(32),
                x: Some(1)
            }
        ));

        let parsed = output_buffer.push(b"\x1b[32;H");
        assert_eq!(parsed.len(), 1);
        assert!(matches!(
            parsed[0],
            TerminalOutput::SetCursorPos {
                y: Some(32),
                x: Some(1)
            }
        ));

        let parsed = output_buffer.push(b"\x1b[H");
        assert_eq!(parsed.len(), 1);
        assert!(matches!(
            parsed[0],
            TerminalOutput::SetCursorPos {
                y: Some(1),
                x: Some(1)
            }
        ));

        let parsed = output_buffer.push(b"\x1b[;H");
        assert_eq!(parsed.len(), 1);
        assert!(matches!(
            parsed[0],
            TerminalOutput::SetCursorPos {
                y: Some(1),
                x: Some(1)
            }
        ));
    }

    #[test]
    fn test_clear() {
        let mut output_buffer = FreminalAnsiParser::new();
        let parsed = output_buffer.push(b"\x1b[J");
        assert_eq!(parsed.len(), 1);
        assert!(matches!(parsed[0], TerminalOutput::ClearForwards,));

        let mut output_buffer = FreminalAnsiParser::new();
        let parsed = output_buffer.push(b"\x1b[0J");
        assert_eq!(parsed.len(), 1);
        assert!(matches!(parsed[0], TerminalOutput::ClearForwards,));

        let mut output_buffer = FreminalAnsiParser::new();
        let parsed = output_buffer.push(b"\x1b[2J");
        assert_eq!(parsed.len(), 1);
        assert!(matches!(parsed[0], TerminalOutput::ClearAll,));
    }

    #[test]
    fn test_invalid_clear() {
        let mut output_buffer = FreminalAnsiParser::new();
        let parsed = output_buffer.push(b"\x1b[8J");
        assert_eq!(parsed.len(), 1);
        assert!(matches!(parsed[0], TerminalOutput::Invalid,));
    }

    #[test]
    fn test_invalid_csi() {
        let mut output_buffer = FreminalAnsiParser::new();
        let parsed = output_buffer.push(b"\x1b[-23;H");
        assert!(matches!(parsed[0], TerminalOutput::Invalid));

        let mut output_buffer = FreminalAnsiParser::new();
        let parsed = output_buffer.push(b"\x1b[asdf");
        assert!(matches!(parsed[0], TerminalOutput::Invalid));
    }

    #[test]
    fn test_parsing_unknown_csi() {
        let mut parser = AnsiCsiParser::new();
        for b in b"0123456789:;<=>?!\"#$%&'()*+,-./}" {
            parser.push(*b);
        }

        assert_eq!(parser.params, b"0123456789:;<=>?");
        assert_eq!(parser.intermediates, b"!\"#$%&'()*+,-./");
        assert!(matches!(parser.state, AnsiCsiParserState::Finished(b'}')));

        let mut parser = AnsiCsiParser::new();
        parser.push(0x40);

        assert_eq!(parser.params, &[]);
        assert_eq!(parser.intermediates, &[]);
        assert!(matches!(parser.state, AnsiCsiParserState::Finished(0x40)));

        let mut parser = AnsiCsiParser::new();
        parser.push(0x7e);

        assert_eq!(parser.params, &[]);
        assert_eq!(parser.intermediates, &[]);
        assert!(matches!(parser.state, AnsiCsiParserState::Finished(0x7e)));
    }

    #[test]
    fn test_parsing_invalid_csi() {
        let mut parser = AnsiCsiParser::new();
        for b in b"0$0" {
            parser.push(*b);
        }

        assert!(matches!(parser.state, AnsiCsiParserState::Invalid));
        parser.push(b'm');
        assert!(matches!(parser.state, AnsiCsiParserState::InvalidFinished));
    }

    #[test]
    fn test_empty_sgr() {
        let mut output_buffer = FreminalAnsiParser::new();
        let parsed = output_buffer.push(b"\x1b[m");
        assert!(matches!(
            parsed[0],
            TerminalOutput::Sgr(SelectGraphicRendition::Reset)
        ));
    }

    #[test]
    fn test_color_parsing() {
        let mut output_buffer = FreminalAnsiParser::new();

        let mut test_input = String::new();
        for i in 30..=37 {
            test_input.push_str(&ColorCode(i).to_string());
            test_input.push('a');
        }

        for i in 90..=97 {
            test_input.push_str(&ColorCode(i).to_string());
            test_input.push('a');
        }

        let output = output_buffer.push(test_input.as_bytes());
        assert_eq!(
            output,
            &[
                TerminalOutput::Sgr(SelectGraphicRendition::Foreground(TerminalColor::Black)),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::Foreground(TerminalColor::Red)),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::Foreground(TerminalColor::Green)),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::Foreground(TerminalColor::Yellow)),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::Foreground(TerminalColor::Blue)),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::Foreground(TerminalColor::Magenta)),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::Foreground(TerminalColor::Cyan)),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::Foreground(TerminalColor::White)),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::Foreground(
                    TerminalColor::BrightBlack
                )),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::Foreground(TerminalColor::BrightRed)),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::Foreground(
                    TerminalColor::BrightGreen
                )),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::Foreground(
                    TerminalColor::BrightYellow
                )),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::Foreground(
                    TerminalColor::BrightBlue
                )),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::Foreground(
                    TerminalColor::BrightMagenta
                )),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::Foreground(
                    TerminalColor::BrightCyan
                )),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::Foreground(
                    TerminalColor::BrightWhite
                )),
                TerminalOutput::Data(b"a".into()),
            ]
        );
    }

    #[test]
    fn test_mode_parsing() {
        let mut output_buffer = FreminalAnsiParser::new();
        let output = output_buffer.push(b"\x1b[1h");
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0],
            TerminalOutput::SetMode(Mode::Unknown(b"1".to_vec()))
        );

        let output = output_buffer.push(b"\x1b[1l");
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0],
            TerminalOutput::ResetMode(Mode::Unknown(b"1".to_vec()))
        );

        let output = output_buffer.push(b"\x1b[?1l");
        assert_eq!(output.len(), 1);
        assert_eq!(output[0], TerminalOutput::ResetMode(Mode::Decckm));

        let output = output_buffer.push(b"\x1b[?1h");
        assert_eq!(output.len(), 1);
        assert_eq!(output[0], TerminalOutput::SetMode(Mode::Decckm));
    }

    #[test]
    fn test_rel_move_up_parsing() {
        let mut output_buffer = FreminalAnsiParser::new();
        let output = output_buffer.push(b"\x1b[1A");
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0],
            TerminalOutput::SetCursorPosRel {
                x: None,
                y: Some(-1)
            }
        );

        let output = output_buffer.push(b"\x1b[A");
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0],
            TerminalOutput::SetCursorPosRel {
                x: None,
                y: Some(-1)
            }
        );

        let output = output_buffer.push(b"\x1b[10A");
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0],
            TerminalOutput::SetCursorPosRel {
                x: None,
                y: Some(-10)
            }
        );
    }

    #[test]
    fn test_rel_move_down_parsing() {
        let mut output_buffer = FreminalAnsiParser::new();
        let output = output_buffer.push(b"\x1b[1B");
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0],
            TerminalOutput::SetCursorPosRel {
                x: None,
                y: Some(1)
            }
        );

        let output = output_buffer.push(b"\x1b[B");
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0],
            TerminalOutput::SetCursorPosRel {
                x: None,
                y: Some(1)
            }
        );

        let output = output_buffer.push(b"\x1b[10B");
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0],
            TerminalOutput::SetCursorPosRel {
                x: None,
                y: Some(10)
            }
        );
    }

    #[test]
    fn test_rel_move_right_parsing() {
        let mut output_buffer = FreminalAnsiParser::new();
        let output = output_buffer.push(b"\x1b[1C");
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0],
            TerminalOutput::SetCursorPosRel {
                y: None,
                x: Some(1)
            }
        );

        let output = output_buffer.push(b"\x1b[C");
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0],
            TerminalOutput::SetCursorPosRel {
                y: None,
                x: Some(1)
            }
        );

        let output = output_buffer.push(b"\x1b[10C");
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0],
            TerminalOutput::SetCursorPosRel {
                y: None,
                x: Some(10)
            }
        );
    }

    #[test]
    fn test_rel_move_left_parsing() {
        let mut output_buffer = FreminalAnsiParser::new();
        let output = output_buffer.push(b"\x1b[1D");
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0],
            TerminalOutput::SetCursorPosRel {
                y: None,
                x: Some(-1)
            }
        );

        let output = output_buffer.push(b"\x1b[D");
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0],
            TerminalOutput::SetCursorPosRel {
                y: None,
                x: Some(-1)
            }
        );

        let output = output_buffer.push(b"\x1b[10D");
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0],
            TerminalOutput::SetCursorPosRel {
                y: None,
                x: Some(-10)
            }
        );
    }

    #[test]
    fn test_fmt_display_terminal_output() {
        let output = TerminalOutput::SetCursorPos {
            x: Some(1),
            y: Some(1),
        };
        assert_eq!(format!("{output}"), "SetCursorPos: x: Some(1), y: Some(1)");

        let output = TerminalOutput::SetCursorPosRel {
            x: Some(1),
            y: Some(1),
        };
        assert_eq!(
            format!("{output}"),
            "SetCursorPosRel: x: Some(1), y: Some(1)"
        );

        let output = TerminalOutput::ClearForwards;
        assert_eq!(format!("{output}"), "ClearForwards");

        let output = TerminalOutput::ClearAll;
        assert_eq!(format!("{output}"), "ClearAll");

        let output = TerminalOutput::CarriageReturn;
        assert_eq!(format!("{output}"), "CarriageReturn");

        let output = TerminalOutput::ClearLineForwards;
        assert_eq!(format!("{output}"), "ClearLineForwards");

        let output = TerminalOutput::Newline;
        assert_eq!(format!("{output}"), "Newline");

        let output = TerminalOutput::Backspace;
        assert_eq!(format!("{output}"), "Backspace");

        let output = TerminalOutput::Bell;
        assert_eq!(format!("{output}"), "Bell");

        let output = TerminalOutput::InsertLines(1);
        assert_eq!(format!("{output}"), "InsertLines(1)");

        let output = TerminalOutput::Delete(1);
        assert_eq!(format!("{output}"), "Delete(1)");

        let output = TerminalOutput::Sgr(SelectGraphicRendition::Reset);
        assert_eq!(format!("{output}"), "Sgr(Reset)");

        let output = TerminalOutput::Data(b"test".to_vec());
        assert_eq!(format!("{output}"), "Data(test)");

        let output = TerminalOutput::SetMode(Mode::Decckm);
        assert_eq!(format!("{output}"), "SetMode(Decckm)");

        let output = TerminalOutput::ResetMode(Mode::Decckm);
        assert_eq!(format!("{output}"), "ResetMode(Decckm)");

        let output = TerminalOutput::InsertSpaces(1);
        assert_eq!(format!("{output}"), "InsertSpaces(1)");

        let output = TerminalOutput::OscResponse(AnsiOscType::SetTitleBar("test".to_string()));
        assert_eq!(format!("{output}"), "OscResponse(SetTitleBar(\"test\"))");

        let output = TerminalOutput::CursorReport;
        assert_eq!(format!("{output}"), "CursorReport");

        let output = TerminalOutput::Invalid;
        assert_eq!(format!("{output}"), "Invalid");
    }

    #[test]
    fn test_osc_response() {
        let mut output_buffer = FreminalAnsiParser::new();
        let output = output_buffer.push(b"\x1b]0;test\x07");
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0],
            TerminalOutput::OscResponse(AnsiOscType::SetTitleBar("test".to_string()))
        );

        // test the FTCS
        let output = output_buffer.push(b"\x1b]133;test\x07");
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0],
            TerminalOutput::OscResponse(AnsiOscType::Ftcs("test".to_string()))
        );

        // test the background color query
        let output = output_buffer.push(b"\x1b]11;?\x07");
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0],
            TerminalOutput::OscResponse(AnsiOscType::RequestColorQueryBackground(
                AnsiOscInternalType::Query
            ))
        );

        // test the foreground color query
        let output = output_buffer.push(b"\x1b]10;?\x07");
        assert_eq!(output.len(), 1);
        assert_eq!(
            output[0],
            TerminalOutput::OscResponse(AnsiOscType::RequestColorQueryForeground(
                AnsiOscInternalType::Query
            ))
        );
    }

    #[test]
    fn test_insert_lines() {
        let mut output_buffer = FreminalAnsiParser::new();
        let output = output_buffer.push(b"\x1b[1L");
        assert_eq!(output.len(), 1);
        assert_eq!(output[0], TerminalOutput::InsertLines(1));

        let output = output_buffer.push(b"\x1b[L");
        assert_eq!(output.len(), 1);
        assert_eq!(output[0], TerminalOutput::InsertLines(1));

        let output = output_buffer.push(b"\x1b[10L");
        assert_eq!(output.len(), 1);
        assert_eq!(output[0], TerminalOutput::InsertLines(10));
    }

    #[test]
    fn test_insert_spaces() {
        let mut output_buffer = FreminalAnsiParser::new();
        let output = output_buffer.push(b"\x1b[1@");
        assert_eq!(output.len(), 1);
        assert_eq!(output[0], TerminalOutput::InsertSpaces(1));

        let output = output_buffer.push(b"\x1b[@");
        assert_eq!(output.len(), 1);
        assert_eq!(output[0], TerminalOutput::InsertSpaces(1));

        let output = output_buffer.push(b"\x1b[10@");
        assert_eq!(output.len(), 1);
        assert_eq!(output[0], TerminalOutput::InsertSpaces(10));
    }
}
