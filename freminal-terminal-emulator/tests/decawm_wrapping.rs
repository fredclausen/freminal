use test_log::test;

// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use freminal_terminal_emulator::{
    ansi_components::modes::decawm::Decawm,
    state::{
        buffer::TerminalBufferHolder,
        cursor::CursorPos,
        internal::BufferType,
        term_char::{display_vec_tchar_as_string, TChar},
    },
};

#[test]
fn test_decawm_basic_no_wrap() {
    let decawm = Decawm::NoAutoWrap;
    let mut buffer = TerminalBufferHolder::new(5, 5, BufferType::Primary);
    let response = buffer
        .insert_data(&CursorPos::default(), b"test", &decawm)
        .unwrap();

    let expected = [
        TChar::new_from_single_char(b't'),
        TChar::new_from_single_char(b'e'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b't'),
        TChar::NewLine,
    ];
    assert_eq!(
        buffer.buf,
        expected,
        "\nInternal buffer: {}Expected: {}",
        display_vec_tchar_as_string(&buffer.buf),
        display_vec_tchar_as_string(&expected),
    );
    let expected_cursor = CursorPos { x: 4, y: 0 };
    assert_eq!(
        response.new_cursor_pos, expected_cursor,
        "\nInternal cursor: {}Expected: {}",
        response.new_cursor_pos, expected_cursor
    );

    let cursor = CursorPos { x: 4, y: 0 };
    let response = buffer.insert_data(&cursor, b"abcd", &decawm).unwrap();
    let expected = [
        TChar::new_from_single_char(b't'),
        TChar::new_from_single_char(b'e'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b't'),
        TChar::new_from_single_char(b'd'),
        TChar::NewLine,
    ];
    assert_eq!(
        buffer.buf,
        expected,
        "\nInternal buffer: {}Expected: {}",
        display_vec_tchar_as_string(&buffer.buf),
        display_vec_tchar_as_string(&expected),
    );
    let expected_cursor = CursorPos { x: 4, y: 0 };
    assert_eq!(
        response.new_cursor_pos, expected_cursor,
        "\nInternal cursor: {}Expected: {}",
        response.new_cursor_pos, expected_cursor
    );
}

#[test]
fn test_decawm_basic_longer_line_no_wrap() {
    let decawm = Decawm::NoAutoWrap;
    let mut buffer = TerminalBufferHolder::new(7, 7, BufferType::Primary);
    let response = buffer
        .insert_data(&CursorPos::default(), b"test", &decawm)
        .unwrap();

    let expected = [
        TChar::new_from_single_char(b't'),
        TChar::new_from_single_char(b'e'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b't'),
        TChar::NewLine,
    ];
    assert_eq!(
        buffer.buf,
        expected,
        "\nInternal buffer: {}Expected: {}",
        display_vec_tchar_as_string(&buffer.buf),
        display_vec_tchar_as_string(&expected),
    );
    let expected_cursor = CursorPos { x: 4, y: 0 };
    assert_eq!(
        response.new_cursor_pos, expected_cursor,
        "\nInternal cursor: {}Expected: {}",
        response.new_cursor_pos, expected_cursor
    );

    let cursor = CursorPos { x: 4, y: 0 };
    let response = buffer.insert_data(&cursor, b"abcd", &decawm).unwrap();
    let expected = [
        TChar::new_from_single_char(b't'),
        TChar::new_from_single_char(b'e'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b't'),
        TChar::new_from_single_char(b'a'),
        TChar::new_from_single_char(b'b'),
        TChar::new_from_single_char(b'd'),
        TChar::NewLine,
    ];
    assert_eq!(
        buffer.buf,
        expected,
        "\nInternal buffer: {}Expected: {}",
        display_vec_tchar_as_string(&buffer.buf),
        display_vec_tchar_as_string(&expected),
    );
    let expected_cursor = CursorPos { x: 6, y: 0 };
    assert_eq!(
        response.new_cursor_pos, expected_cursor,
        "\nInternal cursor: {}Expected: {}",
        response.new_cursor_pos, expected_cursor
    );
}

#[test]
fn test_decawm_basic_wrap_insert_single_end_of_line() {
    let decawm = Decawm::NoAutoWrap;
    let mut buffer = TerminalBufferHolder::new(5, 5, BufferType::Primary);
    let response = buffer
        .insert_data(&CursorPos::default(), b"test", &decawm)
        .unwrap();

    let expected = [
        TChar::new_from_single_char(b't'),
        TChar::new_from_single_char(b'e'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b't'),
        TChar::NewLine,
    ];
    assert_eq!(
        buffer.buf,
        expected,
        "\nInternal buffer: {}Expected: {}",
        display_vec_tchar_as_string(&buffer.buf),
        display_vec_tchar_as_string(&expected),
    );
    let expected_cursor = CursorPos { x: 4, y: 0 };
    assert_eq!(
        response.new_cursor_pos, expected_cursor,
        "\nInternal cursor: {}Expected: {}",
        response.new_cursor_pos, expected_cursor
    );
    let cursor = CursorPos { x: 4, y: 0 };
    let response = buffer.insert_data(&cursor, b"a", &decawm).unwrap();
    let expected = [
        TChar::new_from_single_char(b't'),
        TChar::new_from_single_char(b'e'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b't'),
        TChar::new_from_single_char(b'a'),
        TChar::NewLine,
    ];
    assert_eq!(
        buffer.buf,
        expected,
        "\nInternal buffer: {}Expected: {}",
        display_vec_tchar_as_string(&buffer.buf),
        display_vec_tchar_as_string(&expected),
    );
    let expected_cursor = CursorPos { x: 4, y: 0 };
    assert_eq!(
        response.new_cursor_pos, expected_cursor,
        "\nInternal cursor: {}Expected: {}",
        response.new_cursor_pos, expected_cursor
    );

    let cursor = CursorPos { x: 4, y: 0 };
    let response = buffer.insert_data(&cursor, b"b", &decawm).unwrap();
    let expected = [
        TChar::new_from_single_char(b't'),
        TChar::new_from_single_char(b'e'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b't'),
        TChar::new_from_single_char(b'b'),
        TChar::NewLine,
    ];
    assert_eq!(
        buffer.buf,
        expected,
        "\nInternal buffer: {}Expected: {}",
        display_vec_tchar_as_string(&buffer.buf),
        display_vec_tchar_as_string(&expected),
    );
    assert_eq!(
        response.new_cursor_pos, expected_cursor,
        "\nInternal cursor: {}Expected: {}",
        response.new_cursor_pos, expected_cursor
    );
}
