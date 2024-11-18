// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use freminal_common::colors::TerminalColor;
use terminal_emulator::{
    ansi_components::mode::Decawm,
    state::{
        cursor::{CursorPos, CursorState, StateColors},
        fonts::{FontDecorations, FontWeight},
    },
};

#[test]
fn test_cursor_state_default() {
    let cursor = CursorState::default();
    assert_eq!(cursor, CursorState::default());

    // test cursorstate new
    let cursor = CursorState::new();
    assert_eq!(cursor, CursorState::default());
}

#[test]
fn test_cursor_state_with() {
    let cursor = CursorState::default().with_background_color(TerminalColor::Black);

    assert_eq!(
        cursor,
        CursorState {
            colors: StateColors {
                background_color: TerminalColor::Black,
                ..Default::default()
            },
            ..Default::default()
        }
    );

    let cursor = CursorState::default().with_color(TerminalColor::Blue);
    assert_eq!(
        cursor,
        CursorState::default().with_color(TerminalColor::Blue)
    );

    let cursor = CursorState::default().with_font_weight(FontWeight::Bold);
    assert_eq!(
        cursor,
        CursorState {
            pos: CursorPos::default(),
            font_weight: FontWeight::Bold,
            font_decorations: Vec::new(),
            colors: StateColors::default(),
            line_wrap_mode: Decawm::default(),
            url: None,
        }
    );

    let cursor = CursorState::default().with_font_decorations(vec![FontDecorations::Underline]);
    assert_eq!(
        cursor,
        CursorState {
            pos: CursorPos::default(),
            font_weight: FontWeight::default(),
            font_decorations: vec![FontDecorations::Underline],
            colors: StateColors::default(),
            line_wrap_mode: Decawm::default(),
            url: None,
        }
    );

    let cursor = CursorState::default().with_pos(CursorPos { x: 10, y: 10 });
    assert_eq!(
        cursor,
        CursorState {
            pos: CursorPos { x: 10, y: 10 },
            font_weight: FontWeight::default(),
            font_decorations: Vec::new(),
            colors: StateColors::default(),
            line_wrap_mode: Decawm::default(),
            url: None,
        }
    );
}
