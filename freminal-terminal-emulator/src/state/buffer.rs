// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use super::{cursor::CursorPos, data::TerminalSections, term_char::TChar};
use anyhow::Result;
use freminal_common::scroll::ScrollDirection;
use std::ops::Range;

pub struct PadBufferForWriteResponse {
    /// Where to copy data into
    pub write_idx: usize,
    /// Indexes where we added data
    pub inserted_padding: Range<usize>,
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
    _insertion_range: Range<usize>,
    pub new_cursor_pos: CursorPos,
}

#[derive(Eq, PartialEq, Debug)]
pub struct TerminalBufferHolder {
    pub buf: Vec<TChar>,
    width: usize,
    height: usize,
    visible_line_ranges: Vec<Range<usize>>,
    buffer_line_ranges: Vec<Range<usize>>,
    viewable_index_bottom: usize, // usize::MAX represents the bottom of the buffer
}

impl Default for TerminalBufferHolder {
    fn default() -> Self {
        Self {
            buf: Vec::with_capacity(500_000),
            width: 80,
            height: 24,
            visible_line_ranges: Vec::with_capacity(24),
            buffer_line_ranges: Vec::with_capacity(5000),
            viewable_index_bottom: usize::MAX,
        }
    }
}

impl TerminalBufferHolder {
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            buf: Vec::with_capacity(500_000),
            width,
            height,
            visible_line_ranges: Vec::with_capacity(height),
            buffer_line_ranges: Vec::with_capacity(5000),
            viewable_index_bottom: usize::MAX,
        }
    }

    pub fn scroll_down(&mut self, num_lines: &usize) {
        if self.viewable_index_bottom == usize::MAX {
            return;
        }

        if self.viewable_index_bottom + num_lines == self.buffer_line_ranges.len() {
            self.viewable_index_bottom = usize::MAX;
        }

        self.viewable_index_bottom += num_lines;
    }

    pub fn scroll_up(&mut self, num_lines: &usize) {
        if self.viewable_index_bottom == 0 {
            return;
        }

        self.viewable_index_bottom = self.viewable_index_bottom.saturating_sub(*num_lines);
    }

    pub fn scroll(&mut self, direction: &ScrollDirection) {
        match direction {
            ScrollDirection::Up(n) => self.scroll_up(n),
            ScrollDirection::Down(n) => self.scroll_down(n),
        }
    }

    #[must_use]
    pub fn get_visible_line_ranges(&self) -> &[Range<usize>] {
        &self.visible_line_ranges
    }

    pub fn set_visible_line_ranges(&mut self, visible_line_ranges: Vec<Range<usize>>) {
        self.visible_line_ranges = visible_line_ranges;
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
        // loop through all of the characters
        // if the character is utf8, then we need all of the bytes to be written

        let converted_buffer = TChar::from_vec(data)?;

        let PadBufferForWriteResponse {
            write_idx,
            inserted_padding,
        } = self.pad_buffer_for_write(cursor_pos, converted_buffer.len());
        let write_range = write_idx..write_idx + converted_buffer.len();

        self.buf
            .splice(write_range.clone(), converted_buffer.iter().cloned());

        self.line_ranges_to_visible_line_ranges();

        let new_cursor_pos = self.buf_to_cursor_pos(write_range.end);
        Ok(TerminalBufferInsertResponse {
            written_range: write_range,
            insertion_range: inserted_padding,
            new_cursor_pos,
        })
    }

    /// Inserts data, but will not wrap. If line end is hit, data stops
    pub fn insert_spaces(
        &mut self,
        cursor_pos: &CursorPos,
        mut num_spaces: usize,
    ) -> TerminalBufferInsertResponse {
        num_spaces = self.width.min(num_spaces);

        let buf_pos = self.cursor_to_buf_pos(cursor_pos);
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
            }
        } else {
            let PadBufferForWriteResponse {
                write_idx,
                inserted_padding,
            } = self.pad_buffer_for_write(cursor_pos, num_spaces);
            self.line_ranges_to_visible_line_ranges();
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
        let visible_line_ranges = &self.visible_line_ranges;

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
        let visible_line_ranges = self.visible_line_ranges.clone();

        let Some((buf_pos, _)) = self.cursor_to_buf_pos(cursor_pos) else {
            return Ok(None);
        };

        // we want to clear from the start of the visible line to the cursor pos

        // clear from the buf pos that is the start of the visible line to the cursor pos

        let previous_last_char = self.buf[buf_pos].clone();

        for line in &visible_line_ranges {
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

        let mut pos = self.buf_to_cursor_pos(buf_pos);
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

        self.line_ranges_to_visible_line_ranges();

        Ok(Some(visible_line_ranges[cursor_pos.y].start..buf_pos))
    }

    /// Clear forwards from the cursor position
    ///
    /// Returns the buffer position that was cleared to
    ///
    /// # Errors
    /// Will error if the cursor position changes during the clear
    pub fn clear_forwards(&mut self, cursor_pos: &CursorPos) -> Result<Option<usize>> {
        let visible_line_ranges = &self.visible_line_ranges;

        let Some((buf_pos, _)) = self.cursor_to_buf_pos(cursor_pos) else {
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

        let mut pos = self.buf_to_cursor_pos(buf_pos);

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

        if new_cursor_pos != *cursor_pos {
            return Err(anyhow::anyhow!(
                "Cursor position changed while clearing forwards"
            ));
        }

        self.line_ranges_to_visible_line_ranges();

        Ok(Some(buf_pos))
    }

    pub fn clear_line_forwards(&mut self, cursor_pos: &CursorPos) -> Option<Range<usize>> {
        // Can return early if none, we didn't delete anything if there is nothing to delete
        let (buf_pos, line_range) = self.cursor_to_buf_pos(cursor_pos)?;

        let del_range = buf_pos..line_range.end;
        self.buf.drain(del_range.clone());

        self.line_ranges_to_visible_line_ranges();

        Some(del_range)
    }

    pub fn clear_line(&mut self, cursor_pos: &CursorPos) -> Option<Range<usize>> {
        let (_buf_pos, line_range) = self.cursor_to_buf_pos(cursor_pos)?;

        let del_range = line_range;
        self.buf.drain(del_range.clone());
        self.line_ranges_to_visible_line_ranges();
        Some(del_range)
    }

    pub fn clear_line_backwards(&mut self, cursor_pos: &CursorPos) -> Option<Range<usize>> {
        let (buf_pos, line_range) = self.cursor_to_buf_pos(cursor_pos)?;

        let del_range = line_range.start..buf_pos;
        self.buf.drain(del_range.clone());
        self.line_ranges_to_visible_line_ranges();
        Some(del_range)
    }

    pub fn clear_all(&mut self) {
        self.buf.clear();
        self.visible_line_ranges.clear();
    }

    pub fn clear_visible(&mut self) -> Option<std::ops::Range<usize>> {
        let visible_line_ranges = self.visible_line_ranges.clone();

        if visible_line_ranges.is_empty() {
            return None;
        }

        // replace all NONE newlines with spaces
        for line in &visible_line_ranges {
            self.buf[line.start..line.end].iter_mut().for_each(|c| {
                if *c != TChar::NewLine {
                    *c = TChar::Space;
                }
            });
        }

        self.line_ranges_to_visible_line_ranges();

        Some(visible_line_ranges[0].start..usize::MAX)
    }

    pub fn delete_forwards(
        &mut self,
        cursor_pos: &CursorPos,
        num_chars: usize,
    ) -> Option<Range<usize>> {
        let (buf_pos, line_range) = self.cursor_to_buf_pos(cursor_pos)?;

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

    pub fn erase_forwards(
        &mut self,
        cursor_pos: &CursorPos,
        num_chars: usize,
    ) -> Option<Range<usize>> {
        let (buf_pos, line_range) = self.cursor_to_buf_pos(cursor_pos)?;

        let mut erase_range = buf_pos..buf_pos + num_chars;

        if erase_range.end > line_range.end {
            erase_range.end = line_range.end;
        }

        // remove the range from the buffer
        self.buf.drain(erase_range.clone());
        Some(erase_range)
    }

    #[must_use]
    pub fn data(&self) -> TerminalSections<Vec<TChar>> {
        let visible_line_ranges = &self.visible_line_ranges;
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
    pub fn clip_lines(&mut self, keep_buf_pos: usize) -> Option<Range<usize>> {
        // FIXME: This arbitrary clipping without context of the line start for where we're clipping back
        //        is not ideal. We should be able to clip back to the start of the line. We'll end up showing
        //        the clip position as the start of a new line
        //        I don't want to calculate line positions for the entire buffer here because it's expensive
        //        we can probably get clever and walk from the position we're clipping back to the start of the line
        //        This is a temporary fix to prevent the buffer from growing indefinitely

        if keep_buf_pos.saturating_sub(50_000) == 0 {
            return None;
        }

        self.buf.drain(0..keep_buf_pos.saturating_sub(50_000));
        self.line_ranges_to_visible_line_ranges();
        Some(0..keep_buf_pos.saturating_sub(50_000))
    }

    #[must_use]
    pub fn get_raw_buffer(&self) -> &[TChar] {
        &self.buf
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
        let pad_response = self.pad_buffer_for_write(cursor_pos, 0);
        self.line_ranges_to_visible_line_ranges();
        let buf_pos = pad_response.write_idx;
        let inserted_padding = pad_response.inserted_padding;
        let new_cursor_pos = self.buf_to_cursor_pos(buf_pos);

        self.width = width;
        self.height = height;

        TerminalBufferSetWinSizeResponse {
            changed,
            _insertion_range: inserted_padding,
            new_cursor_pos,
        }
    }

    /// Given terminal height `height`, extract the visible line ranges from all line ranges (which
    /// include scrollback) assuming "visible" is the bottom N lines
    pub fn line_ranges_to_visible_line_ranges(&mut self) {
        let buf = &self.buf;
        let height = self.height;
        let width = self.width;
        if buf.is_empty() {
            self.visible_line_ranges = vec![];
            return;
        }

        // FIXME: This entire thing is janky af. It probably needs a rewrite

        // The goal here is to get the visible line ranges from the buffer. This is easy if we walk the buffer from the start, because we can track where lines start and end with ease
        // However, for efficiency reasons we need to walk the buffer *from the back* because there is no sense in going through 500,000 characters, representing 10000+ lines, if we only care about the last x lines that represent the visible screen
        // The problem becomes tricky with line wrapping in this case. If a consecutive sequence of non-newline characters is longer than the width of the terminal, we need to split it into multiple lines, but if the line is not % 0 of the width, then starting at the back and walking forward we will end up with different break points than if we started at the front and walked back.

        let mut current_start = buf.len() - 1; // start of the current line
        let mut ret: Vec<Range<usize>> = Vec::with_capacity(height); // the ranges of the visible lines
        let mut wrapping = false; // flag to indicate if we are wrapping
        let mut previous_char_was_newline = false; // This flag is used to determine some special cases when we are wrapping
        let mut consecutive_newlines = false; // This is used to track the number of consecutive newlines we have encountered. If we have more than the height of the terminal, we need to stop

        // iterate over the buffer in reverse order
        for (position, character) in buf.iter().enumerate().rev() {
            // special case for the last character in the buffer. If the character is a new line, we DO NOT want to include it in the output. Why, not entirely sure. But it's what the original code did
            // Otherwise, we want the line range to capture the character so we set the current start to be inclusive of the character
            if buf.len() - 1 == position {
                if *character == TChar::NewLine {
                    current_start = position;
                } else {
                    current_start = position + 1;
                }
                continue;
            }

            // if we have enough lines, we can break out of the loop
            if ret.len() == height {
                current_start = position;
                break;
            }

            // We've encountered a newline character. This means we need to add a new line to the output
            if character == &TChar::NewLine {
                // If we are wrapping, we need to take the position to the current start, splitting the ranges on width
                if wrapping {
                    // take the position to current start, splitting the ranges on width

                    // The total characters in the line is the current start minus the position because we are already including the start character in the range
                    let mut current_length = current_start.saturating_sub(position);

                    // If the previous character was a newline, we need to subtract one from the length because the newline is implied
                    if previous_char_was_newline {
                        current_length = current_length.saturating_sub(1);
                    }

                    let new_position = position + 1;
                    let to_add = ranges_from_start_and_end(current_length, new_position, width, 0);
                    ret.extend_from_slice(&to_add);

                    wrapping = false;
                } else if previous_char_was_newline {
                    // If the previous character was a newline, we need to add an empty line but the range is just going to include the newline character
                    ret.push(position + 1..position + 1);
                } else {
                    // If we are not wrapping, we can just add the line as is
                    ret.push(position + 1..current_start);
                }

                current_start = position;
                consecutive_newlines = previous_char_was_newline;
                previous_char_was_newline = true;

                continue;
            }

            if !wrapping && current_start.saturating_sub(position) == width {
                // if we have not hit the max length already, AND the current line is the width of the terminal, we need to set the wrapping flag. We also set the newline flag in case the very next character is a newline
                // current_start = position;
                previous_char_was_newline = true;
                wrapping = true;
            } else if !wrapping {
                // if we are not wrapping, we need to set the newline flag to false
                previous_char_was_newline = false;
            }
        }

        // Done looping. If we have not hit the max length, we need to add the last line to the output
        if ret.len() < height {
            // If we are wrapping, we need to take the position to the current start, splitting the ranges on width using the same logic as above for wrapping
            if wrapping && current_start > width {
                let mut current_length = current_start;
                let mut offset_end = 1;
                if previous_char_was_newline && !consecutive_newlines {
                    current_length = current_length.saturating_sub(1);
                } else if consecutive_newlines {
                    offset_end = 0;
                }
                let new_position = 0;
                let to_add =
                    ranges_from_start_and_end(current_length, new_position, width, offset_end);
                ret.extend_from_slice(&to_add);
            } else {
                // otherwise, just add the line
                ret.push(0..current_start);
            }
        }

        // sort the ranges by start position
        ret.sort_by(|a, b| a.start.cmp(&b.start));

        // if we have more lines than the height, we need to remove the extra lines
        if ret.len() > height {
            // remove extra lines from the front of the buffer
            let to_remove = ret.len() - height;
            ret.drain(0..to_remove);
        }

        self.visible_line_ranges = ret;
    }

    fn buf_to_cursor_pos(&self, buf_pos: usize) -> CursorPos {
        let visible_line_ranges = &self.visible_line_ranges;
        let (new_cursor_y, new_cursor_line) = if let Some((i, r)) = visible_line_ranges
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

    #[must_use]
    pub fn cursor_pos_to_buf_pos(&self, cursor_pos: &CursorPos) -> Option<usize> {
        let visible_line_ranges = &self.visible_line_ranges;
        let line_range = visible_line_ranges.get(cursor_pos.y)?;

        let buf_pos = line_range.start + cursor_pos.x;
        if buf_pos >= line_range.end {
            None
        } else {
            Some(buf_pos)
        }
    }

    pub fn pad_buffer_for_write(
        &mut self,
        cursor_pos: &CursorPos,
        write_len: usize,
    ) -> PadBufferForWriteResponse {
        let visible_line_ranges = &mut self.visible_line_ranges;
        let buf = &mut self.buf;

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
        let actual_end = buf
            .iter()
            .enumerate()
            .skip(line_range.start)
            .find_map(|(i, c)| match *c {
                TChar::NewLine => Some(i),
                _ => None,
            })
            .unwrap_or(buf.len());

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

    fn cursor_to_buf_pos(&self, cursor_pos: &CursorPos) -> Option<(usize, Range<usize>)> {
        let visible_line_ranges = &self.visible_line_ranges;
        visible_line_ranges.get(cursor_pos.y).and_then(|range| {
            let candidate_pos = range.start + cursor_pos.x;
            if candidate_pos > range.end {
                None
            } else {
                Some((candidate_pos, range.clone()))
            }
        })
    }
}

fn ranges_from_start_and_end(
    current_length: usize,
    position: usize,
    width: usize,
    offset_end: usize,
) -> Vec<Range<usize>> {
    let mut to_add = vec![];
    let mut current_length = current_length;
    let mut current_range = position..position;

    if current_length <= width {
        to_add.push(position..position + current_length);

        return to_add;
    }

    let mut did_just_add: bool;
    loop {
        did_just_add = false;
        current_range.end += 1;

        if current_range.end - current_range.start == width {
            to_add.push(current_range.clone());
            current_range.start = current_range.end;
            did_just_add = true;
        }

        if current_length.saturating_sub(1) == 0 {
            break;
        }

        current_length -= 1;
    }

    if !did_just_add {
        current_range.end += offset_end;
        to_add.push(current_range);
    }

    to_add
}
