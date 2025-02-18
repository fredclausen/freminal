// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use test_log::test;

use freminal_common::{
    colors::TerminalColor, cursor::CursorVisualStyle, window_manipulation::WindowManipulation,
};
use freminal_terminal_emulator::{
    ansi::{split_params_into_semicolon_delimited_usize, ParserInner, TerminalOutput},
    ansi_components::{
        csi_commands::{
            cha::ansi_parser_inner_csi_finished_set_cursor_position_g,
            cub::ansi_parser_inner_csi_finished_move_cursor_left,
            cud::ansi_parser_inner_csi_finished_move_down,
            cuf::ansi_parser_inner_csi_finished_move_right,
            cup::ansi_parser_inner_csi_finished_set_position_h,
            cuu::ansi_parser_inner_csi_finished_move_up,
            dch::ansi_parser_inner_csi_finished_set_position_p,
            decrqm::ansi_parser_inner_csi_finished_decrqm,
            decscusr::ansi_parser_inner_csi_finished_set_position_q,
            decslpp::ansi_parser_inner_csi_finished_set_position_t,
            decstbm::ansi_parser_inner_csi_set_top_and_bottom_margins,
            ech::ansi_parser_inner_csi_finished_set_position_x,
            ed::ansi_parser_inner_csi_finished_set_position_j,
            el::ansi_parser_inner_csi_finished_set_position_k,
            ict::ansi_parser_inner_csi_finished_ich,
            il::ansi_parser_inner_csi_finished_set_position_l,
            send_device_attributes::ansi_parser_inner_csi_finished_send_da,
            sgr::{ansi_parser_inner_csi_finished_sgr_ansi, handle_custom_color},
        },
        mode::Mode,
        modes::decckm::Decckm,
        sgr::SelectGraphicRendition,
    },
};

#[test]
fn test_cha() {
    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_set_cursor_position_g(b"1", &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPos {
            x: Some(1),
            y: None
        }]
    );

    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_set_cursor_position_g(b"2", &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPos {
            x: Some(2),
            y: None
        }]
    );

    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_set_cursor_position_g(b"3", &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPos {
            x: Some(3),
            y: None
        }]
    );

    // test invalid
    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_finished_set_cursor_position_g(b"test", &mut output);
    assert!(result.is_err());
    assert_eq!(output, vec![TerminalOutput::Invalid]);
}

#[test]
fn test_cub() {
    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_move_cursor_left(&[], &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPosRel {
            x: Some(-1),
            y: None
        }]
    );

    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_move_cursor_left(b"1", &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPosRel {
            x: Some(-1),
            y: None
        }]
    );

    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_move_cursor_left(b"2", &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPosRel {
            x: Some(-2),
            y: None
        }]
    );

    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_finished_move_cursor_left(b"test", &mut output);
    assert_eq!(output, vec![TerminalOutput::Invalid]);
    assert!(result.is_err());
}

#[test]
fn test_cud() {
    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_move_down(&[], &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPosRel {
            x: None,
            y: Some(1)
        }]
    );

    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_move_down(b"1", &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPosRel {
            x: None,
            y: Some(1)
        }]
    );

    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_move_down(b"2", &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPosRel {
            x: None,
            y: Some(2)
        }]
    );

    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_finished_move_down(b"test", &mut output);
    assert_eq!(output, vec![TerminalOutput::Invalid]);
    assert!(result.is_err());
}

#[test]
fn test_cuf() {
    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_move_right(&[], &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPosRel {
            x: Some(1),
            y: None
        }]
    );

    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_move_right(b"1", &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPosRel {
            x: Some(1),
            y: None
        }]
    );

    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_move_right(b"2", &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPosRel {
            x: Some(2),
            y: None
        }]
    );

    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_finished_move_right(b"test", &mut output);
    assert_eq!(output, vec![TerminalOutput::Invalid]);
    assert!(result.is_err());
}

#[test]
fn test_cup() {
    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_set_position_h(b"1;1", &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPos {
            x: Some(1),
            y: Some(1)
        }]
    );

    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_set_position_h(b"1;", &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPos {
            x: Some(1),
            y: Some(1)
        }]
    );

    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_set_position_h(b";1", &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPos {
            x: Some(1),
            y: Some(1)
        }]
    );

    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_set_position_h(b"", &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPos {
            x: Some(1),
            y: Some(1)
        }]
    );

    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_finished_set_position_h(b"test", &mut output);
    assert_eq!(output, vec![TerminalOutput::Invalid]);
    assert!(result.is_err());
}

#[test]
fn test_cuu() {
    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_move_up(&[], &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPosRel {
            x: None,
            y: Some(-1)
        }]
    );

    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_move_up(b"1", &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPosRel {
            x: None,
            y: Some(-1)
        }]
    );

    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_move_up(b"2", &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::SetCursorPosRel {
            x: None,
            y: Some(-2)
        }]
    );

    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_finished_move_up(b"test", &mut output);
    assert_eq!(output, vec![TerminalOutput::Invalid]);
    assert!(result.is_err());
}

#[test]
fn test_dch() {
    let mut output = Vec::new();

    ansi_parser_inner_csi_finished_set_position_p(&[], &mut output).unwrap();
    assert_eq!(output, vec![TerminalOutput::Delete(1)]);

    output.clear();
    ansi_parser_inner_csi_finished_set_position_p(b"0", &mut output).unwrap();
    assert_eq!(output, vec![TerminalOutput::Delete(1)]);

    output.clear();
    ansi_parser_inner_csi_finished_set_position_p(b"1", &mut output).unwrap();
    assert_eq!(output, vec![TerminalOutput::Delete(1)]);

    output.clear();
    ansi_parser_inner_csi_finished_set_position_p(b"2", &mut output).unwrap();
    assert_eq!(output, vec![TerminalOutput::Delete(2)]);

    output.clear();
    let result = ansi_parser_inner_csi_finished_set_position_p(b"test", &mut output);
    assert_eq!(output, vec![TerminalOutput::Invalid]);
    assert!(result.is_err());
}

#[test]
fn test_ed() {
    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_set_position_j(&[], &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::ClearDisplayfromCursortoEndofDisplay]
    );

    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_set_position_j(b"1", &mut output).unwrap();
    assert_eq!(
        output,
        vec![TerminalOutput::ClearDiplayfromStartofDisplaytoCursor]
    );

    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_set_position_j(b"2", &mut output).unwrap();
    assert_eq!(output, vec![TerminalOutput::ClearDisplay]);

    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_set_position_j(b"3", &mut output).unwrap();
    assert_eq!(output, vec![TerminalOutput::ClearScrollbackandDisplay]);

    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_set_position_j(b"4", &mut output).unwrap();
    assert_eq!(output, vec![TerminalOutput::Invalid]);

    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_finished_set_position_j(b"test", &mut output);
    assert_eq!(output, vec![TerminalOutput::Invalid]);
    assert!(result.is_err());
}

#[test]
fn test_el() {
    let mut output = Vec::new();

    ansi_parser_inner_csi_finished_set_position_k(b"0", &mut output).unwrap();
    assert_eq!(output, vec![TerminalOutput::ClearLineForwards]);

    output.clear();
    ansi_parser_inner_csi_finished_set_position_k(b"1", &mut output).unwrap();
    assert_eq!(output, vec![TerminalOutput::ClearLineBackwards]);

    output.clear();
    ansi_parser_inner_csi_finished_set_position_k(b"2", &mut output).unwrap();
    assert_eq!(output, vec![TerminalOutput::ClearLine]);

    output.clear();
    ansi_parser_inner_csi_finished_set_position_k(b"3", &mut output).unwrap();
    assert_eq!(output, vec![TerminalOutput::Invalid]);

    output.clear();
    ansi_parser_inner_csi_finished_set_position_k(b"", &mut output).unwrap();
    assert_eq!(output, vec![TerminalOutput::ClearLineForwards]);

    output.clear();
    let result = ansi_parser_inner_csi_finished_set_position_k(b"test", &mut output);
    assert_eq!(output, vec![TerminalOutput::Invalid]);
    assert!(result.is_err());
}

#[test]
fn test_ich() {
    let mut output = Vec::new();
    let mut params = Vec::new();

    ansi_parser_inner_csi_finished_ich(&params, &mut output).unwrap();
    assert_eq!(output, vec![TerminalOutput::InsertSpaces(1)]);

    output.clear();
    params.push(b'0');
    ansi_parser_inner_csi_finished_ich(&params, &mut output).unwrap();
    assert_eq!(output, vec![TerminalOutput::InsertSpaces(1)]);

    output.clear();
    params.push(b';');
    let result = ansi_parser_inner_csi_finished_ich(&params, &mut output);
    assert!(result.is_err());
    assert_eq!(output, vec![TerminalOutput::Invalid]);
}

#[test]
fn test_il() {
    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_set_position_l(&[], &mut output).unwrap();
    assert_eq!(output, vec![TerminalOutput::InsertLines(1)]);

    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_set_position_l(b"1", &mut output).unwrap();
    assert_eq!(output, vec![TerminalOutput::InsertLines(1)]);

    let mut output = Vec::new();
    ansi_parser_inner_csi_finished_set_position_l(b"2", &mut output).unwrap();
    assert_eq!(output, vec![TerminalOutput::InsertLines(2)]);

    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_finished_set_position_l(b"test", &mut output);
    assert_eq!(output, vec![TerminalOutput::Invalid]);
    assert!(result.is_err());
}

#[test]
fn test_sgr() {
    for i in 0..=107usize {
        let mut output = Vec::new();
        let i_string = i.to_string();
        let params = i_string.as_bytes();
        let result = ansi_parser_inner_csi_finished_sgr_ansi(params, &mut output);
        match result {
            Ok(Some(ParserInner::Empty)) => (),
            _ => panic!("Failed for {i}"),
        }
        assert_eq!(
            output,
            vec![TerminalOutput::Sgr(SelectGraphicRendition::from_usize(i))],
            "Failed for {i}"
        );
    }

    // now test SGR 38 and 48

    let mut output = Vec::new();
    let params = b"38;2;255;255;255";
    let result = ansi_parser_inner_csi_finished_sgr_ansi(params, &mut output);
    assert!(matches!(result, Ok(Some(ParserInner::Empty))));
    assert_eq!(
        output,
        vec![TerminalOutput::Sgr(SelectGraphicRendition::Foreground(
            TerminalColor::Custom(255, 255, 255)
        ))]
    );

    let mut output = Vec::new();
    let params = b"48;5;255";
    let result = ansi_parser_inner_csi_finished_sgr_ansi(params, &mut output);
    assert!(matches!(result, Ok(Some(ParserInner::Empty))));
    assert_eq!(
        output,
        vec![TerminalOutput::Sgr(SelectGraphicRendition::Background(
            TerminalColor::Custom(238, 238, 238)
        ))]
    );
}

#[test]
fn test_sgr_invalid() {
    let mut output = Vec::new();
    let params = b"test";
    let result = ansi_parser_inner_csi_finished_sgr_ansi(params, &mut output);
    assert!(result.is_err());
    assert_eq!(output, vec![TerminalOutput::Invalid]);

    // no params
    let mut output = Vec::new();
    let params = b"";
    let result = ansi_parser_inner_csi_finished_sgr_ansi(params, &mut output);
    assert!(result.is_ok());
    // check the output
    assert_eq!(
        output,
        vec![TerminalOutput::Sgr(SelectGraphicRendition::Reset)],
        "Failed for {output:?}"
    );

    // test 38, 48, 58 with no color
    let mut output = Vec::new();
    let params = b"38";
    let result = ansi_parser_inner_csi_finished_sgr_ansi(params, &mut output);
    assert!(result.is_ok());
    // check the output
    assert_eq!(
        output,
        vec![TerminalOutput::Sgr(SelectGraphicRendition::Foreground(
            TerminalColor::Default
        ))],
        "Failed for {output:?}"
    );

    let mut output = Vec::new();
    let params = b"48";
    let result = ansi_parser_inner_csi_finished_sgr_ansi(params, &mut output);
    assert!(result.is_ok());
    // check the output
    assert_eq!(
        output,
        vec![TerminalOutput::Sgr(SelectGraphicRendition::Background(
            TerminalColor::DefaultBackground
        ))],
        "Failed for {output:?}"
    );

    let mut output = Vec::new();
    let params = b"58";
    let result = ansi_parser_inner_csi_finished_sgr_ansi(params, &mut output);
    assert!(result.is_ok());
    // check the output
    assert_eq!(
        output,
        vec![TerminalOutput::Sgr(SelectGraphicRendition::UnderlineColor(
            TerminalColor::DefaultUnderlineColor
        ))],
        "Failed for {output:?}"
    );

    // now test 38, 48, 58 with 2 but not enough params
    let mut output = Vec::new();
    let params = b"38;2";
    let result = ansi_parser_inner_csi_finished_sgr_ansi(params, &mut output);
    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::Sgr(SelectGraphicRendition::Foreground(
            TerminalColor::Custom(0, 0, 0)
        ))]
    );

    let mut output = Vec::new();
    let params = b"48;5";
    let result = ansi_parser_inner_csi_finished_sgr_ansi(params, &mut output);
    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::Sgr(SelectGraphicRendition::Background(
            TerminalColor::Custom(0, 0, 0)
        ))]
    );

    let mut output = Vec::new();
    let params = b"58;2;255";
    let result = ansi_parser_inner_csi_finished_sgr_ansi(params, &mut output);
    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::Sgr(SelectGraphicRendition::UnderlineColor(
            TerminalColor::Custom(255, 0, 0)
        ))]
    );

    // now test 38, 48 and 58 with color space
    let mut output = Vec::new();
    let params = b"38:2:1:255:255:0";
    let result = ansi_parser_inner_csi_finished_sgr_ansi(params, &mut output);
    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::Sgr(SelectGraphicRendition::Foreground(
            TerminalColor::Custom(255, 255, 0)
        ))]
    );

    let mut output = Vec::new();
    let params = b"48:2:1:255:255:0";
    let result = ansi_parser_inner_csi_finished_sgr_ansi(params, &mut output);
    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::Sgr(SelectGraphicRendition::Background(
            TerminalColor::Custom(255, 255, 0)
        ))]
    );

    let mut output = Vec::new();
    let params = b"58:2::255:255:0";
    let result = ansi_parser_inner_csi_finished_sgr_ansi(params, &mut output);
    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::Sgr(SelectGraphicRendition::UnderlineColor(
            TerminalColor::Custom(255, 255, 0)
        ))]
    );

    let mut output = Vec::new();
    let params = b"38:2::255:255:0";
    let result = ansi_parser_inner_csi_finished_sgr_ansi(params, &mut output);
    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::Sgr(SelectGraphicRendition::Foreground(
            TerminalColor::Custom(255, 255, 0)
        ))]
    );

    let mut output = Vec::new();
    let params = b"48:2::255:255:0";
    let result = ansi_parser_inner_csi_finished_sgr_ansi(params, &mut output);
    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::Sgr(SelectGraphicRendition::Background(
            TerminalColor::Custom(255, 255, 0)
        ))]
    );

    let mut output = Vec::new();
    let params = b"58:2::255:255:0";
    let result = ansi_parser_inner_csi_finished_sgr_ansi(params, &mut output);
    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::Sgr(SelectGraphicRendition::UnderlineColor(
            TerminalColor::Custom(255, 255, 0)
        ))]
    );
}

#[test]
fn test_from_usize_color() {
    let val = 69;
    let r = 255;
    let g = 255;
    let b = 255;
    let result = SelectGraphicRendition::from_usize_color(val, r, g, b).unwrap();

    assert_eq!(
        result,
        SelectGraphicRendition::Unknown(69),
        "Failed for {result:?}"
    );
}

#[test]
fn test_deqrqm() {
    let intermediates = vec![b'$'];
    let terminator = b'h';
    let params = b"?1";

    let mut output = Vec::new();

    let result =
        ansi_parser_inner_csi_finished_decrqm(params, &intermediates, terminator, &mut output);

    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::Mode(Mode::Decckm(Decckm::Query))],
        "Failed for {output:?}"
    );

    let intermediates = vec![];
    let mut output = Vec::new();

    let result =
        ansi_parser_inner_csi_finished_decrqm(params, &intermediates, terminator, &mut output);
    assert!(result.is_ok());
    let result = result.unwrap();
    assert_eq!(result, Some(ParserInner::Empty));
    assert_eq!(
        output,
        vec![TerminalOutput::Mode(Mode::Decckm(Decckm::Application))],
        "Failed for {output:?}"
    );

    let terminator = b'l';
    let mut output = Vec::new();

    let result =
        ansi_parser_inner_csi_finished_decrqm(params, &intermediates, terminator, &mut output);
    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::Mode(Mode::Decckm(Decckm::Ansi))],
        "Failed for {output:?}"
    );

    let terminator = b'p';
    let mut output = Vec::new();
    let result =
        ansi_parser_inner_csi_finished_decrqm(params, &intermediates, terminator, &mut output);
    assert!(result.is_err());
}

#[test]
fn test_decscusr() {
    let params = b"?1";
    let mut output = vec![];

    let result = ansi_parser_inner_csi_finished_set_position_q(params, &mut output);
    assert!(result.is_err());
    assert_eq!(output, vec![TerminalOutput::Invalid]);

    let params = b"0";
    let mut output = vec![];
    let result = ansi_parser_inner_csi_finished_set_position_q(params, &mut output);
    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::CursorVisualStyle(
            CursorVisualStyle::BlockCursorBlink
        )],
        "Failed for {output:?}"
    );

    let params = b"1";
    let mut output = vec![];
    let result = ansi_parser_inner_csi_finished_set_position_q(params, &mut output);
    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::CursorVisualStyle(
            CursorVisualStyle::BlockCursorBlink
        )],
        "Failed for {output:?}"
    );

    let params = b"2";
    let mut output = vec![];
    let result = ansi_parser_inner_csi_finished_set_position_q(params, &mut output);
    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::CursorVisualStyle(
            CursorVisualStyle::BlockCursorSteady
        )],
        "Failed for {output:?}"
    );

    let params = b"3";
    let mut output = vec![];
    let result = ansi_parser_inner_csi_finished_set_position_q(params, &mut output);
    assert!(result.is_ok());

    assert_eq!(
        output,
        vec![TerminalOutput::CursorVisualStyle(
            CursorVisualStyle::UnderlineCursorBlink
        )],
        "Failed for {output:?}"
    );

    let params = b"4";
    let mut output = vec![];
    let result = ansi_parser_inner_csi_finished_set_position_q(params, &mut output);
    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::CursorVisualStyle(
            CursorVisualStyle::UnderlineCursorSteady
        )],
        "Failed for {output:?}"
    );

    let params = b"5";
    let mut output = vec![];
    let result = ansi_parser_inner_csi_finished_set_position_q(params, &mut output);
    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::CursorVisualStyle(
            CursorVisualStyle::VerticalLineCursorBlink
        )],
        "Failed for {output:?}"
    );

    let params = b"6";
    let mut output = vec![];
    let result = ansi_parser_inner_csi_finished_set_position_q(params, &mut output);
    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::CursorVisualStyle(
            CursorVisualStyle::VerticalLineCursorSteady
        )],
        "Failed for {output:?}"
    );

    let params = b"7";
    let mut output = vec![];
    let result = ansi_parser_inner_csi_finished_set_position_q(params, &mut output);
    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::CursorVisualStyle(
            CursorVisualStyle::BlockCursorBlink
        )],
        "Failed for {output:?}"
    );
}

#[test]
fn test_ech() {
    let mut output = Vec::new();
    let params = Vec::new();

    let result = ansi_parser_inner_csi_finished_set_position_x(&params, &mut output);
    assert!(result.is_ok());
    let result = result.unwrap();
    assert_eq!(result, Some(ParserInner::Empty));
    assert_eq!(output, vec![TerminalOutput::Erase(1)]);

    let params = b"67";
    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_finished_set_position_x(params, &mut output);
    assert!(result.is_ok());
    assert_eq!(output, vec![TerminalOutput::Erase(67)]);

    let params = b"test";
    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_finished_set_position_x(params, &mut output);
    assert!(result.is_err());
    assert_eq!(output, vec![TerminalOutput::Invalid]);
}

#[test]
fn test_decslpp() {
    let mut output = Vec::new();
    let params = Vec::new();

    let result = ansi_parser_inner_csi_finished_set_position_t(&params, &mut output);
    assert!(result.is_err());

    let params = b"tesT";
    let result = ansi_parser_inner_csi_finished_set_position_t(params, &mut output);
    assert!(result.is_err());

    let params = b"1";
    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_finished_set_position_t(params, &mut output);
    assert!(result.is_ok());
    let result = result.unwrap();
    assert_eq!(result, Some(ParserInner::Empty));

    assert_eq!(
        output,
        vec![TerminalOutput::WindowManipulation(
            WindowManipulation::DeIconifyWindow
        )]
    );

    let params = b";";
    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_finished_set_position_t(params, &mut output);
    assert!(result.is_err());

    let params = b"";
    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_finished_set_position_t(params, &mut output);
    assert!(result.is_err());

    let params = b"1;";
    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_finished_set_position_t(params, &mut output);
    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::WindowManipulation(
            WindowManipulation::DeIconifyWindow
        )]
    );

    let params = b"1;0";
    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_finished_set_position_t(params, &mut output);
    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::WindowManipulation(
            WindowManipulation::DeIconifyWindow
        )]
    );

    let params = b"1;0;";
    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_finished_set_position_t(params, &mut output);
    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::WindowManipulation(
            WindowManipulation::DeIconifyWindow
        )]
    );

    let params = b"1;0;0";
    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_finished_set_position_t(params, &mut output);
    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::WindowManipulation(
            WindowManipulation::DeIconifyWindow
        )]
    );

    let params = b"67;0;0;";
    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_finished_set_position_t(params, &mut output);
    assert!(result.is_err());
    assert_eq!(output, vec![TerminalOutput::Invalid]);
}

#[test]
fn test_decstbm() {
    let params = b"";
    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_set_top_and_bottom_margins(params, &mut output);
    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::SetTopAndBottomMargins {
            top_margin: 1,
            bottom_margin: usize::MAX
        }]
    );

    let params = b"2;test";
    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_set_top_and_bottom_margins(params, &mut output);
    assert!(result.is_err());
    assert_eq!(output, vec![TerminalOutput::Invalid]);

    let params = b"2;";
    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_set_top_and_bottom_margins(params, &mut output);
    assert!(result.is_ok(), "Failed for {result:?}");
    assert_eq!(
        output,
        vec![TerminalOutput::SetTopAndBottomMargins {
            top_margin: 2,
            bottom_margin: usize::MAX
        }]
    );

    let params = b"2;2";
    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_set_top_and_bottom_margins(params, &mut output);
    assert!(result.is_err(), "Failed for {result:?}");
    assert_eq!(output, vec![TerminalOutput::Invalid]);

    let params = b"2;3;4";
    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_set_top_and_bottom_margins(params, &mut output);
    assert!(result.is_err(), "Failed for {result:?}");
    assert_eq!(output, vec![TerminalOutput::Invalid]);

    let params = b"2;3";
    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_set_top_and_bottom_margins(params, &mut output);
    assert!(result.is_ok(), "Failed for {result:?}");
    assert_eq!(
        output,
        vec![TerminalOutput::SetTopAndBottomMargins {
            top_margin: 2,
            bottom_margin: 3
        }]
    );

    let params = b"0;1";
    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_set_top_and_bottom_margins(params, &mut output);
    assert!(result.is_err(), "Failed for {result:?}");
    assert_eq!(output, vec![TerminalOutput::Invalid]);

    let params = b"1;0";
    let mut output = Vec::new();
    let result = ansi_parser_inner_csi_set_top_and_bottom_margins(params, &mut output);
    assert!(result.is_err(), "Failed for {result:?}");
    assert_eq!(output, vec![TerminalOutput::Invalid]);
}

#[test]
fn test_request_device_attributes() {
    let params = b"";
    let intermediates = b"$";
    let mut output = Vec::new();
    let results = ansi_parser_inner_csi_finished_send_da(params, intermediates, &mut output);
    assert!(results.is_err());
    assert_eq!(output, vec![TerminalOutput::Invalid]);

    let params = b"t";
    let intermediates = b"";
    let mut output = Vec::new();
    let results = ansi_parser_inner_csi_finished_send_da(params, intermediates, &mut output);
    assert!(results.is_err());
    assert_eq!(output, vec![TerminalOutput::Invalid]);

    let params = b"1";
    let intermediates = b"";
    let mut output = Vec::new();
    let results = ansi_parser_inner_csi_finished_send_da(params, intermediates, &mut output);
    assert!(results.is_err());
    assert_eq!(output, vec![TerminalOutput::Invalid]);

    let params = b"";
    let intermediates = b"";
    let mut output = Vec::new();
    let results = ansi_parser_inner_csi_finished_send_da(params, intermediates, &mut output);
    assert!(results.is_ok());
    assert_eq!(output, vec![TerminalOutput::RequestDeviceAttributes]);

    let params = b"0";
    let intermediates = b"";
    let mut output = Vec::new();
    let results = ansi_parser_inner_csi_finished_send_da(params, intermediates, &mut output);
    assert!(results.is_ok());
    assert_eq!(output, vec![TerminalOutput::RequestDeviceAttributes]);
}

#[test]
fn test_handle_custom_color() {
    let mut output = Vec::new();
    let params = b"9;255;255;255";
    let params = split_params_into_semicolon_delimited_usize(params).unwrap();
    let mut param_iter: std::vec::IntoIter<Option<usize>> = params.into_iter();
    handle_custom_color(&mut output, &mut param_iter, 58, false);
    assert_eq!(output, vec![TerminalOutput::Invalid]);

    let mut output = Vec::new();
    let params = b"2;255;255;2556";
    let params = split_params_into_semicolon_delimited_usize(params).unwrap();
    let mut param_iter: std::vec::IntoIter<Option<usize>> = params.into_iter();
    handle_custom_color(&mut output, &mut param_iter, 58, false);
    assert_eq!(output, vec![TerminalOutput::Invalid]);
}

#[test]
fn ansi_parser_inner_sgr_empty() {
    let mut output = Vec::new();
    let params = b"";
    let result = ansi_parser_inner_csi_finished_sgr_ansi(params, &mut output);
    assert!(result.is_ok());
    assert_eq!(
        output,
        vec![TerminalOutput::Sgr(SelectGraphicRendition::Reset)]
    );
}
