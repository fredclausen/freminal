// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use anyhow::Result;
use freminal::terminal_emulator::state::{
    buffer::{
        calc_line_ranges, pad_buffer_for_write, TerminalBufferHolder, TerminalBufferInsertResponse,
    },
    cursor::CursorPos,
    term_char::TChar,
};

fn simulate_resize(
    canvas: &mut TerminalBufferHolder,
    width: usize,
    height: usize,
    cursor_pos: &CursorPos,
) -> Result<TerminalBufferInsertResponse> {
    let mut response = canvas.set_win_size(width, height, cursor_pos);
    response.new_cursor_pos.x = 0;
    let mut response = canvas.insert_data(&response.new_cursor_pos, &vec![b' '; width])?;
    response.new_cursor_pos.x = 0;

    canvas.insert_data(&response.new_cursor_pos, b"$ ")
}

fn crlf(pos: &mut CursorPos) {
    pos.y += 1;
    pos.x = 0;
}

#[test]
fn test_insert_utf8_data() {
    let mut buffer = TerminalBufferHolder::new(10, 10);
    let response = buffer
        .insert_data(&CursorPos { x: 0, y: 0 }, b"asdf")
        .unwrap();
    assert_eq!(response.written_range, 0..4);
    assert_eq!(response.insertion_range, 0..5);
    assert_eq!(response.new_cursor_pos, CursorPos { x: 4, y: 0 });
    let expected = vec![
        TChar::new_from_single_char(b'a'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b'd'),
        TChar::new_from_single_char(b'f'),
        TChar::NewLine,
    ];
    assert_eq!(buffer.data().visible, expected);

    let bytes_utf8 = "👍".as_bytes();
    let response = buffer
        .insert_data(&response.new_cursor_pos, bytes_utf8)
        .unwrap();
    assert_eq!(response.written_range, 4..5);
    assert_eq!(response.insertion_range, 4..5);
    let expected = vec![
        TChar::new_from_single_char(b'a'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b'd'),
        TChar::new_from_single_char(b'f'),
        TChar::new_from_many_chars(bytes_utf8.to_vec()).unwrap(),
        TChar::NewLine,
    ];
    assert_eq!(response.new_cursor_pos, CursorPos { x: 5, y: 0 });

    // verify the buffer is correct
    assert_eq!(buffer.data().visible, expected);
}

#[test]
fn test_calc_line_ranges() {
    let line_starts = calc_line_ranges(
        &"asdf\n0123456789\n012345678901"
            .bytes()
            .map(TChar::new_from_single_char)
            .collect::<Vec<TChar>>(),
        10,
    );
    assert_eq!(line_starts, &[0..4, 5..15, 16..26, 26..28]);
}

#[test]
fn test_buffer_padding() {
    let mut buf = b"asdf\n1234\nzxyw"
        .iter()
        .map(|&b| TChar::new_from_single_char(b))
        .collect::<Vec<TChar>>();

    let cursor_pos = CursorPos { x: 8, y: 0 };
    let response = pad_buffer_for_write(&mut buf, 10, 10, &cursor_pos, 10);
    assert_eq!(
        buf,
        "asdf              \n1234\nzxyw"
            .bytes()
            .map(TChar::new_from_single_char)
            .collect::<Vec<TChar>>()
    );
    assert_eq!(response.write_idx, 8);
    assert_eq!(response.inserted_padding, 4..18);
}

#[test]
fn test_canvas_clear_forwards() {
    let mut buffer = TerminalBufferHolder::new(5, 5);
    // Push enough data to get some in scrollback
    buffer
        .insert_data(&CursorPos { x: 0, y: 0 }, b"012343456789\n0123456789\n1234")
        .unwrap();
    let expected = vec![
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::new_from_single_char(b'8'),
        TChar::new_from_single_char(b'9'),
        TChar::NewLine,
        TChar::new_from_single_char(b'0'),
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::new_from_single_char(b'8'),
        TChar::new_from_single_char(b'9'),
        TChar::NewLine,
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::NewLine,
    ];
    assert_eq!(buffer.data().visible, expected);

    buffer.clear_forwards(&CursorPos { x: 1, y: 1 }).unwrap();
    let expected = vec![
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::new_from_single_char(b'8'),
        TChar::NewLine,
        TChar::NewLine,
        TChar::NewLine,
        TChar::NewLine,
    ];
    // Same amount of lines should be present before and after clear
    assert_eq!(buffer.data().visible, expected);

    // A few special cases.
    // 1. Truncating on beginning of line and previous char was not a newline
    let mut buffer = TerminalBufferHolder::new(5, 5);
    buffer
        .insert_data(&CursorPos { x: 0, y: 0 }, b"012340123401234012340123401234")
        .unwrap();
    buffer.clear_forwards(&CursorPos { x: 0, y: 1 }).unwrap();
    let expected = vec![
        TChar::new_from_single_char(b'0'),
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::NewLine,
        TChar::NewLine,
        TChar::NewLine,
        TChar::NewLine,
        TChar::NewLine,
    ];
    assert_eq!(buffer.data().visible, expected);

    // 2. Truncating on beginning of line and previous char was a newline
    let mut buffer = TerminalBufferHolder::new(5, 5);
    buffer
        .insert_data(
            &CursorPos { x: 0, y: 0 },
            b"01234\n0123401234012340123401234",
        )
        .unwrap();
    buffer.clear_forwards(&CursorPos { x: 0, y: 1 }).unwrap();
    assert_eq!(buffer.data().visible, expected);

    // 3. Truncating on a newline
    let mut buffer = TerminalBufferHolder::new(5, 5);
    buffer
        .insert_data(&CursorPos { x: 0, y: 0 }, b"\n\n\n\n\n\n")
        .unwrap();
    buffer.clear_forwards(&CursorPos { x: 0, y: 1 }).unwrap();
    let expected = vec![
        TChar::NewLine,
        TChar::NewLine,
        TChar::NewLine,
        TChar::NewLine,
        TChar::NewLine,
    ];
    assert_eq!(buffer.data().visible, expected);
}

#[test]
fn test_canvas_clear() {
    let mut buffer = TerminalBufferHolder::new(5, 5);
    buffer
        .insert_data(&CursorPos { x: 0, y: 0 }, b"0123456789")
        .unwrap();
    buffer.clear_all();
    assert_eq!(buffer.data().visible, &[] as &[TChar]);
}

#[test]
fn test_terminal_buffer_overwrite_early_newline() {
    let mut buffer = TerminalBufferHolder::new(5, 5);
    buffer
        .insert_data(&CursorPos { x: 0, y: 0 }, b"012\n3456789")
        .unwrap();
    let expected = vec![
        TChar::new_from_single_char(b'0'),
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::NewLine,
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::new_from_single_char(b'8'),
        TChar::new_from_single_char(b'9'),
        TChar::NewLine,
    ];
    assert_eq!(buffer.data().visible, expected);

    // Cursor pos should be calculated based off wrapping at column 5, but should not result in
    // an extra newline
    buffer
        .insert_data(&CursorPos { x: 2, y: 1 }, b"test")
        .unwrap();
    let expected = vec![
        TChar::new_from_single_char(b'0'),
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::NewLine,
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b't'),
        TChar::new_from_single_char(b'e'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b't'),
        TChar::new_from_single_char(b'9'),
        TChar::NewLine,
    ];
    assert_eq!(buffer.data().visible, expected);
}

#[test]
fn test_terminal_buffer_overwrite_no_newline() {
    let mut buffer = TerminalBufferHolder::new(5, 5);
    buffer
        .insert_data(&CursorPos { x: 0, y: 0 }, b"0123456789")
        .unwrap();
    let expected = vec![
        TChar::new_from_single_char(b'0'),
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::new_from_single_char(b'8'),
        TChar::new_from_single_char(b'9'),
        TChar::NewLine,
    ];
    assert_eq!(buffer.data().visible, expected);

    // Cursor pos should be calculated based off wrapping at column 5, but should not result in
    // an extra newline
    buffer
        .insert_data(&CursorPos { x: 2, y: 1 }, b"test")
        .unwrap();
    let expected = vec![
        TChar::new_from_single_char(b'0'),
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b't'),
        TChar::new_from_single_char(b'e'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b't'),
        TChar::NewLine,
    ];
    assert_eq!(buffer.data().visible, expected);
}

#[test]
fn test_terminal_buffer_overwrite_late_newline() {
    // This should behave exactly as test_terminal_buffer_overwrite_no_newline(), except with a
    // neline between lines 1 and 2
    let mut buffer = TerminalBufferHolder::new(5, 5);
    buffer
        .insert_data(&CursorPos { x: 0, y: 0 }, b"01234\n56789")
        .unwrap();
    let expected = vec![
        TChar::new_from_single_char(b'0'),
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::NewLine,
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::new_from_single_char(b'8'),
        TChar::new_from_single_char(b'9'),
        TChar::NewLine,
    ];
    assert_eq!(buffer.data().visible, expected);

    buffer
        .insert_data(&CursorPos { x: 2, y: 1 }, b"test")
        .unwrap();
    let expected = vec![
        TChar::new_from_single_char(b'0'),
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::NewLine,
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b't'),
        TChar::new_from_single_char(b'e'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b't'),
        TChar::NewLine,
    ];
    assert_eq!(buffer.data().visible, expected);
}

#[test]
fn test_terminal_buffer_insert_unallocated_data() {
    let mut buffer = TerminalBufferHolder::new(10, 10);
    buffer
        .insert_data(&CursorPos { x: 4, y: 5 }, b"hello world")
        .unwrap();
    let expected = vec![
        TChar::NewLine,
        TChar::NewLine,
        TChar::NewLine,
        TChar::NewLine,
        TChar::NewLine,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::new_from_single_char(b'h'),
        TChar::new_from_single_char(b'e'),
        TChar::new_from_single_char(b'l'),
        TChar::new_from_single_char(b'l'),
        TChar::new_from_single_char(b'o'),
        TChar::Space,
        TChar::new_from_single_char(b'w'),
        TChar::new_from_single_char(b'o'),
        TChar::new_from_single_char(b'r'),
        TChar::new_from_single_char(b'l'),
        TChar::new_from_single_char(b'd'),
        TChar::NewLine,
    ];
    assert_eq!(buffer.data().visible, expected);

    buffer
        .insert_data(&CursorPos { x: 3, y: 2 }, b"hello world")
        .unwrap();
    let expected = vec![
        TChar::NewLine,
        TChar::NewLine,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::new_from_single_char(b'h'),
        TChar::new_from_single_char(b'e'),
        TChar::new_from_single_char(b'l'),
        TChar::new_from_single_char(b'l'),
        TChar::new_from_single_char(b'o'),
        TChar::Space,
        TChar::new_from_single_char(b'w'),
        TChar::new_from_single_char(b'o'),
        TChar::new_from_single_char(b'r'),
        TChar::new_from_single_char(b'l'),
        TChar::new_from_single_char(b'd'),
        TChar::NewLine,
        TChar::NewLine,
        TChar::NewLine,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::new_from_single_char(b'h'),
        TChar::new_from_single_char(b'e'),
        TChar::new_from_single_char(b'l'),
        TChar::new_from_single_char(b'l'),
        TChar::new_from_single_char(b'o'),
        TChar::Space,
        TChar::new_from_single_char(b'w'),
        TChar::new_from_single_char(b'o'),
        TChar::new_from_single_char(b'r'),
        TChar::new_from_single_char(b'l'),
        TChar::new_from_single_char(b'd'),
        TChar::NewLine,
    ];
    assert_eq!(buffer.data().visible, expected,);
}

#[test]
fn test_canvas_scrolling() {
    let mut canvas = TerminalBufferHolder::new(10, 3);
    let initial_cursor_pos = CursorPos { x: 0, y: 0 };

    // Simulate real terminal usage where newlines are injected with cursor moves
    let mut response = canvas.insert_data(&initial_cursor_pos, b"asdf").unwrap();
    crlf(&mut response.new_cursor_pos);
    let mut response = canvas
        .insert_data(&response.new_cursor_pos, b"xyzw")
        .unwrap();
    crlf(&mut response.new_cursor_pos);
    let mut response = canvas
        .insert_data(&response.new_cursor_pos, b"1234")
        .unwrap();
    crlf(&mut response.new_cursor_pos);
    let mut response = canvas
        .insert_data(&response.new_cursor_pos, b"5678")
        .unwrap();
    crlf(&mut response.new_cursor_pos);

    let expeceted_scrollback = vec![
        TChar::new_from_single_char(b'a'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b'd'),
        TChar::new_from_single_char(b'f'),
        TChar::NewLine,
    ];
    let expected_visible = vec![
        TChar::new_from_single_char(b'x'),
        TChar::new_from_single_char(b'y'),
        TChar::new_from_single_char(b'z'),
        TChar::new_from_single_char(b'w'),
        TChar::NewLine,
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::NewLine,
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::new_from_single_char(b'8'),
        TChar::NewLine,
    ];
    assert_eq!(canvas.data().scrollback, expeceted_scrollback);
    assert_eq!(canvas.data().visible, expected_visible);
}

#[test]
fn test_canvas_delete_forwards() {
    let mut canvas = TerminalBufferHolder::new(10, 5);

    canvas
        .insert_data(&CursorPos { x: 0, y: 0 }, b"asdf\n123456789012345")
        .unwrap();

    // Test normal deletion
    let deleted_range = canvas.delete_forwards(&CursorPos { x: 1, y: 0 }, 1);

    assert_eq!(deleted_range, Some(1..2));
    let expected = vec![
        TChar::new_from_single_char(b'a'),
        TChar::new_from_single_char(b'd'),
        TChar::new_from_single_char(b'f'),
        TChar::NewLine,
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::new_from_single_char(b'8'),
        TChar::new_from_single_char(b'9'),
        TChar::new_from_single_char(b'0'),
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::NewLine,
    ];
    assert_eq!(canvas.data().visible, expected);

    // Test deletion clamped on newline
    let deleted_range = canvas.delete_forwards(&CursorPos { x: 1, y: 0 }, 10);
    assert_eq!(deleted_range, Some(1..3));
    let expected = vec![
        TChar::new_from_single_char(b'a'),
        TChar::NewLine,
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::new_from_single_char(b'8'),
        TChar::new_from_single_char(b'9'),
        TChar::new_from_single_char(b'0'),
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::NewLine,
    ];
    assert_eq!(canvas.data().visible, expected);

    // Test deletion clamped on wrap
    let deleted_range = canvas.delete_forwards(&CursorPos { x: 7, y: 1 }, 10);
    assert_eq!(deleted_range, Some(9..12));
    let expected = vec![
        TChar::new_from_single_char(b'a'),
        TChar::NewLine,
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::NewLine,
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::NewLine,
    ];
    assert_eq!(canvas.data().visible, expected);

    // Test deletion in case where nothing is deleted
    let deleted_range = canvas.delete_forwards(&CursorPos { x: 5, y: 5 }, 10);
    assert_eq!(deleted_range, None);
    assert_eq!(canvas.data().visible, expected);
}

#[test]
#[allow(clippy::too_many_lines)]
fn test_canvas_insert_spaces() {
    let mut canvas = TerminalBufferHolder::new(10, 5);
    canvas
        .insert_data(&CursorPos { x: 0, y: 0 }, b"asdf\n123456789012345")
        .unwrap();

    // Happy path
    let response = canvas.insert_spaces(&CursorPos { x: 2, y: 0 }, 2);
    assert_eq!(response.written_range, 2..4);
    assert_eq!(response.insertion_range, 2..4);
    assert_eq!(response.new_cursor_pos, CursorPos { x: 2, y: 0 });
    let expected = vec![
        TChar::new_from_single_char(b'a'),
        TChar::new_from_single_char(b's'),
        TChar::Space,
        TChar::Space,
        TChar::new_from_single_char(b'd'),
        TChar::new_from_single_char(b'f'),
        TChar::NewLine,
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::new_from_single_char(b'8'),
        TChar::new_from_single_char(b'9'),
        TChar::new_from_single_char(b'0'),
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::NewLine,
    ];
    assert_eq!(canvas.data().visible, expected);

    // Truncation at newline
    let response = canvas.insert_spaces(&CursorPos { x: 2, y: 0 }, 1000);
    assert_eq!(response.written_range, 2..10);
    assert_eq!(response.insertion_range, 2..6);
    assert_eq!(response.new_cursor_pos, CursorPos { x: 2, y: 0 });
    let expected = vec![
        TChar::new_from_single_char(b'a'),
        TChar::new_from_single_char(b's'),
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::NewLine,
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::new_from_single_char(b'8'),
        TChar::new_from_single_char(b'9'),
        TChar::new_from_single_char(b'0'),
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::NewLine,
    ];
    assert_eq!(canvas.data().visible, expected);

    // Truncation at line wrap
    let response = canvas.insert_spaces(&CursorPos { x: 4, y: 1 }, 1000);
    assert_eq!(response.written_range, 15..21);
    assert_eq!(
        response.insertion_range.start - response.insertion_range.end,
        0
    );
    assert_eq!(response.new_cursor_pos, CursorPos { x: 4, y: 1 });
    let expected = vec![
        TChar::new_from_single_char(b'a'),
        TChar::new_from_single_char(b's'),
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::NewLine,
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::NewLine,
    ];

    assert_eq!(canvas.data().visible, expected);

    // Insertion at non-existent buffer pos
    let response = canvas.insert_spaces(&CursorPos { x: 2, y: 4 }, 3);
    assert_eq!(response.written_range, 30..33);
    assert_eq!(response.insertion_range, 27..34);
    assert_eq!(response.new_cursor_pos, CursorPos { x: 2, y: 4 });
    let expected = vec![
        TChar::new_from_single_char(b'a'),
        TChar::new_from_single_char(b's'),
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::NewLine,
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::NewLine,
        TChar::NewLine,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::NewLine,
    ];

    assert_eq!(canvas.data().visible, expected);
}

#[test]
fn test_clear_line_forwards() {
    let mut canvas = TerminalBufferHolder::new(10, 5);
    canvas
        .insert_data(&CursorPos { x: 0, y: 0 }, b"asdf\n123456789012345")
        .unwrap();

    // Nothing do delete
    let response = canvas.clear_line_forwards(&CursorPos { x: 5, y: 5 });
    assert_eq!(response, None);
    let expected = vec![
        TChar::new_from_single_char(b'a'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b'd'),
        TChar::new_from_single_char(b'f'),
        TChar::NewLine,
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::new_from_single_char(b'8'),
        TChar::new_from_single_char(b'9'),
        TChar::new_from_single_char(b'0'),
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::NewLine,
    ];
    assert_eq!(canvas.data().visible, expected);

    // Hit a newline
    let response = canvas.clear_line_forwards(&CursorPos { x: 2, y: 0 });
    assert_eq!(response, Some(2..4));
    let expected = vec![
        TChar::new_from_single_char(b'a'),
        TChar::new_from_single_char(b's'),
        TChar::NewLine,
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::new_from_single_char(b'8'),
        TChar::new_from_single_char(b'9'),
        TChar::new_from_single_char(b'0'),
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::NewLine,
    ];
    assert_eq!(canvas.data().visible, expected);

    // Hit a wrap
    let response = canvas.clear_line_forwards(&CursorPos { x: 2, y: 1 });
    assert_eq!(response, Some(5..13));
    let expected = vec![
        TChar::new_from_single_char(b'a'),
        TChar::new_from_single_char(b's'),
        TChar::NewLine,
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::NewLine,
    ];
    assert_eq!(canvas.data().visible, expected);
}

#[test]
fn test_resize_expand() {
    // Ensure that on window size increase, text stays in same spot relative to cursor position
    // This was problematic with our initial implementation. It's less of a problem after some
    // later improvements, but we can keep the test to make sure it still seems sane
    let mut canvas = TerminalBufferHolder::new(10, 6);

    let cursor_pos = CursorPos { x: 0, y: 0 };
    let response = simulate_resize(&mut canvas, 10, 5, &cursor_pos).unwrap();
    let response = simulate_resize(&mut canvas, 10, 4, &response.new_cursor_pos).unwrap();
    let response = simulate_resize(&mut canvas, 10, 3, &response.new_cursor_pos).unwrap();
    simulate_resize(&mut canvas, 10, 5, &response.new_cursor_pos).unwrap();
    let expected = vec![
        TChar::new_from_single_char(b'$'),
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::NewLine,
    ];
    assert_eq!(canvas.data().visible, expected);
}

#[test]
fn test_insert_lines() {
    let mut canvas = TerminalBufferHolder::new(5, 5);

    // Test empty canvas
    let response = canvas.insert_lines(&CursorPos { x: 0, y: 0 }, 3);
    // Clear doesn't have to do anything as there's nothing in the canvas to push aside
    assert_eq!(response.deleted_range.start - response.deleted_range.end, 0);
    assert_eq!(
        response.inserted_range.start - response.inserted_range.end,
        0
    );
    assert_eq!(canvas.data().visible, b"");

    // Test edge wrapped
    canvas
        .insert_data(&CursorPos { x: 0, y: 0 }, b"0123456789asdf\nxyzw")
        .unwrap();
    let expected = vec![
        TChar::new_from_single_char(b'0'),
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::new_from_single_char(b'8'),
        TChar::new_from_single_char(b'9'),
        TChar::new_from_single_char(b'a'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b'd'),
        TChar::new_from_single_char(b'f'),
        TChar::NewLine,
        TChar::new_from_single_char(b'x'),
        TChar::new_from_single_char(b'y'),
        TChar::new_from_single_char(b'z'),
        TChar::new_from_single_char(b'w'),
        TChar::NewLine,
    ];

    assert_eq!(canvas.data().visible, expected);
    let response = canvas.insert_lines(&CursorPos { x: 3, y: 2 }, 1);
    let expected = vec![
        TChar::new_from_single_char(b'0'),
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::new_from_single_char(b'8'),
        TChar::new_from_single_char(b'9'),
        TChar::NewLine,
        TChar::NewLine,
        TChar::new_from_single_char(b'a'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b'd'),
        TChar::new_from_single_char(b'f'),
        TChar::NewLine,
        TChar::new_from_single_char(b'x'),
        TChar::new_from_single_char(b'y'),
        TChar::new_from_single_char(b'z'),
        TChar::new_from_single_char(b'w'),
        TChar::NewLine,
    ];

    assert_eq!(canvas.data().visible, expected);
    assert_eq!(response.deleted_range.start - response.deleted_range.end, 0);
    assert_eq!(response.inserted_range, 10..12);

    // Test newline wrapped + lines pushed off the edge
    let response = canvas.insert_lines(&CursorPos { x: 3, y: 2 }, 1);
    let expected = vec![
        TChar::new_from_single_char(b'0'),
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::new_from_single_char(b'8'),
        TChar::new_from_single_char(b'9'),
        TChar::NewLine,
        TChar::NewLine,
        TChar::NewLine,
        TChar::new_from_single_char(b'a'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b'd'),
        TChar::new_from_single_char(b'f'),
        TChar::NewLine,
    ];

    assert_eq!(canvas.data().visible, expected);
    assert_eq!(response.deleted_range, 17..22);
    assert_eq!(response.inserted_range, 11..12);
}

#[test]
fn test_clear_line() {
    let mut canvas = TerminalBufferHolder::new(5, 5);

    // Test empty canvas
    let response = canvas.clear_line(&CursorPos { x: 0, y: 0 });
    assert_eq!(response, None);
    assert_eq!(canvas.data().visible, b"");

    // Test edge wrapped
    canvas
        .insert_data(&CursorPos { x: 0, y: 0 }, b"0123456789asdf\nxyzw")
        .unwrap();
    let expected = vec![
        TChar::new_from_single_char(b'0'),
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::new_from_single_char(b'8'),
        TChar::new_from_single_char(b'9'),
        TChar::new_from_single_char(b'a'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b'd'),
        TChar::new_from_single_char(b'f'),
        TChar::NewLine,
        TChar::new_from_single_char(b'x'),
        TChar::new_from_single_char(b'y'),
        TChar::new_from_single_char(b'z'),
        TChar::new_from_single_char(b'w'),
        TChar::NewLine,
    ];

    assert_eq!(canvas.data().visible, expected);
    let response = canvas.clear_line(&CursorPos { x: 0, y: 0 });
    let expected = vec![
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::new_from_single_char(b'8'),
        TChar::new_from_single_char(b'9'),
        TChar::new_from_single_char(b'a'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b'd'),
        TChar::new_from_single_char(b'f'),
        TChar::NewLine,
        TChar::new_from_single_char(b'x'),
        TChar::new_from_single_char(b'y'),
        TChar::new_from_single_char(b'z'),
        TChar::new_from_single_char(b'w'),
        TChar::NewLine,
    ];

    assert_eq!(canvas.data().visible, expected);
    assert_eq!(response, Some(0..5));

    // Test newline wrapped
    let response = canvas.clear_line(&CursorPos { x: 0, y: 1 });
    let expected = vec![
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::new_from_single_char(b'8'),
        TChar::new_from_single_char(b'9'),
        // TChar::new_from_single_char(b'a'),
        // TChar::new_from_single_char(b's'),
        // TChar::new_from_single_char(b'd'),
        // TChar::new_from_single_char(b'f'),
        TChar::NewLine,
        TChar::new_from_single_char(b'x'),
        TChar::new_from_single_char(b'y'),
        TChar::new_from_single_char(b'z'),
        TChar::new_from_single_char(b'w'),
        TChar::NewLine,
    ];
    assert_eq!(canvas.data().visible, expected);
    assert_eq!(response, Some(5..9));
}

#[test]
fn clear_line_backwards() {
    let mut canvas = TerminalBufferHolder::new(5, 5);

    // Test empty canvas
    let response = canvas.clear_line_backwards(&CursorPos { x: 0, y: 0 });
    assert_eq!(response, None);
    assert_eq!(canvas.data().visible, b"");

    // Test edge wrapped
    canvas
        .insert_data(&CursorPos { x: 0, y: 0 }, b"0123456789asdf\nxyzw")
        .unwrap();
    let expected = vec![
        TChar::new_from_single_char(b'0'),
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::new_from_single_char(b'8'),
        TChar::new_from_single_char(b'9'),
        TChar::new_from_single_char(b'a'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b'd'),
        TChar::new_from_single_char(b'f'),
        TChar::NewLine,
        TChar::new_from_single_char(b'x'),
        TChar::new_from_single_char(b'y'),
        TChar::new_from_single_char(b'z'),
        TChar::new_from_single_char(b'w'),
        TChar::NewLine,
    ];

    assert_eq!(canvas.data().visible, expected);
    let response = canvas.clear_line_backwards(&CursorPos { x: 3, y: 0 });
    let expected = vec![
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::new_from_single_char(b'8'),
        TChar::new_from_single_char(b'9'),
        TChar::new_from_single_char(b'a'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b'd'),
        TChar::new_from_single_char(b'f'),
        TChar::NewLine,
        TChar::new_from_single_char(b'x'),
        TChar::new_from_single_char(b'y'),
        TChar::new_from_single_char(b'z'),
        TChar::new_from_single_char(b'w'),
        TChar::NewLine,
    ];

    assert_eq!(canvas.data().visible, expected);
    assert_eq!(response, Some(0..3));
}

#[test]
fn test_clear_backwards() {
    let mut canvas = TerminalBufferHolder::new(5, 5);

    // Test empty canvas
    let response = canvas.clear_backwards(&CursorPos { x: 0, y: 0 }).unwrap();
    assert_eq!(response, None);
    assert_eq!(canvas.data().visible, b"");

    // Test edge wrapped
    canvas
        .insert_data(&CursorPos { x: 0, y: 0 }, b"0123456789asdf\nxyzw")
        .unwrap();
    let expected = vec![
        TChar::new_from_single_char(b'0'),
        TChar::new_from_single_char(b'1'),
        TChar::new_from_single_char(b'2'),
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::new_from_single_char(b'8'),
        TChar::new_from_single_char(b'9'),
        TChar::new_from_single_char(b'a'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b'd'),
        TChar::new_from_single_char(b'f'),
        TChar::NewLine,
        TChar::new_from_single_char(b'x'),
        TChar::new_from_single_char(b'y'),
        TChar::new_from_single_char(b'z'),
        TChar::new_from_single_char(b'w'),
        TChar::NewLine,
    ];

    assert_eq!(canvas.data().visible, expected);
    let response = canvas.clear_backwards(&CursorPos { x: 3, y: 0 }).unwrap();
    let expected = vec![
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::new_from_single_char(b'3'),
        TChar::new_from_single_char(b'4'),
        TChar::new_from_single_char(b'5'),
        TChar::new_from_single_char(b'6'),
        TChar::new_from_single_char(b'7'),
        TChar::new_from_single_char(b'8'),
        TChar::new_from_single_char(b'9'),
        TChar::new_from_single_char(b'a'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b'd'),
        TChar::new_from_single_char(b'f'),
        TChar::NewLine,
        TChar::new_from_single_char(b'x'),
        TChar::new_from_single_char(b'y'),
        TChar::new_from_single_char(b'z'),
        TChar::new_from_single_char(b'w'),
        TChar::NewLine,
    ];

    assert_eq!(canvas.data().visible, expected);
    assert_eq!(response, Some(0..3));

    // clearing on the second line
    let response = canvas.clear_backwards(&CursorPos { x: 3, y: 1 }).unwrap();
    let expected = vec![
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::new_from_single_char(b'8'),
        TChar::new_from_single_char(b'9'),
        TChar::new_from_single_char(b'a'),
        TChar::new_from_single_char(b's'),
        TChar::new_from_single_char(b'd'),
        TChar::new_from_single_char(b'f'),
        TChar::NewLine,
        TChar::new_from_single_char(b'x'),
        TChar::new_from_single_char(b'y'),
        TChar::new_from_single_char(b'z'),
        TChar::new_from_single_char(b'w'),
        TChar::NewLine,
    ];

    assert_eq!(canvas.data().visible, expected);
    assert_eq!(response, Some(5..8));
}

#[test]
fn test_clear_visible() {
    let mut canvas = TerminalBufferHolder::new(5, 5);

    // Test edge wrapped
    canvas
        .insert_data(
            &CursorPos { x: 0, y: 0 },
            b"0123456789asdf0123456789asdf0123456789asdf0123456789asdf0123456789asdf\nxyzw",
        )
        .unwrap();

    let response = canvas.clear_visible();
    let expected: Vec<TChar> = vec![
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::NewLine,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::Space,
        TChar::NewLine,
    ];
    assert_eq!(canvas.data().visible, expected);
    assert_eq!(response, 50..usize::MAX);
}
