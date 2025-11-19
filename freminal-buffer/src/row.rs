// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use freminal_common::buffer_states::{cursor::CursorPos, format_tag::FormatTag, tchar::TChar};

use crate::{cell::Cell, response::InsertResponse};

pub struct Row {
    characters: Vec<Cell>,
    width: usize,
}

impl Row {
    #[must_use]
    pub const fn new(width: usize) -> Self {
        let characters = Vec::new();
        Self { characters, width }
    }

    #[must_use]
    pub const fn get_row_width(&self) -> usize {
        self.characters.len()
    }

    #[must_use]
    pub fn get_char_at(&self, col: usize) -> Option<&Cell> {
        self.characters.get(col)
    }

    #[must_use]
    pub const fn get_characters(&self) -> &Vec<Cell> {
        &self.characters
    }

    pub fn insert_text(
        &mut self,
        start_col: usize,
        text: &[TChar],
        tag: &FormatTag,
        row_width: usize,
    ) -> InsertResponse {
        let mut col = start_col;

        for (i, tchar) in text.iter().enumerate() {
            let w = tchar.display_width();

            // Overflow must return progress up to this point
            if col + w > row_width {
                return InsertResponse::Leftover {
                    data: text[i..].to_vec(),
                    final_col: col, // ✔ xterm-compatible behavior
                };
            }

            // Insert "head" cell
            let head_cell = Cell::new(tchar.clone(), tag.clone());

            if col < self.characters.len() {
                self.characters[col] = head_cell;
            } else {
                self.characters.push(head_cell);
            }

            // Insert continuation cells if wide char
            for offset in 1..w {
                let cont_cell = Cell::wide_continuation();
                let idx = col + offset;

                if idx < self.characters.len() {
                    self.characters[idx] = cont_cell;
                } else {
                    self.characters.push(cont_cell);
                }
            }

            col += w;
        }

        InsertResponse::Consumed(col)
    }
}
