// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::{borrow::Cow, ops::Range};
use thiserror::Error;

use super::{CursorPos, TerminalData};

/// Calculate the indexes of the start and end of each line in the buffer given an input width.
/// Ranges do not include newlines. If a newline appears past the width, it does not result in an
/// extra line
///
/// Example
/// ```
/// let ranges = calc_line_ranges(b"12\n1234\n12345", 4);
/// assert_eq!(ranges, [0..2, 3..7, 8..11, 12..13]);
/// ```
fn calc_line_ranges(buf: &[u8], width: usize) -> Vec<Range<usize>> {
    let mut ret = vec![];
    let mut current_start = 0;

    for (i, c) in buf.iter().enumerate() {
        if *c == b'\n' {
            ret.push(current_start..i);
            current_start = i + 1;
            continue;
        }

        let bytes_since_start = i - current_start;
        // assert!(bytes_since_start <= width);
        if bytes_since_start >= width {
            // verify current_start..i is a valid utf8 string
            match std::str::from_utf8(&buf[current_start..i]) {
                Ok(s) => {
                    if s.chars().count() >= width {
                        ret.push(current_start..i);
                        current_start = i;
                    }
                }
                Err(_e) => (),
            }
            continue;
        }
    }

    if buf.len() > current_start {
        ret.push(current_start..buf.len());
    }
    ret
}

#[derive(Debug, Error, Eq, PartialEq)]
#[error("invalid buffer position {buf_pos} for buffer of len {buf_len}")]
struct InvalidBufPos {
    buf_pos: usize,
    buf_len: usize,
}

fn buf_to_cursor_pos(
    buf: &[u8],
    width: usize,
    height: usize,
    buf_pos: usize,
) -> Result<CursorPos, InvalidBufPos> {
    let new_line_ranges = calc_line_ranges(buf, width);
    let new_visible_line_ranges = line_ranges_to_visible_line_ranges(&new_line_ranges, height);
    let (new_cursor_y, new_cursor_line) = new_visible_line_ranges
        .iter()
        .enumerate()
        .find(|(_i, r)| r.end >= buf_pos)
        .ok_or(InvalidBufPos {
            buf_pos,
            buf_len: buf.len(),
        })?;

    if buf_pos < new_cursor_line.start {
        info!("Old cursor position no longer on screen");
        return Ok(CursorPos::default());
    };

    let new_cursor_x = buf_pos - new_cursor_line.start;

    // We need to know the number of **visible** characters on the line. This is different from the bytes on the line
    // FIXME: can we do this without creating a new string?
    // and also should this be lossy or nah?
    let new_cursor_x_as_character_pos = match String::from_utf8_lossy(&buf[new_cursor_line.clone()])
    {
        Cow::Borrowed(s) => s.chars().count(),
        Cow::Owned(s) => s.chars().count(),
    };

    info!("The line: {:?}", &buf[new_cursor_line.clone()]);

    Ok(CursorPos {
        x: new_cursor_x,
        y: new_cursor_y,
        x_as_characters: new_cursor_x_as_character_pos,
    })
}

fn unwrapped_line_end_pos(buf: &[u8], start_pos: usize) -> usize {
    buf.iter()
        .enumerate()
        .skip(start_pos)
        .find_map(|(i, c)| match *c {
            b'\n' => Some(i),
            _ => None,
        })
        .unwrap_or(buf.len())
}

/// Given terminal height `height`, extract the visible line ranges from all line ranges (which
/// include scrollback) assuming "visible" is the bottom N lines
fn line_ranges_to_visible_line_ranges(
    line_ranges: &[Range<usize>],
    height: usize,
) -> &[Range<usize>] {
    if line_ranges.is_empty() {
        return line_ranges;
    }
    let num_lines = line_ranges.len();
    let first_visible_line = num_lines.saturating_sub(height);
    &line_ranges[first_visible_line..]
}

struct PadBufferForWriteResponse {
    /// Where to copy data into
    write_idx: usize,
    /// Indexes where we added data
    inserted_padding: Range<usize>,
}

fn pad_buffer_for_write(
    buf: &mut Vec<u8>,
    width: usize,
    height: usize,
    cursor_pos: &CursorPos,
    write_len: usize,
) -> PadBufferForWriteResponse {
    let mut visible_line_ranges = {
        // Calculate in block scope to avoid accidental usage of scrollback line ranges later
        let line_ranges = calc_line_ranges(buf, width);
        line_ranges_to_visible_line_ranges(&line_ranges, height).to_vec()
    };

    let mut padding_start_pos = None;
    let mut num_inserted_characters = 0;

    let vertical_padding_needed = if cursor_pos.y + 1 > visible_line_ranges.len() {
        cursor_pos.y + 1 - visible_line_ranges.len()
    } else {
        0
    };

    if vertical_padding_needed != 0 {
        padding_start_pos = Some(buf.len());
        num_inserted_characters += vertical_padding_needed;
    }

    for _ in 0..vertical_padding_needed {
        buf.push(b'\n');
        let newline_pos = buf.len() - 1;
        visible_line_ranges.push(newline_pos..newline_pos);
    }

    let line_range = &visible_line_ranges[cursor_pos.y];

    let desired_start = line_range.start + cursor_pos.x;
    let desired_end = desired_start + write_len;

    // NOTE: We only want to pad if we hit an early newline. If we wrapped because we hit the edge
    // of the screen we can just keep writing and the wrapping will stay as is. This is an
    // important distinction because in the no-newline case we want to make sure we overwrite
    // whatever was in the buffer before
    let actual_end = unwrapped_line_end_pos(buf, line_range.start);

    // If we did not set the padding start position, it means that we are padding not at the end of
    // the buffer, but at the end of a line
    if padding_start_pos.is_none() {
        padding_start_pos = Some(actual_end);
    }

    let number_of_spaces = if desired_end > actual_end {
        desired_end - actual_end
    } else {
        0
    };

    num_inserted_characters += number_of_spaces;

    for i in 0..number_of_spaces {
        buf.insert(actual_end + i, b' ');
    }

    let start_buf_pos =
        padding_start_pos.expect("start buf pos should be guaranteed initialized by this point");

    PadBufferForWriteResponse {
        write_idx: desired_start,
        inserted_padding: start_buf_pos..start_buf_pos + num_inserted_characters,
    }
}

fn cursor_to_buf_pos_from_visible_line_ranges(
    cursor_pos: &CursorPos,
    visible_line_ranges: &[Range<usize>],
) -> Option<(usize, Range<usize>)> {
    visible_line_ranges.get(cursor_pos.y).and_then(|range| {
        let candidate_pos = range.start + cursor_pos.x;
        if candidate_pos > range.end {
            None
        } else {
            Some((candidate_pos, range.clone()))
        }
    })
}

fn cursor_to_buf_pos(
    buf: &[u8],
    cursor_pos: &CursorPos,
    width: usize,
    height: usize,
) -> Option<(usize, Range<usize>)> {
    let line_ranges = calc_line_ranges(buf, width);
    let visible_line_ranges = line_ranges_to_visible_line_ranges(&line_ranges, height);

    cursor_to_buf_pos_from_visible_line_ranges(cursor_pos, visible_line_ranges)
}

pub struct TerminalBufferInsertResponse {
    /// Range of written data after insertion of padding
    pub written_range: Range<usize>,
    /// Range of written data that is new. Note this will shift all data after it
    /// Includes padding that was previously not there, e.g. newlines needed to get to the
    /// requested row for writing
    pub insertion_range: Range<usize>,
    pub new_cursor_pos: CursorPos,
}

#[derive(Debug)]
pub struct TerminalBufferInsertLineResponse {
    /// Range of deleted data **before insertion**
    pub deleted_range: Range<usize>,
    /// Range of inserted data
    pub inserted_range: Range<usize>,
}

pub struct TerminalBufferSetWinSizeResponse {
    pub changed: bool,
    pub _insertion_range: Range<usize>,
    pub new_cursor_pos: CursorPos,
}

#[derive(Eq, PartialEq, Debug)]
pub struct TerminalBufferHolder {
    buf: Vec<u8>,
    width: usize,
    height: usize,
}

impl TerminalBufferHolder {
    pub const fn new(width: usize, height: usize) -> Self {
        Self {
            buf: vec![],
            width,
            height,
        }
    }

    pub fn insert_data(
        &mut self,
        cursor_pos: &CursorPos,
        data: &[u8],
    ) -> TerminalBufferInsertResponse {
        info!("Inserting data : {:?}", data);
        let PadBufferForWriteResponse {
            write_idx,
            inserted_padding,
        } = pad_buffer_for_write(
            &mut self.buf,
            self.width,
            self.height,
            cursor_pos,
            data.len(),
        );
        let write_range = write_idx..write_idx + data.len();
        self.buf[write_range.clone()].copy_from_slice(data);
        info!("Buffer after insert: {:?}", self.buf);
        let new_cursor_pos = buf_to_cursor_pos(&self.buf, self.width, self.height, write_range.end)
            .expect("write range should be valid in buf");
        debug!("New cursor pos: {:?}", new_cursor_pos);
        TerminalBufferInsertResponse {
            written_range: write_range,
            insertion_range: inserted_padding,
            new_cursor_pos,
        }
    }

    /// Inserts data, but will not wrap. If line end is hit, data stops
    pub fn insert_spaces(
        &mut self,
        cursor_pos: &CursorPos,
        mut num_spaces: usize,
    ) -> TerminalBufferInsertResponse {
        num_spaces = self.width.min(num_spaces);

        let buf_pos = cursor_to_buf_pos(&self.buf, cursor_pos, self.width, self.height);
        if let Some((buf_pos, line_range)) = buf_pos {
            // Insert spaces until either we hit num_spaces, or the line width is too long
            let line_len = line_range.end - line_range.start;
            let num_inserted = (num_spaces).min(self.width - line_len);

            // Overwrite existing with spaces until we hit num_spaces or we hit the line end
            let num_overwritten = (num_spaces - num_inserted).min(line_range.end - buf_pos);

            // NOTE: We do the overwrite first so we don't have to worry about adjusting
            // indices for the newly inserted data
            self.buf[buf_pos..buf_pos + num_overwritten].fill(b' ');
            self.buf
                .splice(buf_pos..buf_pos, std::iter::repeat(b' ').take(num_inserted));

            let used_spaces = num_inserted + num_overwritten;
            TerminalBufferInsertResponse {
                written_range: buf_pos..buf_pos + used_spaces,
                insertion_range: buf_pos..buf_pos + num_inserted,
                new_cursor_pos: cursor_pos.clone(),
            }
        } else {
            let PadBufferForWriteResponse {
                write_idx,
                inserted_padding,
            } = pad_buffer_for_write(
                &mut self.buf,
                self.width,
                self.height,
                cursor_pos,
                num_spaces,
            );
            TerminalBufferInsertResponse {
                written_range: write_idx..write_idx + num_spaces,
                insertion_range: inserted_padding,
                new_cursor_pos: cursor_pos.clone(),
            }
        }
    }

    pub fn insert_lines(
        &mut self,
        cursor_pos: &CursorPos,
        mut num_lines: usize,
    ) -> TerminalBufferInsertLineResponse {
        let line_ranges = calc_line_ranges(&self.buf, self.width);
        let visible_line_ranges = line_ranges_to_visible_line_ranges(&line_ranges, self.height);

        // NOTE: Cursor x position is not used. If the cursor position was too far to the right,
        // there may be no buffer position associated with it. Use Y only
        let Some(line_range) = visible_line_ranges.get(cursor_pos.y) else {
            return TerminalBufferInsertLineResponse {
                deleted_range: 0..0,
                inserted_range: 0..0,
            };
        };

        let available_space = self.height - visible_line_ranges.len();
        // If height is 10, and y is 5, we can only insert 5 lines. If we inserted more it would
        // adjust the visible line range, and that would be a problem
        num_lines = num_lines.min(self.height - cursor_pos.y);

        let deletion_range = if num_lines > available_space {
            let num_lines_removed = num_lines - available_space;
            let removal_start_idx =
                visible_line_ranges[visible_line_ranges.len() - num_lines_removed].start;
            let deletion_range = removal_start_idx..self.buf.len();
            self.buf.truncate(removal_start_idx);
            deletion_range
        } else {
            0..0
        };

        let insertion_pos = line_range.start;

        // Edge case, if the previous line ended in a line wrap, inserting a new line will not
        // result in an extra line being shown on screen. E.g. with a width of 5, 01234 and 01234\n
        // both look like a line of length 5. In this case we need to add another newline
        if insertion_pos > 0 && self.buf[insertion_pos - 1] != b'\n' {
            num_lines += 1;
        }

        self.buf.splice(
            insertion_pos..insertion_pos,
            std::iter::repeat(b'\n').take(num_lines),
        );

        TerminalBufferInsertLineResponse {
            deleted_range: deletion_range,
            inserted_range: insertion_pos..insertion_pos + num_lines,
        }
    }

    pub fn clear_forwards(&mut self, cursor_pos: &CursorPos) -> Option<usize> {
        let line_ranges = calc_line_ranges(&self.buf, self.width);
        let visible_line_ranges = line_ranges_to_visible_line_ranges(&line_ranges, self.height);

        let (buf_pos, _) =
            cursor_to_buf_pos_from_visible_line_ranges(cursor_pos, visible_line_ranges)?;

        let previous_last_char = self.buf[buf_pos];
        self.buf.truncate(buf_pos);

        // If we truncate at the start of a line, and the previous line did not end with a newline,
        // the first inserted newline will not have an effect on the number of visible lines. This
        // is because we are allowed to have a trailing newline that is longer than the terminal
        // width. To keep the cursor pos the same as it was before, if the truncate position is the
        // start of a line, and the previous character is _not_ a newline, insert an extra newline
        // to compensate
        //
        // If we truncated a newline it's the same situation
        if cursor_pos.x == 0 && buf_pos > 0 && self.buf[buf_pos - 1] != b'\n'
            || previous_last_char == b'\n'
        {
            self.buf.push(b'\n');
        }

        for line in visible_line_ranges {
            if line.end > buf_pos {
                self.buf.push(b'\n');
            }
        }

        let new_cursor_pos =
            buf_to_cursor_pos(&self.buf, self.width, self.height, buf_pos).map(|mut pos| {
                // NOTE: buf to cursor pos may put the cursor one past the end of the line. In this
                // case it's ok because there are two valid cursor positions and we only care about one
                // of them
                if pos.x == self.width {
                    pos.x = 0;
                    pos.y += 1;
                    pos.x_as_characters = 0;
                }
                pos
            });

        assert_eq!(new_cursor_pos, Ok(cursor_pos.clone()));
        Some(buf_pos)
    }

    pub fn clear_line_forwards(&mut self, cursor_pos: &CursorPos) -> Option<Range<usize>> {
        // Can return early if none, we didn't delete anything if there is nothing to delete
        let (buf_pos, line_range) =
            cursor_to_buf_pos(&self.buf, cursor_pos, self.width, self.height)?;

        let del_range = buf_pos..line_range.end;
        self.buf.drain(del_range.clone());
        Some(del_range)
    }

    pub fn clear_all(&mut self) {
        self.buf.clear();
    }

    pub fn delete_forwards(
        &mut self,
        cursor_pos: &CursorPos,
        num_chars: usize,
    ) -> Option<Range<usize>> {
        let (buf_pos, line_range) =
            cursor_to_buf_pos(&self.buf, cursor_pos, self.width, self.height)?;

        let mut delete_range = buf_pos..buf_pos + num_chars;

        if delete_range.end > line_range.end && self.buf.get(line_range.end) != Some(&b'\n') {
            self.buf.insert(line_range.end, b'\n');
        }

        delete_range.end = line_range.end.min(delete_range.end);

        self.buf.drain(delete_range.clone());
        Some(delete_range)
    }

    pub fn data(&self) -> TerminalData<&[u8]> {
        let line_ranges = calc_line_ranges(&self.buf, self.width);
        let visible_line_ranges = line_ranges_to_visible_line_ranges(&line_ranges, self.height);
        if self.buf.is_empty() {
            return TerminalData {
                scrollback: &[],
                visible: &self.buf,
            };
        }
        let start = visible_line_ranges[0].start;

        TerminalData {
            scrollback: &self.buf[0..start],
            visible: &self.buf[start..],
        }
    }

    pub const fn get_win_size(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    pub fn set_win_size(
        &mut self,
        width: usize,
        height: usize,
        cursor_pos: &CursorPos,
    ) -> TerminalBufferSetWinSizeResponse {
        let changed = self.width != width || self.height != height;
        if !changed {
            return TerminalBufferSetWinSizeResponse {
                changed: false,
                _insertion_range: 0..0,
                new_cursor_pos: cursor_pos.clone(),
            };
        }

        // Ensure that the cursor position has a valid buffer position. That way when we resize we
        // can just look up where the cursor is supposed to be and map it back to it's new cursor
        // position
        let pad_response =
            pad_buffer_for_write(&mut self.buf, self.width, self.height, cursor_pos, 0);
        let buf_pos = pad_response.write_idx;
        let inserted_padding = pad_response.inserted_padding;
        let new_cursor_pos = buf_to_cursor_pos(&self.buf, width, height, buf_pos)
            .expect("buf pos should exist in buffer");

        self.width = width;
        self.height = height;

        TerminalBufferSetWinSizeResponse {
            changed,
            _insertion_range: inserted_padding,
            new_cursor_pos,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_larger_buffer() {
        let buffer = [
            32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 46,
            46, 39, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 102, 114, 101, 100, 64, 106, 111, 101,
            45, 115, 45, 83, 50, 51, 45, 85, 108, 116, 114, 97, 10, 32, 32, 32, 32, 32, 32, 32, 32,
            32, 32, 32, 32, 32, 32, 32, 32, 32, 44, 120, 78, 77, 77, 46, 32, 32, 32, 32, 32, 32,
            32, 32, 32, 32, 32, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
            45, 45, 45, 10, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 46, 79, 77,
            77, 77, 77, 111, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 79, 83, 58, 32, 109,
            97, 99, 79, 83, 32, 83, 101, 113, 117, 111, 105, 97, 32, 49, 53, 46, 49, 32, 97, 114,
            109, 54, 52, 10, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 108, 77,
            77, 34, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 72, 111, 115, 116,
            58, 32, 77, 97, 99, 66, 111, 111, 107, 32, 80, 114, 111, 32, 40, 49, 52, 45, 105, 110,
            99, 104, 44, 32, 78, 111, 118, 32, 50, 48, 50, 51, 44, 32, 84, 104, 114, 101, 101, 32,
            84, 104, 117, 110, 100, 101, 114, 98, 111, 108, 116, 32, 52, 32, 112, 111, 114, 116,
            115, 41, 10, 32, 32, 32, 32, 32, 46, 59, 108, 111, 100, 100, 111, 58, 46, 32, 32, 46,
            111, 108, 108, 111, 100, 100, 111, 108, 59, 46, 32, 32, 32, 32, 32, 32, 32, 75, 101,
            114, 110, 101, 108, 58, 32, 68, 97, 114, 119, 105, 110, 32, 50, 52, 46, 49, 46, 48, 10,
            32, 32, 32, 99, 75, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 78, 87, 77, 77, 77, 77, 77,
            77, 77, 77, 77, 77, 48, 58, 32, 32, 32, 32, 32, 85, 112, 116, 105, 109, 101, 58, 32,
            53, 32, 100, 97, 121, 115, 44, 32, 57, 32, 104, 111, 117, 114, 115, 44, 32, 54, 32,
            109, 105, 110, 115, 10, 32, 46, 75, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77,
            77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 87, 100, 46, 32, 32, 32, 32, 32, 66, 97, 116,
            116, 101, 114, 121, 32, 40, 98, 113, 52, 48, 122, 54, 53, 49, 41, 58, 32, 49, 48, 48,
            37, 32, 91, 65, 67, 32, 99, 111, 110, 110, 101, 99, 116, 101, 100, 93, 10, 32, 88, 77,
            77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77,
            88, 46, 10, 59, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77,
            77, 77, 77, 77, 77, 77, 58, 32, 32, 32, 32, 32, 32, 32, 32, 80, 97, 99, 107, 97, 103,
            101, 115, 58, 32, 49, 57, 55, 32, 40, 98, 114, 101, 119, 41, 44, 32, 51, 48, 32, 40,
            98, 114, 101, 119, 45, 99, 97, 115, 107, 41, 10, 58, 77, 77, 77, 77, 77, 77, 77, 77,
            77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 58, 32, 32, 32, 32, 32,
            32, 32, 32, 83, 104, 101, 108, 108, 58, 32, 122, 115, 104, 32, 53, 46, 57, 10, 46, 77,
            77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77,
            77, 88, 46, 32, 32, 32, 32, 32, 32, 32, 68, 105, 115, 112, 108, 97, 121, 32, 40, 67,
            111, 108, 111, 114, 32, 76, 67, 68, 41, 58, 32, 51, 48, 50, 52, 120, 49, 57, 54, 52,
            32, 64, 32, 49, 50, 48, 32, 72, 122, 32, 40, 97, 115, 32, 49, 53, 49, 50, 120, 57, 56,
            50, 41, 32, 105, 110, 32, 49, 52, 226, 128, 179, 32, 91, 66, 117, 105, 108, 116, 45,
            105, 110, 93, 10, 32, 107, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77,
            77, 77, 77, 77, 77, 77, 77, 77, 77, 87, 100, 46, 32, 32, 32, 32, 32, 84, 101, 114, 109,
            105, 110, 97, 108, 58, 32, 102, 114, 101, 109, 105, 110, 97, 108, 10, 32, 39, 88, 77,
            77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77,
            77, 77, 77, 107, 10, 32, 32, 39, 88, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77,
            77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 75, 46, 32, 32, 32, 32, 67, 80, 85, 58,
            32, 65, 112, 112, 108, 101, 32, 77, 51, 32, 77, 97, 120, 32, 40, 49, 52, 41, 32, 64,
            32, 52, 46, 48, 54, 32, 71, 72, 122, 10, 32, 32, 32, 32, 107, 77, 77, 77, 77, 77, 77,
            77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 100, 32, 32, 32, 32,
            32, 32, 71, 80, 85, 58, 32, 65, 112, 112, 108, 101, 32, 77, 51, 32, 77, 97, 120, 32,
            40, 51, 48, 41, 32, 64, 32, 49, 46, 51, 56, 32, 71, 72, 122, 32, 91, 73, 110, 116, 101,
            103, 114, 97, 116, 101, 100, 93, 10, 32, 32, 32, 32, 32, 59, 75, 77, 77, 77, 77, 77,
            77, 77, 87, 88, 88, 87, 77, 77, 77, 77, 77, 77, 77, 107, 46, 32, 32, 32, 32, 32, 32,
            32, 77, 101, 109, 111, 114, 121, 58, 32, 50, 50, 46, 49, 54, 32, 71, 105, 66, 32, 47,
            32, 51, 54, 46, 48, 48, 32, 71, 105, 66, 32, 40, 54, 50, 37, 41, 10, 32, 32, 32, 32,
            32, 32, 32, 34, 99, 111, 111, 99, 42, 34, 32, 32, 32, 32, 34, 42, 99, 111, 111, 39, 34,
            10, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
            32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
            32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 10, 32, 32, 32, 32, 32, 32,
            32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
            32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
            32, 32, 32, 32, 32, 32, 32, 32, 10, 238, 130, 176, 32, 102, 114, 101, 100, 64, 106,
            111, 101, 45, 115, 45, 83, 50, 51, 45, 85, 108, 116, 114, 97, 32, 238, 130, 176, 32,
            238, 172, 134, 32, 238, 130, 177, 32, 239, 132, 149, 32, 238, 130, 177, 32, 102, 114,
            101, 109, 105, 110, 97, 108, 32, 238, 130, 176, 32, 238, 130, 160, 109, 97, 105, 110,
            32, 238, 130, 176, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
            32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
            32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
            32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
            32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
            32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
            32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 10, 32, 32, 32, 32, 32,
            108, 108, 97, 175, 32, 32, 108, 108, 97, 122, 121, 103, 105, 116, 10,
        ] as [u8; 1404];

        // verify the buffer is valid utf8

        let mut i = 1380;

        for &byte in &buffer[1380..=1400] {
            println!("{i} {:?} \"{}\"", byte, String::from_utf8_lossy(&[byte]));
            i += 1;
        }

        println!("{:?}", &buffer[1380..1404]);

        // let mut temp = vec![buffer[1391]];
        // println!("{:?}", std::str::from_utf8(&buffer[..1392]));
        // println!("{:?}", std::str::from_utf8(&temp));
        // println!("a {:?}", temp);
        // temp = vec![buffer[1393]];
        // println!("{:?}", std::str::from_utf8(&temp));
        // println!("{:x?}", temp);
        // println!("{:?}", std::str::from_utf8(&buffer[1394..]));
        assert!(std::str::from_utf8(&buffer[..1393]).is_ok());

        let ranges = calc_line_ranges(&buffer, 213);
        assert_eq!(
            ranges,
            [
                0..54,
                55..109,
                110..172,
                173..271,
                272..327,
                328..393,
                394..467,
                468..495,
                496..566,
                567..615,
                616..723,
                724..776,
                777..807,
                808..875,
                876..956,
                957..1026,
                1027..1052,
                1053..1111,
                1112..1170,
                1171..1383,
                1384..1403
            ]
        );

        // for range in ranges {
        //     // verify each line is less than 213 characters
        //     assert!(range.end - range.start <= 213);

        //     //verify each line represents a valid utf8 string

        //     let line = &buffer[range];
        //     assert!(std::str::from_utf8(line).is_ok());
        // }
    }

    #[test]
    fn test_line_ranges_with_utf8() {
        let ranges = calc_line_ranges("😀😀😀\n😀".as_bytes(), 2);
        assert_eq!(ranges, [0..8, 8..12, 13..17]);
    }

    fn simulate_resize(
        canvas: &mut TerminalBufferHolder,
        width: usize,
        height: usize,
        cursor_pos: &CursorPos,
    ) -> TerminalBufferInsertResponse {
        let mut response = canvas.set_win_size(width, height, cursor_pos);
        response.new_cursor_pos.x = 0;
        let mut response = canvas.insert_data(&response.new_cursor_pos, &vec![b' '; width]);
        response.new_cursor_pos.x = 0;

        canvas.insert_data(&response.new_cursor_pos, b"$ ")
    }

    fn crlf(pos: &mut CursorPos) {
        pos.y += 1;
        pos.x = 0;
    }

    #[test]
    fn test_calc_line_ranges() {
        let line_starts = calc_line_ranges(b"asdf\n0123456789\n012345678901", 10);
        assert_eq!(line_starts, &[0..4, 5..15, 16..26, 26..28]);
    }

    #[test]
    fn test_buffer_padding() {
        let mut buf = b"asdf\n1234\nzxyw".to_vec();

        let cursor_pos = CursorPos {
            x: 8,
            y: 0,
            x_as_characters: 8,
        };
        let response = pad_buffer_for_write(&mut buf, 10, 10, &cursor_pos, 10);
        assert_eq!(buf, b"asdf              \n1234\nzxyw");
        assert_eq!(response.write_idx, 8);
        assert_eq!(response.inserted_padding, 4..18);
    }

    #[test]
    fn test_canvas_clear_forwards() {
        let mut buffer = TerminalBufferHolder::new(5, 5);
        // Push enough data to get some in scrollback
        buffer.insert_data(
            &CursorPos {
                x: 0,
                y: 0,
                x_as_characters: 0,
            },
            b"012343456789\n0123456789\n1234",
        );

        assert_eq!(
            buffer.data().visible,
            b"\
                   34567\
                   89\n\
                   01234\
                   56789\n\
                   1234\n"
        );
        buffer.clear_forwards(&CursorPos {
            x: 1,
            y: 1,
            x_as_characters: 1,
        });
        // Same amount of lines should be present before and after clear
        assert_eq!(
            buffer.data().visible,
            b"\
                   34567\
                   8\n\
                   \n\
                   \n\
                   \n"
        );

        // A few special cases.
        // 1. Truncating on beginning of line and previous char was not a newline
        let mut buffer = TerminalBufferHolder::new(5, 5);
        buffer.insert_data(
            &CursorPos {
                x: 0,
                y: 0,
                x_as_characters: 0,
            },
            b"012340123401234012340123401234",
        );
        buffer.clear_forwards(&CursorPos {
            x: 0,
            y: 1,
            x_as_characters: 0,
        });
        assert_eq!(buffer.data().visible, b"01234\n\n\n\n\n");

        // 2. Truncating on beginning of line and previous char was a newline
        let mut buffer = TerminalBufferHolder::new(5, 5);
        buffer.insert_data(
            &CursorPos {
                x: 0,
                y: 0,
                x_as_characters: 0,
            },
            b"01234\n0123401234012340123401234",
        );
        buffer.clear_forwards(&CursorPos {
            x: 0,
            y: 1,
            x_as_characters: 0,
        });
        assert_eq!(buffer.data().visible, b"01234\n\n\n\n\n");

        // 3. Truncating on a newline
        let mut buffer = TerminalBufferHolder::new(5, 5);
        buffer.insert_data(
            &CursorPos {
                x: 0,
                y: 0,
                x_as_characters: 0,
            },
            b"\n\n\n\n\n\n",
        );
        buffer.clear_forwards(&CursorPos {
            x: 0,
            y: 1,
            x_as_characters: 0,
        });
        assert_eq!(buffer.data().visible, b"\n\n\n\n\n");
    }

    #[test]
    fn test_canvas_clear() {
        let mut buffer = TerminalBufferHolder::new(5, 5);
        buffer.insert_data(
            &CursorPos {
                x: 0,
                y: 0,
                x_as_characters: 0,
            },
            b"0123456789",
        );
        buffer.clear_all();
        assert_eq!(buffer.data().visible, &[]);
    }

    #[test]
    fn test_terminal_buffer_overwrite_early_newline() {
        let mut buffer = TerminalBufferHolder::new(5, 5);
        buffer.insert_data(
            &CursorPos {
                x: 0,
                y: 0,
                x_as_characters: 0,
            },
            b"012\n3456789",
        );
        assert_eq!(buffer.data().visible, b"012\n3456789\n");

        // Cursor pos should be calculated based off wrapping at column 5, but should not result in
        // an extra newline
        buffer.insert_data(
            &CursorPos {
                x: 2,
                y: 1,
                x_as_characters: 2,
            },
            b"test",
        );
        assert_eq!(buffer.data().visible, b"012\n34test9\n");
    }

    #[test]
    fn test_terminal_buffer_overwrite_no_newline() {
        let mut buffer = TerminalBufferHolder::new(5, 5);
        buffer.insert_data(
            &CursorPos {
                x: 0,
                y: 0,
                x_as_characters: 0,
            },
            b"0123456789",
        );
        assert_eq!(buffer.data().visible, b"0123456789\n");

        // Cursor pos should be calculated based off wrapping at column 5, but should not result in
        // an extra newline
        buffer.insert_data(
            &CursorPos {
                x: 2,
                y: 1,
                x_as_characters: 2,
            },
            b"test",
        );
        assert_eq!(buffer.data().visible, b"0123456test\n");
    }

    #[test]
    fn test_terminal_buffer_overwrite_late_newline() {
        // This should behave exactly as test_terminal_buffer_overwrite_no_newline(), except with a
        // neline between lines 1 and 2
        let mut buffer = TerminalBufferHolder::new(5, 5);
        buffer.insert_data(
            &CursorPos {
                x: 0,
                y: 0,
                x_as_characters: 0,
            },
            b"01234\n56789",
        );
        assert_eq!(buffer.data().visible, b"01234\n56789\n");

        buffer.insert_data(
            &CursorPos {
                x: 2,
                y: 1,
                x_as_characters: 2,
            },
            b"test",
        );
        assert_eq!(buffer.data().visible, b"01234\n56test\n");
    }

    #[test]
    fn test_terminal_buffer_insert_unallocated_data() {
        let mut buffer = TerminalBufferHolder::new(10, 10);
        buffer.insert_data(
            &CursorPos {
                x: 4,
                y: 5,
                x_as_characters: 4,
            },
            b"hello world",
        );
        assert_eq!(buffer.data().visible, b"\n\n\n\n\n    hello world\n");

        buffer.insert_data(
            &CursorPos {
                x: 3,
                y: 2,
                x_as_characters: 3,
            },
            b"hello world",
        );
        assert_eq!(
            buffer.data().visible,
            b"\n\n   hello world\n\n\n    hello world\n"
        );
    }

    #[test]
    fn test_canvas_scrolling() {
        let mut canvas = TerminalBufferHolder::new(10, 3);
        let initial_cursor_pos = CursorPos {
            x: 0,
            y: 0,
            x_as_characters: 0,
        };

        // Simulate real terminal usage where newlines are injected with cursor moves
        let mut response = canvas.insert_data(&initial_cursor_pos, b"asdf");
        crlf(&mut response.new_cursor_pos);
        let mut response = canvas.insert_data(&response.new_cursor_pos, b"xyzw");
        crlf(&mut response.new_cursor_pos);
        let mut response = canvas.insert_data(&response.new_cursor_pos, b"1234");
        crlf(&mut response.new_cursor_pos);
        let mut response = canvas.insert_data(&response.new_cursor_pos, b"5678");
        crlf(&mut response.new_cursor_pos);

        assert_eq!(canvas.data().scrollback, b"asdf\n");
        assert_eq!(canvas.data().visible, b"xyzw\n1234\n5678\n");
    }

    #[test]
    fn test_canvas_delete_forwards() {
        let mut canvas = TerminalBufferHolder::new(10, 5);
        canvas.insert_data(
            &CursorPos {
                x: 0,
                y: 0,
                x_as_characters: 0,
            },
            b"asdf\n123456789012345",
        );

        // Test normal deletion
        let deleted_range = canvas.delete_forwards(
            &CursorPos {
                x: 1,
                y: 0,
                x_as_characters: 1,
            },
            1,
        );

        assert_eq!(deleted_range, Some(1..2));
        assert_eq!(canvas.data().visible, b"adf\n123456789012345\n");

        // Test deletion clamped on newline
        let deleted_range = canvas.delete_forwards(
            &CursorPos {
                x: 1,
                y: 0,
                x_as_characters: 1,
            },
            10,
        );
        assert_eq!(deleted_range, Some(1..3));
        assert_eq!(canvas.data().visible, b"a\n123456789012345\n");

        // Test deletion clamped on wrap
        let deleted_range = canvas.delete_forwards(
            &CursorPos {
                x: 7,
                y: 1,
                x_as_characters: 7,
            },
            10,
        );
        assert_eq!(deleted_range, Some(9..12));
        assert_eq!(canvas.data().visible, b"a\n1234567\n12345\n");

        // Test deletion in case where nothing is deleted
        let deleted_range = canvas.delete_forwards(
            &CursorPos {
                x: 5,
                y: 5,
                x_as_characters: 5,
            },
            10,
        );
        assert_eq!(deleted_range, None);
        assert_eq!(canvas.data().visible, b"a\n1234567\n12345\n");
    }

    #[test]
    fn test_canvas_insert_spaces() {
        let mut canvas = TerminalBufferHolder::new(10, 5);
        canvas.insert_data(
            &CursorPos {
                x: 0,
                y: 0,
                x_as_characters: 0,
            },
            b"asdf\n123456789012345",
        );

        // Happy path
        let response = canvas.insert_spaces(
            &CursorPos {
                x: 2,
                y: 0,
                x_as_characters: 2,
            },
            2,
        );
        assert_eq!(response.written_range, 2..4);
        assert_eq!(response.insertion_range, 2..4);
        assert_eq!(
            response.new_cursor_pos,
            CursorPos {
                x: 2,
                y: 0,
                x_as_characters: 2
            }
        );
        assert_eq!(canvas.data().visible, b"as  df\n123456789012345\n");

        // Truncation at newline
        let response = canvas.insert_spaces(
            &CursorPos {
                x: 2,
                y: 0,
                x_as_characters: 2,
            },
            1000,
        );
        assert_eq!(response.written_range, 2..10);
        assert_eq!(response.insertion_range, 2..6);
        assert_eq!(
            response.new_cursor_pos,
            CursorPos {
                x: 2,
                y: 0,
                x_as_characters: 2
            }
        );
        assert_eq!(canvas.data().visible, b"as        \n123456789012345\n");

        // Truncation at line wrap
        let response = canvas.insert_spaces(
            &CursorPos {
                x: 4,
                y: 1,
                x_as_characters: 4,
            },
            1000,
        );
        assert_eq!(response.written_range, 15..21);
        assert_eq!(
            response.insertion_range.start - response.insertion_range.end,
            0
        );
        assert_eq!(
            response.new_cursor_pos,
            CursorPos {
                x: 4,
                y: 1,
                x_as_characters: 4
            }
        );
        assert_eq!(canvas.data().visible, b"as        \n1234      12345\n");

        // Insertion at non-existent buffer pos
        let response = canvas.insert_spaces(
            &CursorPos {
                x: 2,
                y: 4,
                x_as_characters: 2,
            },
            3,
        );
        assert_eq!(response.written_range, 30..33);
        assert_eq!(response.insertion_range, 27..34);
        assert_eq!(
            response.new_cursor_pos,
            CursorPos {
                x: 2,
                y: 4,
                x_as_characters: 2
            }
        );
        assert_eq!(
            canvas.data().visible,
            b"as        \n1234      12345\n\n     \n"
        );
    }

    #[test]
    fn test_clear_line_forwards() {
        let mut canvas = TerminalBufferHolder::new(10, 5);
        canvas.insert_data(
            &CursorPos {
                x: 0,
                y: 0,
                x_as_characters: 0,
            },
            b"asdf\n123456789012345",
        );

        // Nothing do delete
        let response = canvas.clear_line_forwards(&CursorPos {
            x: 5,
            y: 5,
            x_as_characters: 5,
        });
        assert_eq!(response, None);
        assert_eq!(canvas.data().visible, b"asdf\n123456789012345\n");

        // Hit a newline
        let response = canvas.clear_line_forwards(&CursorPos {
            x: 2,
            y: 0,
            x_as_characters: 2,
        });
        assert_eq!(response, Some(2..4));
        assert_eq!(canvas.data().visible, b"as\n123456789012345\n");

        // Hit a wrap
        let response = canvas.clear_line_forwards(&CursorPos {
            x: 2,
            y: 1,
            x_as_characters: 2,
        });
        assert_eq!(response, Some(5..13));
        assert_eq!(canvas.data().visible, b"as\n1212345\n");
    }

    #[test]
    fn test_resize_expand() {
        // Ensure that on window size increase, text stays in same spot relative to cursor position
        // This was problematic with our initial implementation. It's less of a problem after some
        // later improvements, but we can keep the test to make sure it still seems sane
        let mut canvas = TerminalBufferHolder::new(10, 6);

        let cursor_pos = CursorPos {
            x: 0,
            y: 0,
            x_as_characters: 0,
        };
        let response = simulate_resize(&mut canvas, 10, 5, &cursor_pos);
        let response = simulate_resize(&mut canvas, 10, 4, &response.new_cursor_pos);
        let response = simulate_resize(&mut canvas, 10, 3, &response.new_cursor_pos);
        simulate_resize(&mut canvas, 10, 5, &response.new_cursor_pos);
        assert_eq!(canvas.data().visible, b"$         \n");
    }

    #[test]
    fn test_insert_lines() {
        let mut canvas = TerminalBufferHolder::new(5, 5);

        // Test empty canvas
        let response = canvas.insert_lines(
            &CursorPos {
                x: 0,
                y: 0,
                x_as_characters: 0,
            },
            3,
        );
        // Clear doesn't have to do anything as there's nothing in the canvas to push aside
        assert_eq!(response.deleted_range.start - response.deleted_range.end, 0);
        assert_eq!(
            response.inserted_range.start - response.inserted_range.end,
            0
        );
        assert_eq!(canvas.data().visible, b"");

        // Test edge wrapped
        canvas.insert_data(
            &CursorPos {
                x: 0,
                y: 0,
                x_as_characters: 0,
            },
            b"0123456789asdf\nxyzw",
        );
        assert_eq!(canvas.data().visible, b"0123456789asdf\nxyzw\n");
        let response = canvas.insert_lines(
            &CursorPos {
                x: 3,
                y: 2,
                x_as_characters: 3,
            },
            1,
        );
        assert_eq!(canvas.data().visible, b"0123456789\n\nasdf\nxyzw\n");
        assert_eq!(response.deleted_range.start - response.deleted_range.end, 0);
        assert_eq!(response.inserted_range, 10..12);

        // Test newline wrapped + lines pushed off the edge
        let response = canvas.insert_lines(
            &CursorPos {
                x: 3,
                y: 2,
                x_as_characters: 3,
            },
            1,
        );
        assert_eq!(canvas.data().visible, b"0123456789\n\n\nasdf\n");
        assert_eq!(response.deleted_range, 17..22);
        assert_eq!(response.inserted_range, 11..12);
    }
}
