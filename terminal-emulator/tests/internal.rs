// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use eframe::egui::Context;
use terminal_emulator::{
    ansi::FreminalAnsiParser,
    ansi_components::mode::{BracketedPaste, Decckm, TerminalModes},
    format_tracker::FormatTracker,
    state::{
        buffer::TerminalBufferHolder,
        cursor::CursorState,
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
}
