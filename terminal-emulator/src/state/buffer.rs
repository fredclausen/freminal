// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use super::{cursor::CursorPos, data::TerminalSections, term_char::TChar};
use anyhow::Result;
use std::ops::Range;

/// Calculate the indexes of the start and end of each line in the buffer given an input width.
/// Ranges do not include newlines. If a newline appears past the width, it does not result in an
/// extra line

pub fn calc_line_ranges(buf: &[TChar], width: usize) -> Vec<Range<usize>> {
    let mut ret = vec![];

    let mut current_start = 0;

    for (i, c) in buf.iter().enumerate() {
        if *c == TChar::NewLine {
            ret.push(current_start..i);
            current_start = i + 1;
            continue;
        }

        let bytes_since_start = i - current_start;
        //assert!(bytes_since_start <= width);

        if bytes_since_start > width {
            error!(
                "Buffer line is longer than width. Returning what we have....it's probably wrong"
            );
            return ret;
        }

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

pub struct PadBufferForWriteResponse {
    /// Where to copy data into
    pub write_idx: usize,
    /// Indexes where we added data
    pub inserted_padding: Range<usize>,
}

pub fn pad_buffer_for_write(
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

    let number_of_spaces = desired_end.saturating_sub(actual_end);

    num_inserted_characters += number_of_spaces;

    for i in 0..number_of_spaces {
        buf.insert(actual_end + i, TChar::Space);
    }

    let start_buf_pos = padding_start_pos.map_or_else(
        || {
            // If we did not insert padding, we are at the end of a line
            error!("Padding start position not set and it should have been. This is a bug");
            actual_end
        },
        |p| p,
    );

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

        // loop through all of the characters
        // if the character is utf8, then we need all of the bytes to be written

        let converted_buffer = TChar::from_vec(&data_to_use)?;

        let PadBufferForWriteResponse {
            write_idx,
            inserted_padding,
        } = pad_buffer_for_write(
            &mut self.buf,
            self.width,
            self.height,
            cursor_pos,
            converted_buffer.len(),
        );
        let write_range = write_idx..write_idx + converted_buffer.len();

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

    pub fn clear_visible(&mut self) -> Option<std::ops::Range<usize>> {
        let line_ranges = calc_line_ranges(&self.buf, self.width);
        let visible_line_ranges = line_ranges_to_visible_line_ranges(&line_ranges, self.height);

        if visible_line_ranges.is_empty() {
            return None;
        }

        // replace all NONE newlines with spaces
        for line in visible_line_ranges {
            self.buf[line.start..line.end].iter_mut().for_each(|c| {
                if *c != TChar::NewLine {
                    *c = TChar::Space;
                }
            });
        }

        Some(visible_line_ranges[0].start..usize::MAX)
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
