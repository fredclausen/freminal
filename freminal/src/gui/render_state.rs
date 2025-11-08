// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::gui::terminal::{render_terminal_text, CachedRow};
use std::collections::HashSet;

/// Tracks terminal text lines and which rows require redraw.
#[derive(Default)]
pub struct TerminalRenderState {
    pub lines: Vec<String>,
    pub row_cache: Vec<CachedRow>,
    dirty_rows: HashSet<usize>,
}

impl TerminalRenderState {
    pub fn mark_dirty(&mut self, row: usize) {
        self.dirty_rows.insert(row);
    }

    pub fn mark_all_dirty(&mut self) {
        self.dirty_rows = (0..self.lines.len()).collect();
    }

    pub fn take_and_clear_dirty_rows(&mut self) -> Vec<usize> {
        self.dirty_rows.drain().collect()
    }

    pub fn ensure_row_cache(&mut self) {
        if self.row_cache.len() < self.lines.len() {
            self.row_cache
                .resize_with(self.lines.len(), Default::default);
        }
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        job: &egui::text::LayoutJob,
        font_size: f32,
    ) -> egui::Response {
        self.ensure_row_cache();
        let full_text = self.lines.join("\n");
        let dirty = self.take_and_clear_dirty_rows();
        let dirty_opt = if dirty.is_empty() {
            None
        } else {
            Some(&dirty[..])
        };
        render_terminal_text(
            ui,
            &full_text,
            job,
            font_size,
            &mut self.row_cache,
            dirty_opt,
        )
    }
}
