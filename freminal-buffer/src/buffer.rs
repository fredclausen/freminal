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

use crate::{
    response::InsertResponse,
    row::{Row, RowJoin, RowOrigin},
};

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

    /// LMN mode
    lnm_enabled: bool,
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
            lnm_enabled: false,
        }
    }

    fn push_row(&mut self, origin: RowOrigin, join: RowJoin) {
        let row = Row::new_with_origin(self.width, origin, join);
        self.rows.push(row);
        self.enforce_scrollback_limit();
    }

    fn push_row_with_kind(&mut self, origin: RowOrigin, join: RowJoin) {
        self.rows
            .push(Row::new_with_origin(self.width, origin, join));
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
        // If we're in the primary buffer and the user has scrolled back,
        // jump back to the live bottom view when new output arrives.
        if self.kind == BufferType::Primary && self.scroll_offset > 0 {
            self.scroll_offset = 0;
        }

        let mut remaining = text.to_vec();
        let mut row_idx = self.cursor.pos.y;
        let mut col = self.cursor.pos.x;

        // FIX #3: first write into row 0 turns it into a real logical line
        if row_idx == 0 && self.rows[0].origin == RowOrigin::ScrollFill {
            let row = &mut self.rows[0];
            row.origin = RowOrigin::HardBreak;
            row.join = RowJoin::NewLogicalLine;
        }

        loop {
            // ┌─────────────────────────────────────────────┐
            // │ PRE-WRAP: if we're already at/past width,   │
            // │ move to the next row as a soft-wrap row.    │
            // └─────────────────────────────────────────────┘
            if col >= self.width {
                row_idx += 1;
                col = 0;

                if row_idx >= self.rows.len() {
                    // brand new soft-wrap continuation row
                    self.push_row(RowOrigin::SoftWrap, RowJoin::ContinueLogicalLine);
                } else {
                    // reuse existing row as a soft-wrap continuation
                    let row = &mut self.rows[row_idx];
                    row.origin = RowOrigin::SoftWrap;
                    row.join = RowJoin::ContinueLogicalLine;
                    row.clear();
                }

                self.cursor.pos.y = row_idx;
            }

            // ┌─────────────────────────────────────────────┐
            // │ Ensure the target row exists. If we get     │
            // │ here without wrapping, this is just a       │
            // │ normal blank row (no SoftWrap metadata).    │
            // └─────────────────────────────────────────────┘
            if row_idx >= self.rows.len() {
                self.rows.push(Row::new(self.width));
            }

            // clone tag here to avoid long-lived borrows of &self
            let tag = self.current_tag.clone();

            // ┌─────────────────────────────────────────────┐
            // │ Try to insert into this row.                │
            // └─────────────────────────────────────────────┘
            match self.rows[row_idx].insert_text(col, &remaining, &tag) {
                InsertResponse::Consumed(final_col) => {
                    // All text fit on this row.
                    self.cursor.pos.x = final_col;
                    self.cursor.pos.y = row_idx;

                    self.enforce_scrollback_limit();
                    return;
                }

                InsertResponse::Leftover { data, final_col } => {
                    // This row filled; some data remains.
                    self.cursor.pos.x = final_col;
                    self.cursor.pos.y = row_idx;

                    remaining = data;

                    // Move to next row for continuation.
                    row_idx += 1;
                    col = 0;

                    // POST-WRAP: we now know a wrap actually occurred.
                    if row_idx >= self.rows.len() {
                        // brand new continuation
                        self.push_row(RowOrigin::SoftWrap, RowJoin::ContinueLogicalLine);
                    } else {
                        // reuse existing row as continuation
                        let row = &mut self.rows[row_idx];
                        row.origin = RowOrigin::SoftWrap;
                        row.join = RowJoin::ContinueLogicalLine;
                        row.clear();
                    }

                    self.cursor.pos.y = row_idx;
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
                // If scrolled back and writing output: jump to bottom.
                if self.scroll_offset > 0 {
                    self.scroll_offset = 0;
                }

                // LNM: Linefeed implies carriage return
                if self.lnm_enabled {
                    self.cursor.pos.x = 0;
                }

                // Always move down one row
                self.cursor.pos.y += 1;

                // If row doesn't exist, create a new hard-break row
                if self.cursor.pos.y >= self.rows.len() {
                    self.rows.push(Row::new_with_origin(
                        self.width,
                        RowOrigin::HardBreak,
                        RowJoin::NewLogicalLine,
                    ));
                } else {
                    // If row does exist, LF still means: row begins a logical line
                    let row = &mut self.rows[self.cursor.pos.y];
                    row.origin = RowOrigin::HardBreak;
                    row.join = RowJoin::NewLogicalLine;
                    row.clear();
                }

                // Keep scrollback cap
                self.enforce_scrollback_limit();
            }

            BufferType::Alternate => {
                if self.lnm_enabled {
                    self.cursor.pos.x = 0;
                }

                if self.cursor.pos.y + 1 < self.height {
                    self.cursor.pos.y += 1;
                } else {
                    self.scroll_up(); // fixed-size alternate screen buffer
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

#[cfg(test)]
mod pty_behavior_tests {
    use super::*;
    use crate::row::{RowJoin, RowOrigin};
    use freminal_common::buffer_states::tchar::TChar;

    // Helper: convert &str to Vec<TChar::Ascii>
    fn to_tchars(s: &str) -> Vec<TChar> {
        s.bytes().map(TChar::Ascii).collect()
    }

    // Helper: pretty row origins for debugging
    fn row_kinds(buf: &Buffer) -> Vec<(RowOrigin, RowJoin)> {
        buf.rows.iter().map(|r| (r.origin, r.join)).collect()
    }

    // B1 — CR-only redraw: no new rows, cursor stays on same row
    #[test]
    fn cr_only_redraw_does_not_create_new_rows() {
        // width large enough to not wrap
        let mut buf = Buffer::new(100, 20);

        let initial_rows = buf.rows.len();
        let initial_row_y = buf.cursor.pos.y;

        // "Loading 1%\rLoading 2%\rLoading 3%\r"
        buf.insert_text(&to_tchars("Loading 1%"));
        buf.handle_cr();

        let row_after_first = buf.cursor.pos.y;
        assert_eq!(
            row_after_first, initial_row_y,
            "CR should not move to a new row"
        );

        buf.insert_text(&to_tchars("Loading 2%"));
        buf.handle_cr();

        buf.insert_text(&to_tchars("Loading 3%"));
        buf.handle_cr();

        // Still on the same physical row, and no extra rows created by CR
        assert_eq!(
            buf.cursor.pos.y, initial_row_y,
            "CR redraw loop should not change row index"
        );
        assert_eq!(
            buf.rows.len(),
            initial_rows,
            "CR redraw loop should not create new rows"
        );
    }

    // B2 — CRLF newline pattern: one new row per LF
    #[test]
    fn crlf_creates_new_logical_lines() {
        let mut buf = Buffer::new(100, 20);

        let start_row = buf.cursor.pos.y;

        // "hello\r\nworld\r\n"
        buf.insert_text(&to_tchars("hello"));
        buf.handle_cr();
        buf.handle_lf(); // first CRLF

        let after_first_lf_row = buf.cursor.pos.y;
        assert_eq!(
            after_first_lf_row,
            start_row + 1,
            "First CRLF should move cursor to next row"
        );

        buf.insert_text(&to_tchars("world"));
        buf.handle_cr();
        buf.handle_lf(); // second CRLF

        let after_second_lf_row = buf.cursor.pos.y;
        assert_eq!(
            after_second_lf_row,
            start_row + 2,
            "Second CRLF should move cursor down one more row"
        );

        // Check row metadata of the line starts
        let kinds = row_kinds(&buf);

        // At least three rows now: initial + two LF-created rows
        assert!(
            kinds.len() >= (start_row + 3),
            "Expected at least three rows after two CRLFs"
        );

        let first_line = kinds[start_row];
        let second_line = kinds[start_row + 1];
        let third_line = kinds[start_row + 2];

        // All LF-started rows should be HardBreak + NewLogicalLine
        assert_eq!(
            first_line.0,
            RowOrigin::HardBreak,
            "Initial line should be a HardBreak logical start"
        );
        assert_eq!(
            first_line.1,
            RowJoin::NewLogicalLine,
            "Initial row should begin a logical line"
        );

        assert_eq!(
            second_line.0,
            RowOrigin::HardBreak,
            "Row after first LF should be HardBreak"
        );
        assert_eq!(
            second_line.1,
            RowJoin::NewLogicalLine,
            "Row after first LF should begin a new logical line"
        );

        assert_eq!(
            third_line.0,
            RowOrigin::HardBreak,
            "Row after second LF should be HardBreak"
        );
        assert_eq!(
            third_line.1,
            RowJoin::NewLogicalLine,
            "Row after second LF should begin a new logical line"
        );
    }

    // B3 — Soft-wrap mid-insertion: long text overflows width into SoftWrap row
    #[test]
    fn soft_wrap_marks_continuation_rows() {
        let width = 10;
        let mut buf = Buffer::new(width, 100);

        let start_row = buf.cursor.pos.y;

        buf.insert_text(&to_tchars("1234567890ABCDE"));

        // Look for a SoftWrap row after start_row
        let kinds = row_kinds(&buf);
        let mut found = false;
        for (idx, (origin, join)) in kinds.iter().enumerate().skip(start_row + 1) {
            if *origin == RowOrigin::SoftWrap && *join == RowJoin::ContinueLogicalLine {
                found = true;
                // Optionally: assert cursor ended up here
                assert_eq!(
                    buf.cursor.pos.y, idx,
                    "Cursor should end on the soft-wrapped continuation row"
                );
                break;
            }
        }

        assert!(found,"Soft-wrap should produce at least one SoftWrap/ContinueLogicalLine row after the first");
    }

    // B6-ish — Wrap into an existing row: reused row must become SoftWrap continuation
    #[test]
    fn soft_wrap_reuses_existing_next_row_as_continuation() {
        let width = 8;
        let mut buf = Buffer::new(width, 100);

        // Fill the first row exactly, starting from 0
        buf.insert_text(&to_tchars("ABCDEFGH")); // 8 chars

        let first_row = buf.cursor.pos.y;
        assert_eq!(first_row, 0);

        // Now write more to force a wrap into the next row
        buf.insert_text(&to_tchars("ABC"));

        // Cursor must now be on the next row
        let second_row = buf.cursor.pos.y;
        assert_eq!(
            second_row,
            first_row + 1,
            "Soft-wrap should move cursor to next row"
        );

        let kinds = row_kinds(&buf);
        let wrapped = kinds[second_row];

        assert_eq!(
            wrapped.0,
            RowOrigin::SoftWrap,
            "Wrapped row should have SoftWrap origin"
        );
        assert_eq!(
            wrapped.1,
            RowJoin::ContinueLogicalLine,
            "Wrapped row should continue the logical line"
        );
    }

    #[test]
    fn cr_only_redraw_never_creates_new_rows_even_after_wrap() {
        let mut buf = Buffer::new(10, 100);

        buf.insert_text(&to_tchars("1234567890")); // full row
        let row0 = buf.cursor.pos.y;

        buf.handle_cr(); // reset X
        buf.insert_text(&to_tchars("HELLO"));

        assert_eq!(buf.cursor.pos.y, row0, "CR must not change row");
        assert_eq!(buf.rows.len(), 1, "No new row must be created");
    }

    #[test]
    fn lf_after_softwrap_creates_new_hardbreak_row() {
        let mut buf = Buffer::new(5, 100);

        buf.insert_text(&to_tchars("123456789")); // wraps
        assert!(matches!(buf.rows[1].origin, RowOrigin::SoftWrap));

        buf.handle_lf(); // HARD BREAK

        let last = buf.cursor.pos.y;
        assert!(matches!(buf.rows[last].origin, RowOrigin::HardBreak));
        assert!(matches!(buf.rows[last].join, RowJoin::NewLogicalLine));
    }

    #[test]
    fn crlf_moves_to_new_hardbreak_row() {
        let mut buf = Buffer::new(20, 100);

        buf.insert_text(&to_tchars("hello"));
        buf.handle_cr();
        buf.handle_lf();

        let y = buf.cursor.pos.y;
        assert!(y == 1);
        assert!(matches!(buf.rows[1].origin, RowOrigin::HardBreak));
    }

    #[test]
    fn lnm_enabled_lf_behaves_like_crlf() {
        let mut buf = Buffer::new(20, 100);
        buf.lnm_enabled = true;

        buf.insert_text(&to_tchars("hello"));
        buf.cursor.pos.x = 5;

        buf.handle_lf(); // LNM → CRLF

        assert_eq!(buf.cursor.pos.x, 0, "LNM LF resets X to 0");
        assert_eq!(buf.cursor.pos.y, 1, "LNM LF advances row");
    }

    #[test]
    fn cr_inside_softwrap_does_not_create_new_logical_line() {
        let mut buf = Buffer::new(5, 100);

        buf.insert_text(&to_tchars("123456")); // soft-wrap at 5
        assert!(matches!(buf.rows[1].origin, RowOrigin::SoftWrap));

        buf.handle_cr(); // redraw start of continuation row

        buf.insert_text(&to_tchars("ZZ"));

        assert!(matches!(buf.rows[1].origin, RowOrigin::SoftWrap));
        assert_eq!(buf.cursor.pos.y, 1);
    }
}
