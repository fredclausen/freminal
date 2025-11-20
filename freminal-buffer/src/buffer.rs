// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use freminal_common::{
    buffer_states::{
        buffer_type::BufferType, cursor::CursorState, format_tag::FormatTag, tchar::TChar,
    },
    config::FontConfig,
};

use crate::{response::InsertResponse, row::Row};

pub struct Buffer {
    /// All rows in this buffer: scrollback + visible region.
    /// In the primary buffer, this grows until `scrollback_limit` is hit.
    /// In the alternate buffer, this always has exactly `height` rows.
    rows: Vec<Row>,

    /// Width and height of the terminal grid.
    width: usize,
    height: usize,

    /// Current cursor position (row, col).
    cursor: CursorState,

    /// How far the user has scrolled back.
    ///
    /// 0 = bottom (normal live terminal mode)
    /// >0 = viewing older content
    scroll_offset: usize,

    /// Maximum number of scrollback lines allowed.
    ///
    /// For example:
    ///  - height = 40
    ///  - `scrollback_limit` = 1000
    ///    Means `rows.len()` will be at most 1040.
    scrollback_limit: usize,

    /// Whether this is the primary or alternate buffer mode.
    ///
    /// Primary:
    ///   - Has scrollback
    ///   - Writing while scrolled back resets `scroll_offset`
    ///
    /// Alternate:
    ///   - No scrollback
    ///   - Switching back restores primary buffer's saved state
    kind: BufferType,

    /// Saved primary buffer content, cursor, `scroll_offset`,
    /// used when switching to and from alternate buffer.
    saved_primary: Option<SavedPrimaryState>,

    /// Current format tag to apply to inserted text.
    current_tag: FormatTag,
}

/// Everything we need to restore when leaving alternate buffer.
#[derive(Debug, Clone)]
pub struct SavedPrimaryState {
    pub rows: Vec<Row>,
    pub cursor: CursorState,
    pub scroll_offset: usize,
}

impl Buffer {
    /// Creates a new Buffer with the specified width and height.
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        let rows = vec![Row::new(width)];

        Self {
            rows,
            width,
            height,
            cursor: CursorState::default(),
            current_tag: FormatTag::default(),
            scroll_offset: 0,
            scrollback_limit: 4000,
            kind: BufferType::Primary,
            saved_primary: None,
        }
    }

    #[must_use]
    pub const fn get_rows(&self) -> &Vec<Row> {
        &self.rows
    }

    #[must_use]
    pub const fn get_cursor(&self) -> &CursorState {
        &self.cursor
    }

    pub fn insert_text(&mut self, text: &[TChar]) {
        let tag = &self.current_tag;

        // If we're in the primary buffer and the user has scrolled back,
        // jump back to the live bottom view when new output arrives.
        if self.kind == BufferType::Primary && self.scroll_offset > 0 {
            self.scroll_offset = 0;
        }

        let mut remaining = text.to_vec();
        let mut row_idx = self.cursor.pos.y;
        let mut col = self.cursor.pos.x;

        loop {
            // Step 1 — Wrap if needed
            if col >= self.width {
                row_idx += 1;
                col = 0;
            }

            // Step 2 — Ensure row exists *after wrap*
            if row_idx >= self.rows.len() {
                self.rows.push(Row::new(self.width));
            }

            match self.rows[row_idx].insert_text(col, &remaining, tag) {
                InsertResponse::Consumed(final_col) => {
                    self.cursor.pos.x = final_col;
                    self.cursor.pos.y = row_idx;

                    // NEW: enforce scrollback limit after we’re done writing
                    self.enforce_scrollback_limit();
                    return;
                }

                InsertResponse::Leftover { data, final_col } => {
                    // cursor stops at end of this row
                    self.cursor.pos.x = final_col;
                    self.cursor.pos.y = row_idx;

                    // data that didn't fit
                    remaining = data;

                    // move to next row, at col 0
                    row_idx += 1;
                    col = 0;
                }
            }
        }
    }

    fn enforce_scrollback_limit(&mut self) {
        // Only primary buffer keeps scrollback.
        if self.kind == BufferType::Alternate {
            return;
        }

        let max_rows = self.height + self.scrollback_limit;

        if self.rows.len() > max_rows {
            let overflow = self.rows.len() - max_rows;

            // Drop the oldest `overflow` rows.
            self.rows.drain(0..overflow);

            // Cursor row is an index into `rows`, so adjust it.
            if self.cursor.pos.y >= overflow {
                self.cursor.pos.y -= overflow;
            } else {
                self.cursor.pos.y = 0;
            }
        }
    }

    pub fn handle_lf(&mut self) {
        match self.kind {
            BufferType::Primary => {
                // Writing while scrolled back jumps to live bottom.
                if self.scroll_offset > 0 {
                    self.scroll_offset = 0;
                }

                self.cursor.pos.y += 1;

                // Grow rows if needed.
                if self.cursor.pos.y >= self.rows.len() {
                    self.rows.push(Row::new(self.width));
                }

                // Enforce scrollback cap.
                self.enforce_scrollback_limit();
            }

            BufferType::Alternate => {
                // No scrollback; behave like a fixed-height screen.
                if self.cursor.pos.y + 1 < self.height {
                    // Just move down a row if we’re not at the bottom yet.
                    self.cursor.pos.y += 1;
                } else {
                    // At bottom: scroll the screen up by one line.
                    self.scroll_up();
                }
            }
        }
    }

    pub const fn handle_cr(&mut self) {
        self.cursor.pos.x = 0;
    }

    // ----------------------------------------------------------
    // Scrollback: only valid in the PRIMARY buffer
    // ----------------------------------------------------------

    /// How many lines above the live bottom we can scroll.
    const fn max_scroll_offset(&self) -> usize {
        if self.rows.len() <= self.height {
            0
        } else {
            self.rows.len() - self.height
        }
    }

    /// Scroll upward (`lines > 0`) in the primary buffer.
    pub fn scroll_back(&mut self, lines: usize) {
        if self.kind != BufferType::Primary {
            return; // Alternate buffer: no scrollback
        }

        let max = self.max_scroll_offset();
        if max == 0 {
            return;
        }

        self.scroll_offset = (self.scroll_offset + lines).min(max);
    }

    /// Scroll downward (`lines > 0`) toward the live bottom.
    pub fn scroll_forward(&mut self, lines: usize) {
        if self.kind != BufferType::Primary {
            return;
        }

        if self.scroll_offset == 0 {
            return;
        }

        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    /// Jump back to the live view (row = last row).
    pub const fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn scroll_up(&mut self) {
        // remove topmost row
        self.rows.remove(0);

        // add a new empty row at the bottom
        self.rows.push(Row::new(self.width));

        // DO NOT move the cursor in alternate buffer
        if self.kind == BufferType::Primary {
            // primary buffer uses scrollback: move cursor with the visible window
            if self.cursor.pos.y > 0 {
                self.cursor.pos.y -= 1;
            }
        }
    }

    /// Switch from the primary buffer to the alternate screen.
    ///
    /// - Saves current rows, cursor, and `scroll_offset`.
    /// - Replaces contents with a fresh empty screen (height rows).
    /// - Disables scrollback semantics for the alternate screen.
    pub fn enter_alternate(&mut self) {
        // If we're already in the alternate buffer, do nothing.
        if self.kind == BufferType::Alternate {
            return;
        }

        // Save primary state (rows + cursor + scroll_offset).
        let saved = SavedPrimaryState {
            rows: self.rows.clone(),
            cursor: self.cursor.clone(),
            scroll_offset: self.scroll_offset,
        };
        self.saved_primary = Some(saved);

        // Switch to alternate buffer.
        self.kind = BufferType::Alternate;

        // Fresh screen: exactly `height` empty rows.
        self.rows = vec![Row::new(self.width); self.height];

        // Reset cursor and scroll offset for the alternate screen.
        self.cursor = CursorState::default();
        self.scroll_offset = 0;
    }

    /// Leave the alternate screen and restore the primary buffer, if any was saved.
    pub fn leave_alternate(&mut self) {
        // If we're not in alternate, nothing to do.
        if self.kind != BufferType::Alternate {
            return;
        }

        if let Some(saved) = self.saved_primary.take() {
            // Restore saved primary state.
            self.rows = saved.rows;
            self.cursor = saved.cursor;
            self.scroll_offset = saved.scroll_offset;
        }

        self.kind = BufferType::Primary;
    }
}

// tests

// ============================================================================
// Unit Tests for Buffer
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::row::Row;
    use freminal_common::buffer_states::buffer_type::BufferType;
    use freminal_common::buffer_states::tchar::TChar;

    fn ascii(c: char) -> TChar {
        TChar::Ascii(c as u8)
    }

    // ────────────────────────────────────────────────────────────────
    // PRIMARY BUFFER TESTS
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn primary_lf_adds_new_row_no_scroll_yet() {
        let mut buf = Buffer::new(5, 3);

        buf.handle_lf();
        assert_eq!(buf.cursor.pos.y, 1);
        assert_eq!(buf.rows.len(), 2);
    }

    #[test]
    fn primary_lf_accumulates_scrollback() {
        let mut buf = Buffer::new(5, 3);

        for _ in 0..6 {
            buf.handle_lf();
        }

        // initial row + 6 new rows = 7
        assert_eq!(buf.rows.len(), 7);
        assert_eq!(buf.cursor.pos.y, 6);
    }

    #[test]
    fn primary_lf_respects_scrollback_limit() {
        let mut buf = Buffer::new(5, 3);
        buf.scrollback_limit = 2; // very small

        for _ in 0..10 {
            buf.handle_lf();
        }

        // should now be height (3) + limit (2) = 5 rows
        assert_eq!(buf.rows.len(), 5);
        assert_eq!(buf.cursor.pos.y, buf.rows.len() - 1);
    }

    #[test]
    fn primary_insert_text_resets_scroll_offset() {
        let mut buf = Buffer::new(10, 5);
        buf.scroll_offset = 3; // simulate user scrollback

        buf.insert_text(&[ascii('A')]);

        assert_eq!(buf.scroll_offset, 0);
    }

    // ────────────────────────────────────────────────────────────────
    // ALTERNATE BUFFER TESTS
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn alt_buffer_has_no_scrollback() {
        let mut buf = Buffer::new(5, 3);
        buf.enter_alternate();

        assert_eq!(buf.rows.len(), 3);
        assert_eq!(buf.kind, BufferType::Alternate);
    }

    #[test]
    fn alt_buffer_lf_scrolls_screen() {
        let mut buf = Buffer::new(5, 3);
        buf.enter_alternate();

        buf.handle_lf();
        buf.handle_lf();
        assert_eq!(buf.cursor.pos.y, 2);

        // now at bottom → should scroll
        buf.handle_lf();
        assert_eq!(buf.cursor.pos.y, 2);
        assert_eq!(buf.rows.len(), 3);
    }

    #[test]
    fn leaving_alt_restores_primary() {
        let mut buf = Buffer::new(6, 4);

        // create scrollback + move cursor
        buf.handle_lf();
        buf.handle_lf();
        let saved_y = buf.cursor.pos.y;
        let saved_rows = buf.rows.len();

        // Enter alternate buffer via API
        buf.enter_alternate();

        // Do some things in alternate screen (optional)
        buf.handle_lf();

        // Leave alternate, restoring primary
        buf.leave_alternate();

        assert_eq!(buf.kind, BufferType::Primary);
        assert_eq!(buf.rows.len(), saved_rows);
        assert_eq!(buf.cursor.pos.y, saved_y);
    }

    #[test]
    fn scrollback_no_effect_when_no_history() {
        let mut buf = Buffer::new(5, 3);

        buf.scroll_back(10);
        assert_eq!(buf.scroll_offset, 0);
    }

    #[test]
    fn scrollback_clamps_to_max_offset() {
        let mut buf = Buffer::new(5, 3);

        // Add many lines
        for _ in 0..10 {
            buf.handle_lf();
        }

        let max = buf.rows.len() - buf.height;
        buf.scroll_back(999);

        assert_eq!(buf.scroll_offset, max);
    }

    #[test]
    fn scroll_forward_clamps_to_zero() {
        let mut buf = Buffer::new(5, 3);

        for _ in 0..10 {
            buf.handle_lf();
        }

        buf.scroll_back(5); // scroll up some amount
        buf.scroll_forward(999); // scroll down more than enough

        assert_eq!(buf.scroll_offset, 0);
    }

    #[test]
    fn scroll_to_bottom_resets_offset() {
        let mut buf = Buffer::new(5, 3);

        for _ in 0..10 {
            buf.handle_lf();
        }

        buf.scroll_back(5);
        assert!(buf.scroll_offset > 0);

        buf.scroll_to_bottom();

        assert_eq!(buf.scroll_offset, 0);
    }

    #[test]
    fn no_scrollback_in_alternate_buffer() {
        let mut buf = Buffer::new(5, 3);
        buf.enter_alternate();

        for _ in 0..10 {
            buf.handle_lf(); // scrolls but no scrollback
        }

        buf.scroll_back(10);
        assert_eq!(buf.scroll_offset, 0);

        buf.scroll_forward(10);
        assert_eq!(buf.scroll_offset, 0);
    }

    #[test]
    fn insert_text_resets_scrollback() {
        let mut buf = Buffer::new(10, 5);

        for _ in 0..20 {
            buf.handle_lf();
        }

        buf.scroll_back(5);
        assert!(buf.scroll_offset > 0);

        buf.insert_text(&[TChar::Ascii(b'A')]);

        assert_eq!(buf.scroll_offset, 0);
    }
}
