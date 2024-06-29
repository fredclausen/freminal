// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use super::Mode;
use crate::gui::terminal::lookup_256_color_by_index;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectGraphicRendition {
    // NOTE: Non-exhaustive list
    Reset,
    Bold,
    Italic,
    SlowBlink,
    FastBlink,
    ResetBold,
    NormalIntensity,
    ForegroundBlack,
    ForegroundRed,
    ForegroundGreen,
    ForegroundYellow,
    ForegroundBlue,
    ForegroundMagenta,
    ForegroundCyan,
    ForegroundWhite,
    ForegroundCustom(usize, usize, usize),
    BackgroundCustom(usize, usize, usize),
    ForegroundBrightBlack,
    ForegroundBrightRed,
    ForegroundBrightGreen,
    ForegroundBrightYellow,
    ForegroundBrightBlue,
    ForegroundBrightMagenta,
    ForegroundBrightCyan,
    ForegroundBrightWhite,
    DefaultForeground,
    Unknown(usize),
}

impl SelectGraphicRendition {
    fn from_usize(val: usize) -> Self {
        match val {
            0 => Self::Reset,
            1 => Self::Bold,
            3 => Self::Italic,
            5 => Self::SlowBlink,
            6 => Self::FastBlink,
            21 => Self::ResetBold,
            22 => Self::NormalIntensity,
            30 => Self::ForegroundBlack,
            31 => Self::ForegroundRed,
            32 => Self::ForegroundGreen,
            33 => Self::ForegroundYellow,
            34 => Self::ForegroundBlue,
            35 => Self::ForegroundMagenta,
            36 => Self::ForegroundCyan,
            37 => Self::ForegroundWhite,
            38 => {
                error!("We shouldn't end up here! Setting custom foreground color to black");
                Self::ForegroundCustom(0, 0, 0)
            }
            48 => {
                error!("We shouldn't end up here! Setting custom background color to black");
                Self::ForegroundCustom(0, 0, 0)
            }
            39 => Self::DefaultForeground,
            90 => Self::ForegroundBrightBlack,
            91 => Self::ForegroundBrightRed,
            92 => Self::ForegroundBrightGreen,
            93 => Self::ForegroundBrightYellow,
            94 => Self::ForegroundBrightBlue,
            95 => Self::ForegroundBrightMagenta,
            96 => Self::ForegroundBrightCyan,
            97 => Self::ForegroundBrightWhite,
            _ => Self::Unknown(val),
        }
    }

    const fn from_usize_color(val: usize, r: usize, g: usize, b: usize) -> Self {
        match val {
            38 => Self::ForegroundCustom(r, g, b),
            48 => Self::BackgroundCustom(r, g, b),
            _ => Self::Unknown(val),
        }
    }
}

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
    InsertLines(usize),
    Delete(usize),
    Sgr(SelectGraphicRendition),
    Data(Vec<u8>),
    SetMode(Mode),
    ResetMode(Mode),
    // ich (8.3.64 of ecma-48)
    InsertSpaces(usize),
    Invalid,
}

#[derive(Eq, PartialEq, Debug)]
enum CsiParserState {
    Params,
    Intermediates,
    Finished(u8),
    Invalid,
    InvalidFinished,
}

fn is_csi_terminator(b: u8) -> bool {
    (0x40..=0x7e).contains(&b)
}

fn is_csi_param(b: u8) -> bool {
    (0x30..=0x3f).contains(&b)
}

fn is_csi_intermediate(b: u8) -> bool {
    (0x20..=0x2f).contains(&b)
}

fn extract_param(idx: usize, params: &[Option<usize>]) -> Option<usize> {
    params.get(idx).copied().flatten()
}

fn split_params_into_semicolon_delimited_usize(params: &[u8]) -> Result<Vec<Option<usize>>, ()> {
    let params = params
        .split(|b| *b == b';')
        .map(parse_param_as::<usize>)
        .collect::<Result<Vec<Option<usize>>, ()>>();

    params
}

fn parse_param_as<T: std::str::FromStr>(param_bytes: &[u8]) -> Result<Option<T>, ()> {
    let param_str =
        std::str::from_utf8(param_bytes).expect("parameter should always be valid utf8");
    if param_str.is_empty() {
        return Ok(None);
    }
    let param = param_str.parse().map_err(|_| ())?;
    Ok(Some(param))
}

fn push_data_if_non_empty(data: &mut Vec<u8>, output: &mut Vec<TerminalOutput>) {
    if !data.is_empty() {
        output.push(TerminalOutput::Data(std::mem::take(data)));
    }
}

fn mode_from_params(params: &[u8]) -> Mode {
    match params {
        // https://vt100.net/docs/vt510-rm/DECCKM.html
        b"?1" => Mode::Decckm,
        _ => Mode::Unknown(params.to_vec()),
    }
}

#[derive(Eq, PartialEq, Debug)]
struct CsiParser {
    state: CsiParserState,
    params: Vec<u8>,
    intermediates: Vec<u8>,
}

impl CsiParser {
    fn new() -> Self {
        Self {
            state: CsiParserState::Params,
            params: Vec::new(),
            intermediates: Vec::new(),
        }
    }

    fn push(&mut self, b: u8) {
        if let CsiParserState::Finished(_) | CsiParserState::InvalidFinished = &self.state {
            panic!("CsiParser should not be pushed to once finished");
        }

        match &mut self.state {
            CsiParserState::Params => {
                if is_csi_param(b) {
                    self.params.push(b);
                } else if is_csi_intermediate(b) {
                    self.intermediates.push(b);
                    self.state = CsiParserState::Intermediates;
                } else if is_csi_terminator(b) {
                    self.state = CsiParserState::Finished(b);
                } else {
                    self.state = CsiParserState::Invalid;
                }
            }
            CsiParserState::Intermediates => {
                if is_csi_param(b) {
                    self.state = CsiParserState::Invalid;
                } else if is_csi_intermediate(b) {
                    self.intermediates.push(b);
                } else if is_csi_terminator(b) {
                    self.state = CsiParserState::Finished(b);
                } else {
                    self.state = CsiParserState::Invalid;
                }
            }
            CsiParserState::Invalid => {
                if is_csi_terminator(b) {
                    self.state = CsiParserState::InvalidFinished;
                }
            }
            CsiParserState::Finished(_) | CsiParserState::InvalidFinished => {
                unreachable!();
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
enum AnsiParserInner {
    Empty,
    Escape,
    Csi(CsiParser),
}

pub struct FreminalAnsiParser {
    inner: AnsiParserInner,
}

impl FreminalAnsiParser {
    pub const fn new() -> Self {
        Self {
            inner: AnsiParserInner::Empty,
        }
    }

    pub fn push(&mut self, incoming: &[u8]) -> Vec<TerminalOutput> {
        let mut output = Vec::new();
        let mut data_output = Vec::new();
        for b in incoming {
            match &mut self.inner {
                AnsiParserInner::Empty => {
                    if *b == b'\x1b' {
                        self.inner = AnsiParserInner::Escape;
                        continue;
                    }

                    if *b == b'\r' {
                        push_data_if_non_empty(&mut data_output, &mut output);
                        output.push(TerminalOutput::CarriageReturn);
                        continue;
                    }

                    if *b == b'\n' {
                        push_data_if_non_empty(&mut data_output, &mut output);
                        output.push(TerminalOutput::Newline);
                        continue;
                    }

                    if *b == 0x08 {
                        push_data_if_non_empty(&mut data_output, &mut output);
                        output.push(TerminalOutput::Backspace);
                        continue;
                    }

                    data_output.push(*b);
                }
                AnsiParserInner::Escape => {
                    push_data_if_non_empty(&mut data_output, &mut output);

                    if b == &b'[' {
                        self.inner = AnsiParserInner::Csi(CsiParser::new());
                    } else {
                        let b_utf8 = std::char::from_u32(u32::from(*b));
                        warn!("Unhandled escape sequence {b_utf8:?} {b:x}");
                        self.inner = AnsiParserInner::Empty;
                    }
                }
                AnsiParserInner::Csi(parser) => {
                    parser.push(*b);
                    match parser.state {
                        CsiParserState::Finished(b'A') => {
                            let Ok(param) = parse_param_as::<i32>(&parser.params) else {
                                warn!("Invalid cursor move up distance");
                                output.push(TerminalOutput::Invalid);
                                self.inner = AnsiParserInner::Empty;
                                continue;
                            };

                            output.push(TerminalOutput::SetCursorPosRel {
                                x: None,
                                y: Some(-param.unwrap_or(1)),
                            });
                            self.inner = AnsiParserInner::Empty;
                        }
                        CsiParserState::Finished(b'B') => {
                            let Ok(param) = parse_param_as::<i32>(&parser.params) else {
                                warn!("Invalid cursor move down distance");
                                output.push(TerminalOutput::Invalid);
                                self.inner = AnsiParserInner::Empty;
                                continue;
                            };

                            output.push(TerminalOutput::SetCursorPosRel {
                                x: None,
                                y: Some(param.unwrap_or(1)),
                            });
                            self.inner = AnsiParserInner::Empty;
                        }
                        CsiParserState::Finished(b'C') => {
                            let Ok(param) = parse_param_as::<i32>(&parser.params) else {
                                warn!("Invalid cursor move right distance");
                                output.push(TerminalOutput::Invalid);
                                self.inner = AnsiParserInner::Empty;
                                continue;
                            };

                            output.push(TerminalOutput::SetCursorPosRel {
                                x: Some(param.unwrap_or(1)),
                                y: None,
                            });
                            self.inner = AnsiParserInner::Empty;
                        }
                        CsiParserState::Finished(b'D') => {
                            let Ok(param) = parse_param_as::<i32>(&parser.params) else {
                                warn!("Invalid cursor move left distance");
                                output.push(TerminalOutput::Invalid);
                                self.inner = AnsiParserInner::Empty;
                                continue;
                            };

                            output.push(TerminalOutput::SetCursorPosRel {
                                x: Some(-param.unwrap_or(1)),
                                y: None,
                            });
                            self.inner = AnsiParserInner::Empty;
                        }
                        CsiParserState::Finished(b'H') => {
                            let params =
                                split_params_into_semicolon_delimited_usize(&parser.params);

                            let Ok(params) = params else {
                                warn!("Invalid cursor set position sequence");
                                output.push(TerminalOutput::Invalid);
                                self.inner = AnsiParserInner::Empty;
                                continue;
                            };

                            output.push(TerminalOutput::SetCursorPos {
                                x: Some(extract_param(1, &params).unwrap_or(1)),
                                y: Some(extract_param(0, &params).unwrap_or(1)),
                            });
                            self.inner = AnsiParserInner::Empty;
                        }
                        CsiParserState::Finished(b'G') => {
                            let Ok(param) = parse_param_as::<usize>(&parser.params) else {
                                warn!("Invalid cursor set position sequence");
                                output.push(TerminalOutput::Invalid);
                                self.inner = AnsiParserInner::Empty;
                                continue;
                            };

                            let x_pos = param.unwrap_or(1);

                            output.push(TerminalOutput::SetCursorPos {
                                x: Some(x_pos),
                                y: None,
                            });
                            self.inner = AnsiParserInner::Empty;
                        }
                        CsiParserState::Finished(b'J') => {
                            let Ok(param) = parse_param_as::<usize>(&parser.params) else {
                                warn!("Invalid clear command");
                                output.push(TerminalOutput::Invalid);
                                self.inner = AnsiParserInner::Empty;
                                continue;
                            };

                            let ret = match param.unwrap_or(0) {
                                0 => TerminalOutput::ClearForwards,
                                2 | 3 => TerminalOutput::ClearAll,
                                _ => TerminalOutput::Invalid,
                            };
                            output.push(ret);
                            self.inner = AnsiParserInner::Empty;
                        }
                        CsiParserState::Finished(b'K') => {
                            let Ok(param) = parse_param_as::<usize>(&parser.params) else {
                                warn!("Invalid erase in line command");
                                output.push(TerminalOutput::Invalid);
                                self.inner = AnsiParserInner::Empty;
                                continue;
                            };

                            // ECMA-48 8.3.39
                            match param.unwrap_or(0) {
                                0 => output.push(TerminalOutput::ClearLineForwards),
                                v => {
                                    warn!("Unsupported erase in line command ({v})");
                                    output.push(TerminalOutput::Invalid);
                                }
                            }

                            self.inner = AnsiParserInner::Empty;
                        }
                        CsiParserState::Finished(b'L') => {
                            let Ok(param) = parse_param_as::<usize>(&parser.params) else {
                                warn!("Invalid il command");
                                output.push(TerminalOutput::Invalid);
                                self.inner = AnsiParserInner::Empty;
                                continue;
                            };

                            output.push(TerminalOutput::InsertLines(param.unwrap_or(1)));

                            self.inner = AnsiParserInner::Empty;
                        }
                        CsiParserState::Finished(b'P') => {
                            let Ok(param) = parse_param_as::<usize>(&parser.params) else {
                                warn!("Invalid del command");
                                output.push(TerminalOutput::Invalid);
                                self.inner = AnsiParserInner::Empty;
                                continue;
                            };

                            output.push(TerminalOutput::Delete(param.unwrap_or(1)));

                            self.inner = AnsiParserInner::Empty;
                        }
                        CsiParserState::Finished(b'm') => {
                            let params =
                                split_params_into_semicolon_delimited_usize(&parser.params);

                            let Ok(mut params) = params else {
                                warn!("Invalid SGR sequence");
                                output.push(TerminalOutput::Invalid);
                                self.inner = AnsiParserInner::Empty;
                                continue;
                            };

                            if params.is_empty() {
                                params.push(Some(0));
                            }

                            if params.len() == 1 && params[0].is_none() {
                                params[0] = Some(0);
                            }

                            let mut param_iter = params.into_iter();
                            loop {
                                let param = param_iter.next();
                                let Some(mut param) = param.unwrap_or(None) else {
                                    break;
                                };

                                // if control code is 38 or 48, we need to read the next param
                                // otherwise, store the param as is

                                if param == 38 || param == 48 {
                                    let custom_color_control_code = param;
                                    let custom_color_r: usize;
                                    let custom_color_g: usize;
                                    let custom_color_b: usize;

                                    param = if let Some(Some(param)) = param_iter.next() {
                                        param
                                    } else {
                                        warn!("Invalid SGR sequence: {}", param);
                                        output.push(TerminalOutput::Invalid);
                                        continue;
                                    };

                                    match param {
                                        2 => {
                                            custom_color_r =
                                                if let Some(Some(param)) = param_iter.next() {
                                                    param
                                                } else {
                                                    warn!("Invalid SGR sequence: {}", param);
                                                    output.push(TerminalOutput::Invalid);
                                                    continue;
                                                };
                                            custom_color_g =
                                                if let Some(Some(param)) = param_iter.next() {
                                                    param
                                                } else {
                                                    warn!("Invalid SGR sequence: {}", param);
                                                    output.push(TerminalOutput::Invalid);
                                                    continue;
                                                };
                                            custom_color_b =
                                                if let Some(Some(param)) = param_iter.next() {
                                                    param
                                                } else {
                                                    warn!("Invalid SGR sequence: {}", param);
                                                    output.push(TerminalOutput::Invalid);
                                                    continue;
                                                };

                                            // lets make sure the iterator is empty now. Otherwise, it's an invalid sequence
                                            if param_iter.next().is_some() {
                                                warn!("Invalid SGR sequence: {}", param);
                                                output.push(TerminalOutput::Invalid);
                                                continue;
                                            }
                                        }
                                        5 => {
                                            let Some(Some(lookup)) = param_iter.next() else {
                                                warn!("Invalid SGR sequence: {}", param);
                                                output.push(TerminalOutput::Invalid);
                                                continue;
                                            };

                                            // lets make sure the iterator is empty now. Otherwise, it's an invalid sequence
                                            if param_iter.next().is_some() {
                                                warn!("Invalid SGR sequence: {}", param);
                                                output.push(TerminalOutput::Invalid);
                                                continue;
                                            }

                                            // look up the rgb

                                            (custom_color_r, custom_color_g, custom_color_b) =
                                                lookup_256_color_by_index(lookup);
                                        }
                                        _ => {
                                            warn!("Invalid SGR sequence: {}", param);
                                            output.push(TerminalOutput::Invalid);
                                            continue;
                                        }
                                    }

                                    output.push(TerminalOutput::Sgr(
                                        SelectGraphicRendition::from_usize_color(
                                            custom_color_control_code,
                                            custom_color_r,
                                            custom_color_g,
                                            custom_color_b,
                                        ),
                                    ));
                                    continue;
                                }

                                output.push(TerminalOutput::Sgr(
                                    SelectGraphicRendition::from_usize(param),
                                ));
                            }

                            self.inner = AnsiParserInner::Empty;
                        }
                        CsiParserState::Finished(b'h') => {
                            output.push(TerminalOutput::SetMode(mode_from_params(&parser.params)));
                            self.inner = AnsiParserInner::Empty;
                        }
                        CsiParserState::Finished(b'l') => {
                            output
                                .push(TerminalOutput::ResetMode(mode_from_params(&parser.params)));
                            self.inner = AnsiParserInner::Empty;
                        }
                        CsiParserState::Finished(b'@') => {
                            let Ok(param) = parse_param_as::<usize>(&parser.params) else {
                                warn!("Invalid ich command");
                                output.push(TerminalOutput::Invalid);
                                self.inner = AnsiParserInner::Empty;
                                continue;
                            };

                            // ecma-48 8.3.64
                            output.push(TerminalOutput::InsertSpaces(param.unwrap_or(1)));
                            self.inner = AnsiParserInner::Empty;
                        }
                        CsiParserState::Finished(esc) => {
                            warn!(
                                "Unhandled csi code: {:?} {esc:x} {}/{}",
                                std::char::from_u32(u32::from(esc)),
                                esc >> 4,
                                esc & 0xf,
                            );
                            output.push(TerminalOutput::Invalid);
                            self.inner = AnsiParserInner::Empty;
                        }
                        CsiParserState::Invalid => {
                            warn!("Invalid CSI sequence");
                            output.push(TerminalOutput::Invalid);
                            self.inner = AnsiParserInner::Empty;
                        }
                        _ => {}
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
    use super::*;

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
        let mut parser = CsiParser::new();
        for b in b"0123456789:;<=>?!\"#$%&'()*+,-./}" {
            parser.push(*b);
        }

        assert_eq!(parser.params, b"0123456789:;<=>?");
        assert_eq!(parser.intermediates, b"!\"#$%&'()*+,-./");
        assert!(matches!(parser.state, CsiParserState::Finished(b'}')));

        let mut parser = CsiParser::new();
        parser.push(0x40);

        assert_eq!(parser.params, &[]);
        assert_eq!(parser.intermediates, &[]);
        assert!(matches!(parser.state, CsiParserState::Finished(0x40)));

        let mut parser = CsiParser::new();
        parser.push(0x7e);

        assert_eq!(parser.params, &[]);
        assert_eq!(parser.intermediates, &[]);
        assert!(matches!(parser.state, CsiParserState::Finished(0x7e)));
    }

    #[test]
    fn test_parsing_invalid_csi() {
        let mut parser = CsiParser::new();
        for b in b"0$0" {
            parser.push(*b);
        }

        assert!(matches!(parser.state, CsiParserState::Invalid));
        parser.push(b'm');
        assert!(matches!(parser.state, CsiParserState::InvalidFinished));
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

        struct ColorCode(u8);

        impl std::fmt::Display for ColorCode {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_fmt(format_args!("\x1b[{}m", self.0))
            }
        }

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
                TerminalOutput::Sgr(SelectGraphicRendition::ForegroundBlack),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::ForegroundRed),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::ForegroundGreen),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::ForegroundYellow),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::ForegroundBlue),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::ForegroundMagenta),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::ForegroundCyan),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::ForegroundWhite),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::ForegroundBrightBlack),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::ForegroundBrightRed),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::ForegroundBrightGreen),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::ForegroundBrightYellow),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::ForegroundBrightBlue),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::ForegroundBrightMagenta),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::ForegroundBrightCyan),
                TerminalOutput::Data(b"a".into()),
                TerminalOutput::Sgr(SelectGraphicRendition::ForegroundBrightWhite),
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
}
