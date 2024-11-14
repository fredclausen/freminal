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
        cursor::{CursorPos, CursorState},
        internal::{TerminalState, TERMINAL_HEIGHT, TERMINAL_WIDTH},
        term_char::{display_vec_tchar_as_string, TChar},
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

#[test]
fn test_internal_terminal_state_data() {
    let (tx, _rx) = crossbeam_channel::unbounded();
    let mut terminal_state = TerminalState::new(tx.clone());

    // new data for it to process. This is a simple string with no escape codes
    let data = b"Hello, World!";
    terminal_state.handle_incoming_data(data);
    // verify that the data was written to the buffer
    let buffer = terminal_state.terminal_buffer.data();
    let expected = TChar::from_vec(b"Hello, World!\n").unwrap();
    assert_eq!(buffer.visible, expected);

    // test leftover data
    terminal_state.leftover_data = Some(b"Hello, World!".to_vec());
    terminal_state.handle_incoming_data(b"\n");
    let buffer = terminal_state.terminal_buffer.data();
    println!("{:?}", buffer);
    let expected = TChar::from_vec(b"Hello, World!Hello, World!\n").unwrap();
    // combine the two buffers in to one vec of TChar
    let buffer = buffer
        .scrollback
        .into_iter()
        .chain(buffer.visible)
        .collect::<Vec<TChar>>();
    assert_eq!(buffer, expected);
    assert!(terminal_state.leftover_data.is_none());
}

#[test]
fn test_set_cursor_pos() {
    let (tx, _rx) = crossbeam_channel::unbounded();
    let mut terminal_state = TerminalState::new(tx.clone());

    // vector of bytes that represent the string "\0xb[1;1HHello, World!"
    let data: [u8; 19] = [
        0x1b, 0x5b, 0x31, 0x3b, 0x31, 0x48, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x2c, 0x20, 0x57, 0x6f,
        0x72, 0x6c, 0x64, 0x21,
    ];
    terminal_state.handle_incoming_data(&data);
    let buffer = terminal_state.terminal_buffer.data();
    let expected = TChar::from_vec(b"Hello, World!\n").unwrap();
    assert_eq!(buffer.visible, expected);
    // verify that the cursor position is set to the end of the string
    let cursor_pos = terminal_state.cursor_state.pos.clone();
    let expected = CursorPos { x: 13, y: 0 };
    assert_eq!(cursor_pos, expected);

    // test cursor movement
    let data: [u8; 6] = [0x1b, 0x5b, 0x31, 0x3b, 0x31, 0x48];
    terminal_state.handle_incoming_data(&data);
    let cursor_pos = terminal_state.cursor_state.pos.clone();
    let expected = CursorPos { x: 0, y: 0 };
    assert_eq!(cursor_pos, expected);
}

#[test]
fn test_set_cursor_pos_rel() {
    let (tx, _rx) = crossbeam_channel::unbounded();
    let mut terminal_state = TerminalState::new(tx.clone());
    let data = b"Hello, World!";
    terminal_state.handle_incoming_data(data);

    // ESC [ Pn D
    // "\0x1b[3D" moves the cursor right by 3
    let data: [u8; 4] = [0x1b, 0x5b, 0x33, 0x44];
    terminal_state.handle_incoming_data(&data);
    let cursor_pos = terminal_state.cursor_state.pos.clone();
    let expected = CursorPos { x: 10, y: 0 };
    assert_eq!(cursor_pos, expected);
}

#[test]
fn test_clear_display_from_cursor_to_end_of_display() {
    let (tx, _rx) = crossbeam_channel::unbounded();
    let mut terminal_state = TerminalState::new(tx.clone());
    let data = b"Hello, World!";
    terminal_state.handle_incoming_data(data);

    // ESC [ Pn D
    // "\0x1b[3D" moves the cursor right by 3
    let data: [u8; 4] = [0x1b, 0x5b, 0x33, 0x44];
    terminal_state.handle_incoming_data(&data);

    // setup of buffer complete. Now we will clear the display from the cursor to the end of the display
    // ESC [ Pn J
    // "\0x1b[0J" clears the display from the cursor to the end of the display

    let data: [u8; 4] = [0x1b, 0x5b, 0x30, 0x4a];
    terminal_state.handle_incoming_data(&data);
    let buffer = terminal_state.terminal_buffer.data();
    let expected = TChar::from_vec(b"Hello, Wor\n").unwrap();
    assert_eq!(buffer.visible, expected);
}

#[test]
fn test_clear_display_from_start_of_display_to_cursor() {
    let (tx, _rx) = crossbeam_channel::unbounded();
    let mut terminal_state = TerminalState::new(tx.clone());
    let data = b"Hello, World!";
    terminal_state.handle_incoming_data(data);

    // ESC [ Pn D
    // "\0x1b[3D" moves the cursor right by 3
    let data: [u8; 4] = [0x1b, 0x5b, 0x33, 0x44];
    terminal_state.handle_incoming_data(&data);

    // setup of buffer complete. Now we will clear the display from the start of the display to the cursor
    // ESC [ Pn J
    // "\0x1b[1J" clears the display from the start of the display to the cursor

    let data: [u8; 4] = [0x1b, 0x5b, 0x31, 0x4a];
    terminal_state.handle_incoming_data(&data);
    let buffer = terminal_state.terminal_buffer.data();
    let expected = TChar::from_vec(b"          ld!\n").unwrap();
    assert_eq!(buffer.visible, expected);
}

#[test]
fn test_clear_display() {
    let (tx, _rx) = crossbeam_channel::unbounded();
    let mut terminal_state = TerminalState::new(tx.clone());
    terminal_state.set_win_size(30, 5);
    // send the control code to clear the display
    // ESC [ Pn J
    // "\0x1b[3J" clears everything
    let data: [u8; 4] = [0x1b, 0x5b, 0x33, 0x4a];
    terminal_state.handle_incoming_data(&data);
    let buffer = terminal_state.terminal_buffer.data();
    // verify both the visible and scrollback buffers are empty
    assert!(buffer.visible.is_empty());
    assert!(buffer.scrollback.is_empty());

    let data: [u8; 6] = [0x1b, 0x5b, 0x31, 0x3b, 0x31, 0x48];
    terminal_state.handle_incoming_data(&data);

    let data = b"Hello, World!\n\rHello, World!\n\rHello, World!\n\rHello, World!\n\rHello, World!\n\rHello, World!\n\r";
    terminal_state.handle_incoming_data(data);

    // ensure the scrollback and visible buffers are are correct
    let buffer = terminal_state.terminal_buffer.data();
    let expected_visible =
        b"Hello, World!\nHello, World!\nHello, World!\nHello, World!\nHello, World!\n";
    let expected_visible = TChar::from_vec(expected_visible).unwrap();
    let scrollback_expected = b"Hello, World!\n";
    let scrollback_expected = TChar::from_vec(scrollback_expected).unwrap();

    assert_eq!(
        buffer.visible,
        expected_visible,
        "\nvisible buffer:\n{}\nexpected:\n{}",
        display_vec_tchar_as_string(&buffer.visible),
        display_vec_tchar_as_string(&expected_visible)
    );
    assert_eq!(
        buffer.scrollback,
        scrollback_expected,
        "\nscrollback buffer:\n{}\nexpected:\n{}",
        display_vec_tchar_as_string(&buffer.scrollback),
        display_vec_tchar_as_string(&scrollback_expected)
    );

    let old_cursor_pos = terminal_state.cursor_state.pos.clone();
    // clear just the visible buffer
    let data: [u8; 4] = [0x1b, 0x5b, 0x32, 0x4a];
    terminal_state.handle_incoming_data(&data);
    let buffer = terminal_state.terminal_buffer.data();
    // expected visible is the previous expected visible with all the characters replaced with spaces, unless the TChar is a newline
    let expected_visible = expected_visible
        .iter()
        .map(|tchar| {
            if *tchar == TChar::NewLine {
                TChar::NewLine
            } else {
                TChar::Space
            }
        })
        .collect::<Vec<TChar>>();
    assert_eq!(
        buffer.visible,
        expected_visible,
        "visible buffer is not empty: {}",
        display_vec_tchar_as_string(&buffer.visible)
    );
    // verify the cursor position is unchanged
    assert_eq!(old_cursor_pos, terminal_state.cursor_state.pos);
}
