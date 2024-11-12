// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use super::{cursor::CursorPos, data::TerminalSections, term_char::TChar};
use anyhow::Result;
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

/// Calculate the indexes of the start and end of each line in the buffer given an input width.
/// Ranges do not include newlines. If a newline appears past the width, it does not result in an
/// extra line
///
/// Example
/// ```
/// let ranges = calc_line_ranges(b"12\n1234\n12345", 4);
/// assert_eq!(ranges, [0..2, 3..7, 8..11, 12..13]);
/// ```
fn calc_line_ranges(buf: &[TChar], width: usize) -> Vec<Range<usize>> {
    let mut ret = vec![];

    let mut current_start = 0;

    for (i, c) in buf.iter().enumerate() {
        if *c == TChar::NewLine {
            ret.push(current_start..i);
            current_start = i + 1;
            continue;
        }

        let bytes_since_start = i - current_start;
        assert!(bytes_since_start <= width);
        if bytes_since_start == width {
            ret.push(current_start..i);
            current_start = i;
            continue;
        }
    }

    if buf.len() > current_start {
        ret.push(current_start..buf.len());
    }
    ret
}

fn buf_to_cursor_pos(buf: &[TChar], width: usize, height: usize, buf_pos: usize) -> CursorPos {
    let new_line_ranges = calc_line_ranges(buf, width);
    let new_visible_line_ranges = line_ranges_to_visible_line_ranges(&new_line_ranges, height);

    let (new_cursor_y, new_cursor_line) = if let Some((i, r)) = new_visible_line_ranges
        .iter()
        .enumerate()
        .find(|(_i, r)| r.end >= buf_pos)
    {
        (i, r.clone())
    } else {
        info!("Buffer position not on screen");
        return CursorPos::default();
    };

    if buf_pos < new_cursor_line.start {
        info!("Old cursor position no longer on screen");
        return CursorPos::default();
    };

    let new_cursor_x = buf_pos - new_cursor_line.start;

    CursorPos {
        x: new_cursor_x,
        y: new_cursor_y,
    }
}

fn unwrapped_line_end_pos(buf: &[TChar], start_pos: usize) -> usize {
    buf.iter()
        .enumerate()
        .skip(start_pos)
        .find_map(|(i, c)| match *c {
            TChar::NewLine => Some(i),
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
    buf: &mut Vec<TChar>,
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
        buf.push(TChar::NewLine);
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
        buf.insert(actual_end + i, TChar::Space);
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
    buf: &[TChar],
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
    pub left_over: Vec<u8>,
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
    _insertion_range: Range<usize>,
    pub new_cursor_pos: CursorPos,
}

#[derive(Eq, PartialEq, Debug)]
pub struct TerminalBufferHolder {
    buf: Vec<TChar>,
    width: usize,
    height: usize,
}

impl TerminalBufferHolder {
    #[must_use]
    pub const fn new(width: usize, height: usize) -> Self {
        Self {
            buf: vec![],
            width,
            height,
        }
    }

    /// Inserts data into the buffer at the cursor position
    ///
    /// # Errors
    /// Will error if the data is not valid utf8
    pub fn insert_data(
        &mut self,
        cursor_pos: &CursorPos,
        data: &[u8],
    ) -> Result<TerminalBufferInsertResponse> {
        let mut data_to_use = data.to_vec();
        let mut leftover_bytes = vec![];
        while let Err(_e) = String::from_utf8(data_to_use.clone()) {
            let Some(p) = data_to_use.pop() else { break };
            leftover_bytes.insert(0, p);
        }

        let data_converted_to_string = String::from_utf8(data_to_use)?;

        // loop through all of the characters
        // if the character is utf8, then we need all of the bytes to be written

        let graphemes = data_converted_to_string
            .graphemes(true)
            .collect::<Vec<&str>>();

        let converted_buffer = graphemes
            .iter()
            .map(|s| {
                Ok(if s.len() == 1 {
                    TChar::new_from_single_char(s.as_bytes()[0])
                } else {
                    match TChar::new_from_many_chars(s.as_bytes().to_vec()) {
                        Ok(c) => c,
                        Err(e) => {
                            return Err(e);
                        }
                    }
                })
            })
            .collect::<Result<Vec<TChar>>>()?;

        let PadBufferForWriteResponse {
            write_idx,
            inserted_padding,
        } = pad_buffer_for_write(
            &mut self.buf,
            self.width,
            self.height,
            cursor_pos,
            graphemes.len(),
        );
        let write_range = write_idx..write_idx + graphemes.len();

        self.buf
            .splice(write_range.clone(), converted_buffer.iter().cloned());
        let new_cursor_pos = buf_to_cursor_pos(&self.buf, self.width, self.height, write_range.end);
        Ok(TerminalBufferInsertResponse {
            written_range: write_range,
            insertion_range: inserted_padding,
            new_cursor_pos,
            left_over: leftover_bytes,
        })
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
            self.buf[buf_pos..buf_pos + num_overwritten].fill(TChar::Space);
            self.buf.splice(
                buf_pos..buf_pos,
                std::iter::repeat(TChar::Space).take(num_inserted),
            );

            let used_spaces = num_inserted + num_overwritten;
            TerminalBufferInsertResponse {
                written_range: buf_pos..buf_pos + used_spaces,
                insertion_range: buf_pos..buf_pos + num_inserted,
                new_cursor_pos: cursor_pos.clone(),
                left_over: vec![],
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
                left_over: vec![],
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
        if insertion_pos > 0 && self.buf[insertion_pos - 1] != TChar::NewLine {
            num_lines += 1;
        }

        self.buf.splice(
            insertion_pos..insertion_pos,
            std::iter::repeat(TChar::NewLine).take(num_lines),
        );

        TerminalBufferInsertLineResponse {
            deleted_range: deletion_range,
            inserted_range: insertion_pos..insertion_pos + num_lines,
        }
    }

    /// Clear backwards from the cursor position
    ///
    /// Returns the buffer position that was cleared to
    ///
    /// # Errors
    /// Will error if the cursor position changes during the clear
    pub fn clear_backwards(&mut self, cursor_pos: &CursorPos) -> Result<Option<Range<usize>>> {
        let line_ranges = calc_line_ranges(&self.buf, self.width);
        let visible_line_ranges = line_ranges_to_visible_line_ranges(&line_ranges, self.height);

        let Some((buf_pos, _)) =
            cursor_to_buf_pos_from_visible_line_ranges(cursor_pos, visible_line_ranges)
        else {
            return Ok(None);
        };

        // we want to clear from the start of the visible line to the cursor pos

        // clear from the buf pos that is the start of the visible line to the cursor pos

        let previous_last_char = self.buf[buf_pos].clone();

        for line in visible_line_ranges {
            // replace all characters from the start of the visible lines to buf_pos with spaces
            if line.start < buf_pos {
                self.buf[line.start..buf_pos].fill(TChar::Space);
            }

            // if the line is where the cursor is, we want to clear from the start of the line to the cursor pos
            if line.start == buf_pos {
                self.buf[line.start..buf_pos].fill(TChar::Space);
                break;
            }
        }

        let mut pos = buf_to_cursor_pos(&self.buf, self.width, self.height, buf_pos);
        // NOTE: buf to cursor pos may put the cursor one past the end of the line. In this
        // case it's ok because there are two valid cursor positions and we only care about one
        // of them
        if pos.x == self.width {
            pos.x = 0;
            pos.y += 1;
            //pos.x_as_characters = 0;
        }
        let new_cursor_pos = pos;

        // If we truncate at the start of a line, and the previous line did not end with a newline,
        // the first inserted newline will not have an effect on the number of visible lines. This
        // is because we are allowed to have a trailing newline that is longer than the terminal
        // width. To keep the cursor pos the same as it was before, if the truncate position is the
        // start of a line, and the previous character is _not_ a newline, insert an extra newline
        // to compensate
        //
        // If we truncated a newline it's the same situation
        if cursor_pos.x == 0 && buf_pos > 0 && self.buf[buf_pos - 1] != TChar::NewLine
            || previous_last_char == TChar::NewLine
        {
            self.buf.push(TChar::NewLine);
        }

        if new_cursor_pos != cursor_pos.clone() {
            return Err(anyhow::anyhow!(
                "Cursor position changed while clearing backwards"
            ));
        }
        Ok(Some(visible_line_ranges[cursor_pos.y].start..buf_pos))
    }

    /// Clear forwards from the cursor position
    ///
    /// Returns the buffer position that was cleared to
    ///
    /// # Errors
    /// Will error if the cursor position changes during the clear
    pub fn clear_forwards(&mut self, cursor_pos: &CursorPos) -> Result<Option<usize>> {
        let line_ranges = calc_line_ranges(&self.buf, self.width);
        let visible_line_ranges = line_ranges_to_visible_line_ranges(&line_ranges, self.height);

        let Some((buf_pos, _)) =
            cursor_to_buf_pos_from_visible_line_ranges(cursor_pos, visible_line_ranges)
        else {
            return Ok(None);
        };

        let previous_last_char = self.buf[buf_pos].clone();
        self.buf.truncate(buf_pos);

        // If we truncate at the start of a line, and the previous line did not end with a newline,
        // the first inserted newline will not have an effect on the number of visible lines. This
        // is because we are allowed to have a trailing newline that is longer than the terminal
        // width. To keep the cursor pos the same as it was before, if the truncate position is the
        // start of a line, and the previous character is _not_ a newline, insert an extra newline
        // to compensate
        //
        // If we truncated a newline it's the same situation
        if cursor_pos.x == 0 && buf_pos > 0 && self.buf[buf_pos - 1] != TChar::NewLine
            || previous_last_char == TChar::NewLine
        {
            self.buf.push(TChar::NewLine);
        }

        for line in visible_line_ranges {
            if line.end > buf_pos {
                self.buf.push(TChar::NewLine);
            }
        }

        let mut pos = buf_to_cursor_pos(&self.buf, self.width, self.height, buf_pos);
        // NOTE: buf to cursor pos may put the cursor one past the end of the line. In this
        // case it's ok because there are two valid cursor positions and we only care about one
        // of them
        if pos.x == self.width {
            pos.x = 0;
            pos.y += 1;
            //pos.x_as_characters = 0;
        }
        let new_cursor_pos = pos;

        // assert_eq!(new_cursor_pos, cursor_pos.clone());

        if new_cursor_pos != cursor_pos.clone() {
            return Err(anyhow::anyhow!(
                "Cursor position changed while clearing forwards"
            ));
        }
        Ok(Some(buf_pos))
    }

    pub fn clear_line_forwards(&mut self, cursor_pos: &CursorPos) -> Option<Range<usize>> {
        // Can return early if none, we didn't delete anything if there is nothing to delete
        let (buf_pos, line_range) =
            cursor_to_buf_pos(&self.buf, cursor_pos, self.width, self.height)?;

        let del_range = buf_pos..line_range.end;
        self.buf.drain(del_range.clone());
        Some(del_range)
    }

    pub fn clear_line(&mut self, cursor_pos: &CursorPos) -> Option<Range<usize>> {
        let (_buf_pos, line_range) =
            cursor_to_buf_pos(&self.buf, cursor_pos, self.width, self.height)?;

        let del_range = line_range;
        self.buf.drain(del_range.clone());
        Some(del_range)
    }

    pub fn clear_line_backwards(&mut self, cursor_pos: &CursorPos) -> Option<Range<usize>> {
        let (buf_pos, line_range) =
            cursor_to_buf_pos(&self.buf, cursor_pos, self.width, self.height)?;

        let del_range = line_range.start..buf_pos;
        self.buf.drain(del_range.clone());
        Some(del_range)
    }

    pub fn clear_all(&mut self) {
        self.buf.clear();
    }

    pub fn clear_visible(&mut self) -> std::ops::Range<usize> {
        let line_ranges = calc_line_ranges(&self.buf, self.width);
        let visible_line_ranges = line_ranges_to_visible_line_ranges(&line_ranges, self.height);

        // replace all NONE newlines with spaces
        for line in visible_line_ranges {
            self.buf[line.start..line.end].iter_mut().for_each(|c| {
                if *c != TChar::NewLine {
                    *c = TChar::Space;
                }
            });
        }

        visible_line_ranges[0].start..usize::MAX
    }

    pub fn delete_forwards(
        &mut self,
        cursor_pos: &CursorPos,
        num_chars: usize,
    ) -> Option<Range<usize>> {
        let (buf_pos, line_range) =
            cursor_to_buf_pos(&self.buf, cursor_pos, self.width, self.height)?;

        let mut delete_range = buf_pos..buf_pos + num_chars;

        if delete_range.end > line_range.end
            && self.buf.get(line_range.end) != Some(&TChar::NewLine)
        {
            self.buf.insert(line_range.end, TChar::NewLine);
        }

        delete_range.end = line_range.end.min(delete_range.end);

        self.buf.drain(delete_range.clone());
        Some(delete_range)
    }

    #[must_use]
    pub fn data(&self) -> TerminalSections<Vec<TChar>> {
        let line_ranges = calc_line_ranges(&self.buf, self.width);
        let visible_line_ranges = line_ranges_to_visible_line_ranges(&line_ranges, self.height);
        if self.buf.is_empty() {
            return TerminalSections {
                scrollback: vec![],
                visible: self.buf.clone(),
            };
        }

        if visible_line_ranges.is_empty() {
            return TerminalSections {
                scrollback: self.buf.clone(),
                visible: vec![],
            };
        }

        let start = visible_line_ranges[0].start;
        TerminalSections {
            scrollback: self.buf[..start].to_vec(),
            visible: self.buf[start..].to_vec(),
        }
    }

    #[must_use]
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
        let new_cursor_pos = buf_to_cursor_pos(&self.buf, width, height, buf_pos);

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
}
