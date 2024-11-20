// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use freminal_common::colors::TerminalColor;
use terminal_emulator::{
    ansi::{parse_param_as, FreminalAnsiParser, ParserInner, TerminalOutput},
    ansi_components::{
        csi::{AnsiCsiParser, AnsiCsiParserState},
        mode::{Mode, SetMode},
        modes::decckm::Decckm,
        osc::{AnsiOscInternalType, AnsiOscType},
        sgr::SelectGraphicRendition,
    },
};

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
    assert!(matches!(
        parsed[0],
        TerminalOutput::ClearDisplayfromCursortoEndofDisplay,
    ));

    let mut output_buffer = FreminalAnsiParser::new();
    let parsed = output_buffer.push(b"\x1b[0J");
    assert_eq!(parsed.len(), 1);
    assert!(matches!(
        parsed[0],
        TerminalOutput::ClearDisplayfromCursortoEndofDisplay,
    ));

    let mut output_buffer = FreminalAnsiParser::new();
    let parsed = output_buffer.push(b"\x1b[2J");
    assert_eq!(parsed.len(), 1);
    assert!(matches!(parsed[0], TerminalOutput::ClearDisplay,));
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
        parser.push(*b).unwrap();
    }

    assert_eq!(parser.params, b"0123456789:;<=>?");
    assert_eq!(parser.intermediates, b"!\"#$%&'()*+,-./");
    assert!(matches!(parser.state, AnsiCsiParserState::Finished(b'}')));

    let mut parser = AnsiCsiParser::new();
    parser.push(0x40).unwrap();

    assert_eq!(parser.params, &[]);
    assert_eq!(parser.intermediates, &[]);
    assert!(matches!(parser.state, AnsiCsiParserState::Finished(0x40)));

    let mut parser = AnsiCsiParser::new();
    parser.push(0x7e).unwrap();

    assert_eq!(parser.params, &[]);
    assert_eq!(parser.intermediates, &[]);
    assert!(matches!(parser.state, AnsiCsiParserState::Finished(0x7e)));
}

#[test]
fn test_parsing_invalid_csi() {
    let mut parser = AnsiCsiParser::new();
    for b in b"0$0" {
        parser.push(*b).unwrap();
    }

    assert!(matches!(parser.state, AnsiCsiParserState::Invalid));
    parser.push(b'm').unwrap();
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
        TerminalOutput::Mode(Mode::Unknown(b"1".to_vec()))
    );

    let output = output_buffer.push(b"\x1b[1l");
    assert_eq!(output.len(), 1);
    assert_eq!(
        output[0],
        TerminalOutput::Mode(Mode::Unknown(b"1".to_vec()))
    );

    let output = output_buffer.push(b"\x1b[?1l");
    assert_eq!(output.len(), 1);
    assert_eq!(
        output[0],
        TerminalOutput::Mode(Mode::Decckm(Decckm::new(&SetMode::Reset)))
    );

    let output = output_buffer.push(b"\x1b[?1h");
    assert_eq!(output.len(), 1);
    assert_eq!(
        output[0],
        TerminalOutput::Mode(Mode::Decckm(Decckm::new(&SetMode::Set)))
    );
}

#[test]
fn test_set_cursor_pos() {
    let mut output_buffer = FreminalAnsiParser::new();
    let output = output_buffer.push(b"\x1b[1;1H");
    assert_eq!(output.len(), 1);
    assert_eq!(
        output[0],
        TerminalOutput::SetCursorPos {
            x: Some(1),
            y: Some(1)
        }
    );

    let output = output_buffer.push(b"\x1b[;1H");
    assert_eq!(output.len(), 1);
    assert_eq!(
        output[0],
        TerminalOutput::SetCursorPos {
            x: Some(1),
            y: Some(1)
        }
    );

    let output = output_buffer.push(b"\x1b[1;H");
    assert_eq!(output.len(), 1);
    assert_eq!(
        output[0],
        TerminalOutput::SetCursorPos {
            x: Some(1),
            y: Some(1)
        }
    );

    let output = output_buffer.push(b"\x1b[H");
    assert_eq!(output.len(), 1);
    assert_eq!(
        output[0],
        TerminalOutput::SetCursorPos {
            x: Some(1),
            y: Some(1)
        }
    );

    let output = output_buffer.push(b"\x1b[;H");
    assert_eq!(output.len(), 1);
    assert_eq!(
        output[0],
        TerminalOutput::SetCursorPos {
            x: Some(1),
            y: Some(1)
        }
    );
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

    let output = TerminalOutput::ClearDisplayfromCursortoEndofDisplay;
    assert_eq!(format!("{output}"), "ClearForwards");

    let output = TerminalOutput::ClearScrollbackandDisplay;
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

    let output = TerminalOutput::Mode(Mode::Decckm(Decckm::new(&SetMode::Set)));
    assert_eq!(
        format!("{output}"),
        "SetMode(Cursor Key Mode (DECCKM) Application)"
    );

    let output = TerminalOutput::Mode(Mode::Decckm(Decckm::new(&SetMode::Reset)));
    assert_eq!(
        format!("{output}"),
        "SetMode(Cursor Key Mode (DECCKM) ANSI)"
    );

    let output = TerminalOutput::InsertSpaces(1);
    assert_eq!(format!("{output}"), "InsertSpaces(1)");

    let output = TerminalOutput::OscResponse(AnsiOscType::SetTitleBar("test".to_string()));
    assert_eq!(format!("{output}"), "OscResponse(SetTitleBar(\"test\"))");

    let output = TerminalOutput::CursorReport;
    assert_eq!(format!("{output}"), "CursorReport");

    let output = TerminalOutput::Invalid;
    assert_eq!(format!("{output}"), "Invalid");

    let output = TerminalOutput::Skipped;
    assert_eq!(format!("{output}"), "Skipped");

    let output = TerminalOutput::ApplicationKeypadMode;
    assert_eq!(format!("{output}"), "ApplicationKeypadMode");

    let output = TerminalOutput::NormalKeypadMode;
    assert_eq!(format!("{output}"), "NormalKeypadMode");
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
fn test_delete() {
    let mut output_buffer = FreminalAnsiParser::new();
    let output = output_buffer.push(b"\x1b[1P");
    assert_eq!(output.len(), 1);
    assert_eq!(output[0], TerminalOutput::Delete(1));

    let output = output_buffer.push(b"\x1b[P");
    assert_eq!(output.len(), 1);
    assert_eq!(output[0], TerminalOutput::Delete(1));

    let output = output_buffer.push(b"\x1b[10P");
    assert_eq!(output.len(), 1);
    assert_eq!(output[0], TerminalOutput::Delete(10));
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

#[test]
fn test_parse_str_fail_on_invalid_utf8() {
    // parse_param_as

    // invalid utf8
    let result: Result<Option<usize>, anyhow::Error> = parse_param_as(b"\xff");
    assert!(result.is_err());
}

#[test]
fn test_parse_str_fail_on_conversion() {
    // string that should trigger the map_or_else

    let result: Result<Option<bool>, anyhow::Error> = parse_param_as(b"123");

    assert!(result.is_err());
}

#[test]
fn test_verify_parser_state_internal_is_csi() {
    let mut parser = FreminalAnsiParser::new();
    let output = parser.push(b"\x1b[");
    assert_eq!(output.len(), 0);
    assert!(matches!(parser.inner, ParserInner::Csi(_)));
}

#[test]
fn test_verify_parser_state_internal_is_osc() {
    let mut parser = FreminalAnsiParser::new();
    let output = parser.push(b"\x1b]");
    assert_eq!(output.len(), 0);
    assert!(matches!(parser.inner, ParserInner::Osc(_)));
}

#[test]
fn test_application_keypad_support_mode() {
    let mut output_buffer = FreminalAnsiParser::new();
    let output = output_buffer.push(b"\x1b=");
    assert_eq!(output.len(), 1);
    assert_eq!(output[0], TerminalOutput::ApplicationKeypadMode);
}

#[test]
fn test_normal_keypad_support_mode() {
    let mut output_buffer = FreminalAnsiParser::new();
    let output = output_buffer.push(b"\x1b>");
    assert_eq!(output.len(), 1);
    assert_eq!(output[0], TerminalOutput::NormalKeypadMode);
}

#[test]
fn test_terminal_output_backspace() {
    let mut output_buffer = FreminalAnsiParser::new();
    let output = output_buffer.push(b"\x08");
    assert_eq!(output.len(), 1);
    assert_eq!(output[0], TerminalOutput::Backspace);
}

#[test]
fn test_terminal_output_bell() {
    let mut output_buffer = FreminalAnsiParser::new();
    let output = output_buffer.push(b"\x07");
    assert_eq!(output.len(), 1);
    assert_eq!(output[0], TerminalOutput::Bell);
}

#[test]
fn test_invalid_inner_escape() {
    let mut output_buffer = FreminalAnsiParser::new();
    let output = output_buffer.push(b"\x1b_");
    assert_eq!(output.len(), 0);
    assert!(matches!(output_buffer.inner, ParserInner::Empty));
}
