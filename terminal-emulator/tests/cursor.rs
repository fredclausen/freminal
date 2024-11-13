// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use freminal_common::colors::TerminalColor;
use terminal_emulator::{
    ansi_components::mode::Decawm,
    state::{
        cursor::{CursorPos, CursorState},
        fonts::{FontDecorations, FontWeight},
    },
};

#[test]
fn test_cursor_state_default() {
    let cursor = CursorState::default();
    assert_eq!(
        cursor,
        CursorState {
            pos: CursorPos::default(),
            font_weight: FontWeight::default(),
            font_decorations: Vec::new(),
            color: TerminalColor::Default,
            background_color: TerminalColor::DefaultBackground,
            underline_color: TerminalColor::DefaultUnderlineColor,
            line_wrap_mode: Decawm::default(),
        }
    );

    // test cursorstate new
    let cursor = CursorState::new();
    assert_eq!(
        cursor,
        CursorState {
            pos: CursorPos::default(),
            font_weight: FontWeight::default(),
            font_decorations: Vec::new(),
            color: TerminalColor::Default,
            background_color: TerminalColor::DefaultBackground,
            underline_color: TerminalColor::DefaultUnderlineColor,
            line_wrap_mode: Decawm::default(),
        }
    );
}

#[test]
fn test_cursor_state_with() {
    let cursor = CursorState::default().with_background_color(TerminalColor::Black);

    assert_eq!(
        cursor,
        CursorState {
            pos: CursorPos::default(),
            font_weight: FontWeight::default(),
            font_decorations: Vec::new(),
            color: TerminalColor::Default,
            background_color: TerminalColor::Black,
            underline_color: TerminalColor::DefaultUnderlineColor,
            line_wrap_mode: Decawm::default(),
        }
    );

    let cursor = CursorState::default().with_color(TerminalColor::Blue);
    assert_eq!(
        cursor,
        CursorState {
            pos: CursorPos::default(),
            font_weight: FontWeight::default(),
            font_decorations: Vec::new(),
            color: TerminalColor::Blue,
            background_color: TerminalColor::DefaultBackground,
            underline_color: TerminalColor::DefaultUnderlineColor,
            line_wrap_mode: Decawm::default(),
        }
    );

    let cursor = CursorState::default().with_font_weight(FontWeight::Bold);
    assert_eq!(
        cursor,
        CursorState {
            pos: CursorPos::default(),
            font_weight: FontWeight::Bold,
            font_decorations: Vec::new(),
            color: TerminalColor::Default,
            background_color: TerminalColor::DefaultBackground,
            underline_color: TerminalColor::DefaultUnderlineColor,
            line_wrap_mode: Decawm::default(),
        }
    );

    let cursor = CursorState::default().with_font_decorations(vec![FontDecorations::Underline]);
    assert_eq!(
        cursor,
        CursorState {
            pos: CursorPos::default(),
            font_weight: FontWeight::default(),
            font_decorations: vec![FontDecorations::Underline],
            color: TerminalColor::Default,
            background_color: TerminalColor::DefaultBackground,
            underline_color: TerminalColor::DefaultUnderlineColor,
            line_wrap_mode: Decawm::default(),
        }
    );

    let cursor = CursorState::default().with_pos(CursorPos { x: 10, y: 10 });
    assert_eq!(
        cursor,
        CursorState {
            pos: CursorPos { x: 10, y: 10 },
            font_weight: FontWeight::default(),
            font_decorations: Vec::new(),
            color: TerminalColor::Default,
            background_color: TerminalColor::DefaultBackground,
            underline_color: TerminalColor::DefaultUnderlineColor,
            line_wrap_mode: Decawm::default(),
        }
    );
}
