// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use freminal_common::buffer_states::{format_tag::FormatTag, tchar::TChar};

use crate::{cell::Cell, response::InsertResponse};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowOrigin {
    HardBreak,
    SoftWrap,
    ScrollFill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowJoin {
    NewLogicalLine,
    ContinueLogicalLine,
}

#[derive(Debug, Clone)]
pub struct Row {
    cells: Vec<Cell>,
    width: usize,
    pub origin: RowOrigin,
    pub join: RowJoin,
}

impl Row {
    #[must_use]
    pub const fn new(width: usize) -> Self {
        Self {
            cells: Vec::new(),
            width,
            origin: RowOrigin::ScrollFill,
            join: RowJoin::NewLogicalLine,
        }
    }

    #[must_use]
    pub const fn new_with_origin(width: usize, origin: RowOrigin, join: RowJoin) -> Self {
        Self {
            cells: Vec::new(),
            width,
            origin,
            join,
        }
    }

    #[must_use]
    pub const fn from_cells(
        width: usize,
        origin: RowOrigin,
        join: RowJoin,
        cells: Vec<Cell>,
    ) -> Self {
        Self {
            cells,
            width,
            origin,
            join,
        }
    }

    pub fn clear(&mut self) {
        self.cells.clear();
    }

    /// Logical row width (number of *columns*), not number of occupied cells.
    #[must_use]
    pub const fn max_width(&self) -> usize {
        self.width
    }

    /// Update the logical width of this row (number of columns).
    /// This does *not* change the existing cells, only the max width metadata.
    pub const fn set_max_width(&mut self, new_width: usize) {
        self.width = new_width;
    }

    /// How many cells are currently occupied.
    #[must_use]
    pub fn get_row_width(&self) -> usize {
        let mut cols = 0;
        let mut idx = 0;

        while idx < self.cells.len() {
            let cell = &self.cells[idx];
            if cell.is_head() {
                cols += cell.display_width();
                idx += cell.display_width();
            } else {
                // Continuations should always follow heads,
                // but if encountered, advance by 1 cell.
                idx += 1;
            }
        }

        cols
    }

    #[must_use]
    pub fn get_char_at(&self, idx: usize) -> Option<&Cell> {
        self.cells.get(idx)
    }

    #[must_use]
    pub const fn get_characters(&self) -> &Vec<Cell> {
        &self.cells
    }

    /// Clean up when overwriting wide cells:
    /// - If overwriting a continuation, clear the head + all its continuations.
    /// - If overwriting a head, clear its continuations.
    fn cleanup_wide_overwrite(&mut self, col: usize) {
        if col >= self.cells.len() {
            return;
        }

        // Overwriting a continuation: clean up head + all continuations.
        if self.cells[col].is_continuation() {
            if col == 0 {
                // Invariant violation; nothing to the left
                return;
            }
            // find head to the left
            let mut head = col - 1;
            while head > 0 && !self.cells[head].is_head() {
                head -= 1;
            }
            if !self.cells[head].is_head() {
                return;
            }

            // clear head + all following continuations
            let mut idx = head;
            while idx < self.cells.len() && self.cells[idx].is_continuation() || idx == head {
                self.cells[idx] = Cell::new(TChar::Space, FormatTag::default());
                idx += 1;
                if idx >= self.cells.len() {
                    break;
                }
            }
            return;
        }

        // Overwriting a head: clear trailing continuations
        if self.cells[col].is_head() {
            let mut idx = col + 1;
            while idx < self.cells.len() && self.cells[idx].is_continuation() {
                self.cells[idx] = Cell::new(TChar::Space, FormatTag::default());
                idx += 1;
            }
        }
    }

    pub fn insert_text(
        &mut self,
        start_col: usize,
        text: &[TChar],
        tag: &FormatTag,
    ) -> InsertResponse {
        let mut col = start_col;

        // ---------------------------------------------------------------
        // SAFETY CHECK: If start_col is out of bounds, nothing fits here.
        // We just report all text as leftover.
        // ---------------------------------------------------------------
        if col >= self.width {
            return InsertResponse::Consumed(col);
        }

        // ---------------------------------------------------------------
        // Walk each character and try to insert it.
        // ---------------------------------------------------------------
        for (i, tchar) in text.iter().enumerate() {
            let w = tchar.display_width();

            // RULE 1: Glyph won't fit — overflow mid-insertion.
            if col + w > self.width {
                return InsertResponse::Leftover {
                    data: text[i..].to_vec(),
                    final_col: col,
                };
            }

            // -----------------------------------------------------------
            // Pad row up to current col if needed
            // -----------------------------------------------------------
            if col > self.cells.len() {
                let pad = col - self.cells.len();
                for _ in 0..pad {
                    self.cells.push(Cell::new(TChar::Space, tag.clone()));
                }
            }

            // -----------------------------------------------------------
            // Overwriting a wide glyph head/continuation?
            // Clean up any continuation cells.
            // -----------------------------------------------------------
            if col < self.cells.len() {
                self.cleanup_wide_overwrite(col);
            }

            // -----------------------------------------------------------
            // Insert head cell
            // -----------------------------------------------------------
            // Ensure vector is large enough
            if self.cells.len() < col + w {
                self.cells
                    .resize(col + w, Cell::new(TChar::Space, tag.clone()));
            }

            // Insert head
            self.cells[col] = Cell::new(tchar.clone(), tag.clone());

            // Insert continuation cells
            for offset in 1..w {
                self.cells[col + offset] = Cell::wide_continuation();
            }

            // Move column forward
            col += w;

            // RULE 2: Clamp col to width
            if col > self.width {
                col = self.width;
            }
        }

        // ---------------------------------------------------------------
        // All text successfully inserted on this row.
        // ---------------------------------------------------------------
        InsertResponse::Consumed(col)
    }
}
