// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::i32;

use eframe::egui::Context;
use terminal_emulator::{
    ansi::FreminalAnsiParser,
    ansi_components::mode::{BracketedPaste, Decckm, TerminalModes},
    format_tracker::FormatTracker,
    state::{
        buffer::TerminalBufferHolder,
        cursor::{CursorPos, CursorState},
        internal::{TerminalState, TERMINAL_HEIGHT, TERMINAL_WIDTH},
    },
};

#[test]
fn test_internal_terminal_state_new() {
    let (tx, _rx) = crossbeam_channel::unbounded();
    let mut terminal_state = TerminalState::new(tx.clone());
    let ctx = Context::default();
    let expected = TerminalState {
        parser: FreminalAnsiParser::new(),
        terminal_buffer: TerminalBufferHolder::new(TERMINAL_WIDTH, TERMINAL_HEIGHT),
        format_tracker: FormatTracker::new(),
        modes: TerminalModes {
            cursor_key: Decckm::default(),
            bracketed_paste: BracketedPaste::default(),
        },
        cursor_state: CursorState::default(),
        saved_color_state: None,
        window_title: None,
        write_tx: tx,
        changed: false,
        ctx: None,
        leftover_data: None,
    };

    assert_eq!(terminal_state, expected);

    // test is_changed()
    assert!(!terminal_state.is_changed());

    // set changed to true
    terminal_state.set_state_changed();

    assert!(terminal_state.is_changed());

    // set changed to false
    terminal_state.clear_changed();

    assert!(!terminal_state.is_changed());

    // test set_ctx()
    terminal_state.set_ctx(ctx);
    assert!(terminal_state.ctx.is_some());

    // get the window size
    let (width, height) = terminal_state.get_win_size();
    assert_eq!(width, TERMINAL_WIDTH);
    assert_eq!(height, TERMINAL_HEIGHT);
    terminal_state.set_win_size(69, 69);
    let (width, height) = terminal_state.get_win_size();
    assert_eq!(width, 69);
    assert_eq!(height, 69);

    terminal_state.window_title = Some("test".to_string());
    assert_eq!(terminal_state.get_window_title(), Some("test".to_string()));
    terminal_state.clear_window_title();
    assert_eq!(terminal_state.get_window_title(), None);

    let cursor_key_mode = terminal_state.get_cursor_key_mode();
    assert_eq!(cursor_key_mode, Decckm::default());

    terminal_state.set_cursor_pos(Some(5), Some(5));
    let cursor_pos = terminal_state.cursor_state.pos.clone();
    let expected = CursorPos { x: 4, y: 4 };
    assert_eq!(cursor_pos, expected);

    terminal_state.set_cursor_pos(Some(1), None);
    let cursor_pos = terminal_state.cursor_state.pos.clone();
    let expected = CursorPos { x: 0, y: 4 };
    assert_eq!(cursor_pos, expected);

    terminal_state.set_cursor_pos(None, Some(10));
    let cursor_pos = terminal_state.cursor_state.pos.clone();
    let expected = CursorPos { x: 0, y: 9 };
    assert_eq!(cursor_pos, expected);

    // set cursor rel
    terminal_state.set_cursor_pos_rel(Some(1), Some(1));
    let cursor_pos = terminal_state.cursor_state.pos.clone();
    let expected = CursorPos { x: 1, y: 10 };
    assert_eq!(cursor_pos, expected);

    terminal_state.set_cursor_pos_rel(Some(1), None);
    let cursor_pos = terminal_state.cursor_state.pos.clone();
    let expected = CursorPos { x: 2, y: 10 };
    assert_eq!(cursor_pos, expected);

    terminal_state.set_cursor_pos_rel(None, Some(-8));
    let cursor_pos = terminal_state.cursor_state.pos.clone();
    let expected = CursorPos { x: 2, y: 2 };
    assert_eq!(cursor_pos, expected);
}
