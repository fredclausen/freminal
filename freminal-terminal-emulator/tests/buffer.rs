use std::ops::Range;

// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.
#[cfg(test)]
use anyhow::Result;
use freminal_terminal_emulator::state::{
    buffer::{TerminalBufferHolder, TerminalBufferInsertResponse},
    cursor::CursorPos,
    term_char::TChar,
};

/// Calculate the indexes of the start and end of each line in the buffer given an input width.
/// Ranges do not include newlines. If a newline appears past the width, it does not result in an
/// extra line
#[must_use]
fn calc_line_ranges(buf: &[TChar], width: usize, last_capacity: &usize) -> Vec<Range<usize>> {
    //let mut ret = vec![];

    let mut current_start = 0;

    let mut ret = Vec::with_capacity(*last_capacity + 10);

    for (current_pos, c) in buf.iter().enumerate() {
        if c == &TChar::NewLine {
            ret.push(current_start..current_pos);
            current_start = current_pos + 1;
        } else if current_pos - current_start == width {
            ret.push(current_start..current_pos);
            current_start = current_pos;
        }
    }

    if buf.len() > current_start {
        ret.push(current_start..buf.len());
    }

    ret
}

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
    assert_eq!(buffer.data(true).visible, expected);

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
    assert_eq!(buffer.data(true).visible, expected);
}

#[test]
fn test_calc_line_ranges() {
    let line_starts = calc_line_ranges(
        &"asdf\n0123456789\n012345678901"
            .bytes()
            .map(TChar::new_from_single_char)
            .collect::<Vec<TChar>>(),
        10,
        &0,
    );
    assert_eq!(line_starts, &[0..4, 5..15, 16..26, 26..28]);
}

// FIXME: This test is broken
// #[test]
// fn test_buffer_padding() {
//     let mut buf = b"asdf\n1234\nzxyw"
//         .iter()
//         .map(|&b| TChar::new_from_single_char(b))
//         .collect::<Vec<TChar>>();

//     let cursor_pos = CursorPos { x: 8, y: 0 };
//     let visible_line_ranges = calc_line_ranges(&buf, 10, &0);
//     let response = pad_buffer_for_write(&mut buf, &visible_line_ranges, &cursor_pos, 10);
//     assert_eq!(
//         buf,
//         "asdf              \n1234\nzxyw"
//             .bytes()
//             .map(TChar::new_from_single_char)
//             .collect::<Vec<TChar>>()
//     );
//     assert_eq!(response.write_idx, 8);
//     assert_eq!(response.inserted_padding, 4..18);
// }

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

    assert_eq!(buffer.data(true).visible, expected);

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
    assert_eq!(buffer.data(true).visible, expected);

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
    assert_eq!(buffer.data(true).visible, expected);

    // 2. Truncating on beginning of line and previous char was a newline
    let mut buffer = TerminalBufferHolder::new(5, 5);
    buffer
        .insert_data(
            &CursorPos { x: 0, y: 0 },
            b"01234\n0123401234012340123401234",
        )
        .unwrap();

    buffer.clear_forwards(&CursorPos { x: 0, y: 1 }).unwrap();
    assert_eq!(buffer.data(true).visible, expected);

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
    assert_eq!(buffer.data(true).visible, expected);
}

#[test]
fn test_canvas_clear() {
    let mut buffer = TerminalBufferHolder::new(5, 5);
    buffer
        .insert_data(&CursorPos { x: 0, y: 0 }, b"0123456789")
        .unwrap();
    buffer.clear_all();
    assert_eq!(buffer.data(true).visible, &[] as &[TChar]);
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
    assert_eq!(buffer.data(true).visible, expected);

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
    assert_eq!(buffer.data(true).visible, expected);
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
    assert_eq!(buffer.data(true).visible, expected);

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
    assert_eq!(buffer.data(true).visible, expected);
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
    assert_eq!(buffer.data(true).visible, expected);

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
    assert_eq!(buffer.data(true).visible, expected);
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
    assert_eq!(buffer.data(true).visible, expected);

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
    assert_eq!(buffer.data(true).visible, expected,);
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

    let expected_scrollback = vec![
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
    assert_eq!(canvas.data(true).scrollback, expected_scrollback);
    assert_eq!(canvas.data(true).visible, expected_visible);
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

    assert_eq!(canvas.data(true).visible, expected);

    canvas.line_ranges_to_visible_line_ranges();

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
    assert_eq!(canvas.data(true).visible, expected);

    canvas.line_ranges_to_visible_line_ranges();

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
    assert_eq!(canvas.data(true).visible, expected);

    // Test deletion in case where nothing is deleted
    let deleted_range = canvas.delete_forwards(&CursorPos { x: 5, y: 5 }, 10);
    assert_eq!(deleted_range, None);
    assert_eq!(canvas.data(true).visible, expected);
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

    canvas.line_ranges_to_visible_line_ranges();

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
    assert_eq!(canvas.data(true).visible, expected);

    canvas.line_ranges_to_visible_line_ranges();

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
    assert_eq!(canvas.data(true).visible, expected);

    canvas.line_ranges_to_visible_line_ranges();

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

    assert_eq!(canvas.data(true).visible, expected);

    canvas.line_ranges_to_visible_line_ranges();

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

    assert_eq!(canvas.data(true).visible, expected);
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
    assert_eq!(canvas.data(true).visible, expected);

    canvas.line_ranges_to_visible_line_ranges();

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
    assert_eq!(canvas.data(true).visible, expected);

    canvas.line_ranges_to_visible_line_ranges();

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
    assert_eq!(canvas.data(true).visible, expected);
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
    assert_eq!(canvas.data(true).visible, expected);
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
    assert_eq!(canvas.data(true).visible, b"");

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

    canvas.line_ranges_to_visible_line_ranges();

    assert_eq!(canvas.data(true).visible, expected);
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

    assert_eq!(canvas.data(true).visible, expected);
    assert_eq!(response.deleted_range.start - response.deleted_range.end, 0);
    assert_eq!(response.inserted_range, 10..12);

    canvas.line_ranges_to_visible_line_ranges();

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

    assert_eq!(canvas.data(true).visible, expected);
    assert_eq!(response.deleted_range, 17..22);
    assert_eq!(response.inserted_range, 11..12);
}

#[test]
fn test_clear_line() {
    let mut canvas = TerminalBufferHolder::new(5, 5);

    // Test empty canvas
    let response = canvas.clear_line(&CursorPos { x: 0, y: 0 });
    assert_eq!(response, None);
    assert_eq!(canvas.data(true).visible, b"");

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

    assert_eq!(canvas.data(true).visible, expected);

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

    assert_eq!(canvas.data(true).visible, expected);
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
    assert_eq!(canvas.data(true).visible, expected);
    assert_eq!(response, Some(5..9));
}

#[test]
fn clear_line_backwards() {
    let mut canvas = TerminalBufferHolder::new(5, 5);

    // Test empty canvas
    let response = canvas.clear_line_backwards(&CursorPos { x: 0, y: 0 });
    assert_eq!(response, None);
    assert_eq!(canvas.data(true).visible, b"");

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

    assert_eq!(canvas.data(true).visible, expected);
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

    assert_eq!(canvas.data(true).visible, expected);
    assert_eq!(response, Some(0..3));
}

#[test]
fn test_clear_backwards() {
    let mut canvas = TerminalBufferHolder::new(5, 5);

    // Test empty canvas
    let response = canvas.clear_backwards(&CursorPos { x: 0, y: 0 }).unwrap();
    assert_eq!(response, None);
    assert_eq!(canvas.data(true).visible, b"");

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

    assert_eq!(canvas.data(true).visible, expected);
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

    assert_eq!(canvas.data(true).visible, expected);
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

    assert_eq!(canvas.data(true).visible, expected);
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

    let response = canvas.clear_visible().unwrap();
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
    assert_eq!(canvas.data(true).visible, expected);
    assert_eq!(response, 50..usize::MAX);
}

#[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
#[test]
fn test_visible_line_ranges_parsing() {
    let mut buf = TerminalBufferHolder::new(5, 5);
    let data = vec![
        TChar::Ascii(b'0'), // 0 1
        TChar::Ascii(b'1'), // 1 1
        TChar::Ascii(b'2'), // 2 1
        TChar::Ascii(b'3'), // 3 1
        TChar::Ascii(b'4'), // 4 1
        TChar::Ascii(b'3'), // 5 2
        TChar::Ascii(b'4'), // 6 2
        TChar::Ascii(b'5'), // 7 2
        TChar::Ascii(b'6'), // 8 2
        TChar::Ascii(b'7'), // 9 2
        TChar::Ascii(b'8'), // 10 3
        TChar::NewLine,     // 11
        TChar::NewLine,     // 12 4
        TChar::NewLine,     // 13 5
        TChar::NewLine,     // 14 6
    ];
    buf.buf = data;
    buf.line_ranges_to_visible_line_ranges();

    let expected = [5..10, 10..11, 12..12, 13..13, 14..14];

    let visible_line_ranges = buf.get_visible_line_ranges();

    assert_eq!(visible_line_ranges.len(), 5);
    assert_eq!(visible_line_ranges[0], expected[0]);
    assert_eq!(visible_line_ranges[1], expected[1]);
    assert_eq!(visible_line_ranges[2], expected[2]);
    assert_eq!(visible_line_ranges[3], expected[3]);
    assert_eq!(visible_line_ranges[4], expected[4]);

    let mut buf = TerminalBufferHolder::new(15, 15);

    // no scrollback, new line before width reached
    let data = b"0123456789\n0123456789\n0123456789\n0123456789\n0123456789\n";
    buf.insert_data(&CursorPos { x: 0, y: 0 }, data).unwrap();

    let visible_line_ranges = buf.get_visible_line_ranges();

    assert_eq!(visible_line_ranges.len(), 6);
    assert_eq!(visible_line_ranges[0], 0..10);
    assert_eq!(visible_line_ranges[1], 11..21);
    assert_eq!(visible_line_ranges[2], 22..32);
    assert_eq!(visible_line_ranges[3], 33..43);
    assert_eq!(visible_line_ranges[4], 44..54);
    assert_eq!(visible_line_ranges[5], 55..55);

    // scrollback, new line before width reached
    let mut buf = TerminalBufferHolder::new(15, 4);

    let data = b"0123456789\n0123456789\n0123456789\n0123456789\n0123456789\n";
    buf.insert_data(&CursorPos { x: 0, y: 0 }, data).unwrap();
    let visible_line_ranges = buf.get_visible_line_ranges();

    assert_eq!(visible_line_ranges.len(), 4);
    assert_eq!(visible_line_ranges[0], 22..32);
    assert_eq!(visible_line_ranges[1], 33..43);
    assert_eq!(visible_line_ranges[2], 44..54);
    assert_eq!(visible_line_ranges[3], 55..55);

    // no scrollback, new line after width reached
    let mut buf = TerminalBufferHolder::new(10, 15);

    let data = b"0123456789\n0123456789\n0123456789\n0123456789\n0123456789\n";
    buf.insert_data(&CursorPos { x: 0, y: 0 }, data).unwrap();
    let visible_line_ranges = buf.get_visible_line_ranges();

    assert_eq!(visible_line_ranges.len(), 6);
    assert_eq!(visible_line_ranges[0], 0..10);
    assert_eq!(visible_line_ranges[1], 11..21);
    assert_eq!(visible_line_ranges[2], 22..32);
    assert_eq!(visible_line_ranges[3], 33..43);
    assert_eq!(visible_line_ranges[4], 44..54);
    assert_eq!(visible_line_ranges[5], 55..55);

    // scrollback, new line after width reached
    let mut buf = TerminalBufferHolder::new(15, 4);
    let data = b"0123456789\n0123456789\n0123456789\n0123456789\n0123456789\n";
    buf.insert_data(&CursorPos { x: 0, y: 0 }, data).unwrap();
    let visible_line_ranges = buf.get_visible_line_ranges();

    assert_eq!(visible_line_ranges.len(), 4);
    assert_eq!(visible_line_ranges[0], 22..32);
    assert_eq!(visible_line_ranges[1], 33..43);
    assert_eq!(visible_line_ranges[2], 44..54);
    assert_eq!(visible_line_ranges[3], 55..55);

    // no scrollback, no new lines
    let mut buf = TerminalBufferHolder::new(10, 15);
    let data = b"01234567890123456789012345678901234567890123456789";
    buf.insert_data(&CursorPos { x: 0, y: 0 }, data).unwrap();
    let visible_line_ranges = buf.get_visible_line_ranges();

    assert_eq!(visible_line_ranges.len(), 5);
    assert_eq!(visible_line_ranges[0], 0..10);
    assert_eq!(visible_line_ranges[1], 10..20);
    assert_eq!(visible_line_ranges[2], 20..30);
    assert_eq!(visible_line_ranges[3], 30..40);
    assert_eq!(visible_line_ranges[4], 40..50);

    // scrollback, no new lines
    let mut buf = TerminalBufferHolder::new(10, 4);
    let data = b"01234567890123456789012345678901234567890123456789";
    buf.insert_data(&CursorPos { x: 0, y: 0 }, data).unwrap();
    let visible_line_ranges = buf.get_visible_line_ranges();

    assert_eq!(visible_line_ranges.len(), 4);
    assert_eq!(visible_line_ranges[0], 10..20);
    assert_eq!(visible_line_ranges[1], 20..30);
    assert_eq!(visible_line_ranges[2], 30..40);
    assert_eq!(visible_line_ranges[3], 40..50);

    let mut buffer = TerminalBufferHolder::new(5, 5);
    // Push enough data to get some in scrollback
    buffer
        .insert_data(&CursorPos { x: 0, y: 0 }, b"012343456789\n0123456789\n1234")
        .unwrap();
    let visible_line_ranges = buffer.get_visible_line_ranges();

    assert_eq!(visible_line_ranges.len(), 5);
    assert_eq!(visible_line_ranges[0], 5..10);
    assert_eq!(visible_line_ranges[1], 10..12);
    assert_eq!(visible_line_ranges[2], 13..18);
    assert_eq!(visible_line_ranges[3], 18..23);
    assert_eq!(visible_line_ranges[4], 24..28);
}

#[test]
fn test_line_ranges_from_visible_line_ranges_no_spill() {
    // buffer with initial data that does not spill in to scrollback
    let mut buffer = TerminalBufferHolder::new(5, 5);
    // add some data
    let data = b"1234\n".repeat(4);
    let result = buffer.insert_data(&CursorPos::default(), &data).unwrap();

    // buffer_line_ranges should have 5 lines. Visible line ranges should also have 5 lines
    assert_eq!(buffer.get_line_ranges().len(), 5);
    assert_eq!(buffer.get_visible_line_ranges().len(), 5);
    assert_eq!(buffer.get_visible_line_ranges(), buffer.get_line_ranges());

    // push data in to scrollback
    buffer.insert_data(&result.new_cursor_pos, &data).unwrap();
    // buffer_line_ranges should have 10 lines. Visible line ranges should have 5 lines
    assert_eq!(buffer.get_line_ranges().len(), 10);
    assert_eq!(buffer.get_visible_line_ranges().len(), 5);
    assert_eq!(
        buffer.get_visible_line_ranges(),
        [20..24, 25..29, 30..34, 35..39, 40..40]
    );
    assert_eq!(
        buffer.get_line_ranges(),
        [
            0..4,
            5..9,
            10..14,
            15..19,
            20..20,
            20..24,
            25..29,
            30..34,
            35..39,
            40..40
        ]
    );
}

#[test]
fn test_line_ranges_from_visible_line_ranges_spill() {
    // buffer with initial data that does not spill in to scrollback
    let mut buffer = TerminalBufferHolder::new(5, 5);
    // add some data
    let data = b"1234\n".repeat(5);
    let result = buffer.insert_data(&CursorPos::default(), &data).unwrap();

    // buffer_line_ranges should have 5 lines. Visible line ranges should also have 5 lines
    assert_eq!(buffer.get_line_ranges().len(), 6);
    assert_eq!(buffer.get_visible_line_ranges().len(), 5);
    assert_eq!(
        buffer.get_visible_line_ranges(),
        [5..9, 10..14, 15..19, 20..24, 25..25]
    );
    assert_eq!(
        buffer.get_line_ranges(),
        [0..4, 5..9, 10..14, 15..19, 20..24, 25..25]
    );

    // push data in to scrollback
    buffer.insert_data(&result.new_cursor_pos, &data).unwrap();
    // buffer_line_ranges should have 10 lines. Visible line ranges should have 5 lines
    assert_eq!(buffer.get_line_ranges().len(), 11);
    assert_eq!(buffer.get_visible_line_ranges().len(), 5);
    assert_eq!(
        buffer.get_visible_line_ranges(),
        [30..34, 35..39, 40..44, 45..49, 50..50]
    );
    assert_eq!(
        buffer.get_line_ranges(),
        [
            0..4,
            5..9,
            10..14,
            15..19,
            20..24,
            25..29,
            30..34,
            35..39,
            40..44,
            45..49,
            50..50
        ]
    );

    // now lets test with buffers that wrap and don't have newlines
    let mut buffer = TerminalBufferHolder::new(5, 5);
    let data = b"12345".repeat(6);
    let result = buffer.insert_data(&CursorPos::default(), &data).unwrap();

    assert_eq!(buffer.get_line_ranges().len(), 6);
    assert_eq!(buffer.get_visible_line_ranges().len(), 5);
    assert_eq!(
        buffer.get_visible_line_ranges(),
        [5..10, 10..15, 15..20, 20..25, 25..30]
    );
    assert_eq!(
        buffer.get_line_ranges(),
        [0..5, 5..10, 10..15, 15..20, 20..25, 25..30]
    );

    buffer.insert_data(&result.new_cursor_pos, &data).unwrap();
    assert_eq!(buffer.get_line_ranges().len(), 12);
    assert_eq!(buffer.get_visible_line_ranges().len(), 5);
    assert_eq!(
        buffer.get_visible_line_ranges(),
        [35..40, 40..45, 45..50, 50..55, 55..60]
    );
    assert_eq!(
        buffer.get_line_ranges(),
        [
            0..5,
            5..10,
            10..15,
            15..20,
            20..25,
            25..30,
            30..35,
            35..40,
            40..45,
            45..50,
            50..55,
            55..60
        ]
    );
}
