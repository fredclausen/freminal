// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use freminal_common::buffer_states::{format_tag::FormatTag, tchar::TChar};

use crate::{cell::Cell, response::InsertResponse};

#[derive(Debug, Clone)]
pub struct Row {
    characters: Vec<Cell>,
    width: usize,
    remaining_width: usize,
}

impl Row {
    #[must_use]
    pub const fn new(width: usize) -> Self {
        Self {
            characters: Vec::new(),
            width,
            remaining_width: width,
        }
    }

    /// Logical row width (number of *columns*), not number of occupied cells.
    #[must_use]
    pub const fn max_width(&self) -> usize {
        self.width
    }

    /// How many cells are currently occupied.
    #[must_use]
    pub fn get_row_width(&self) -> usize {
        let mut cols = 0;
        let mut idx = 0;

        while idx < self.characters.len() {
            let cell = &self.characters[idx];
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
        self.characters.get(idx)
    }

    #[must_use]
    pub const fn get_characters(&self) -> &Vec<Cell> {
        &self.characters
    }

    /// Clean up when overwriting wide cells:
    /// - If overwriting a continuation, clear the head + all its continuations.
    /// - If overwriting a head, clear its continuations.
    fn cleanup_wide_overwrite(&mut self, col: usize) {
        if col >= self.characters.len() {
            return;
        }

        // Overwriting a continuation: clean up head + all continuations.
        if self.characters[col].is_continuation() {
            if col == 0 {
                // Invariant violation; nothing to the left
                return;
            }
            // find head to the left
            let mut head = col - 1;
            while head > 0 && !self.characters[head].is_head() {
                head -= 1;
            }
            if !self.characters[head].is_head() {
                return;
            }

            // clear head + all following continuations
            let mut idx = head;
            while idx < self.characters.len() && self.characters[idx].is_continuation()
                || idx == head
            {
                self.characters[idx] = Cell::new(TChar::Space, FormatTag::default());
                idx += 1;
                if idx >= self.characters.len() {
                    break;
                }
            }
            return;
        }

        // Overwriting a head: clear trailing continuations
        if self.characters[col].is_head() {
            let mut idx = col + 1;
            while idx < self.characters.len() && self.characters[idx].is_continuation() {
                self.characters[idx] = Cell::new(TChar::Space, FormatTag::default());
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

        // row cannot start past width
        if col >= self.width {
            return InsertResponse::Leftover {
                data: text.to_vec(),
                final_col: col,
            };
        }

        for (i, tchar) in text.iter().enumerate() {
            let w = tchar.display_width();

            // RULE 1: per-glyph overflow check
            if col + w > self.width {
                return InsertResponse::Leftover {
                    data: text[i..].to_vec(),
                    final_col: col,
                };
            }

            // --- Pad up to col (bounded) ---
            if col > self.characters.len() {
                let pad = col - self.characters.len();
                for _ in 0..pad {
                    self.characters.push(Cell::new(TChar::Space, tag.clone()));
                }
            }

            // --- Cleanup wide-glyph overwrite ---
            if col < self.characters.len() {
                self.cleanup_wide_overwrite(col);
            }

            // --- Insert head ---
            let head = Cell::new(tchar.clone(), tag.clone());
            if col < self.characters.len() {
                self.characters[col] = head;
            } else {
                self.characters.push(head);
            }

            // --- Insert continuation cells ---
            for offset in 1..w {
                let cont = Cell::wide_continuation();
                let idx = col + offset;

                if idx < self.characters.len() {
                    self.characters[idx] = cont;
                } else {
                    self.characters.push(cont);
                }
            }

            col += w;

            // RULE 2: clamp col to width to prevent infinite padding loops
            if col > self.width {
                col = self.width;
            }
        }

        InsertResponse::Consumed(col)
    }
}
