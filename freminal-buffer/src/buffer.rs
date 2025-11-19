// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use freminal_common::{
    buffer_states::{cursor::CursorState, format_tag::FormatTag, tchar::TChar},
    config::FontConfig,
};

use crate::{response::InsertResponse, row::Row};

pub struct Buffer {
    rows: Vec<Row>,
    width: usize,
    height: usize,
    cursor: CursorState,
    current_tag: FormatTag,
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

            // Step 3 — Now it's safe to index into rows[row_idx]

            match self.rows[row_idx].insert_text(col, &remaining, tag) {
                InsertResponse::Consumed(final_col) => {
                    self.cursor.pos.x = final_col;
                    self.cursor.pos.y = row_idx;
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
}
