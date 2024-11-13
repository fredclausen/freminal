// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use freminal_common::colors::TerminalColor;

use crate::ansi_components::mode::Decawm;

use super::fonts::{FontDecorations, FontWeight};

#[allow(clippy::module_name_repetitions)]
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct CursorState {
    pub pos: CursorPos,
    pub font_weight: FontWeight,
    pub font_decorations: Vec<FontDecorations>,
    pub color: TerminalColor,
    pub background_color: TerminalColor,
    pub underline_color: TerminalColor,
    pub line_wrap_mode: Decawm,
}

impl Default for CursorState {
    fn default() -> Self {
        Self {
            pos: CursorPos::default(),
            font_weight: FontWeight::default(),
            font_decorations: Vec::new(),
            color: TerminalColor::Default,
            background_color: TerminalColor::DefaultBackground,
            underline_color: TerminalColor::DefaultUnderlineColor,
            line_wrap_mode: Decawm::default(),
        }
    }
}

// FIXME: it would be cool to not lint this out
#[allow(dead_code)]
impl CursorState {
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn with_background_color(mut self, background_color: TerminalColor) -> Self {
        self.background_color = background_color;
        self
    }

    #[must_use]
    pub const fn with_color(mut self, color: TerminalColor) -> Self {
        self.color = color;
        self
    }

    #[must_use]
    pub const fn with_font_weight(mut self, font_weight: FontWeight) -> Self {
        self.font_weight = font_weight;
        self
    }

    #[must_use]
    pub fn with_font_decorations(mut self, font_decorations: Vec<FontDecorations>) -> Self {
        self.font_decorations = font_decorations;
        self
    }

    #[must_use]
    pub const fn with_pos(mut self, pos: CursorPos) -> Self {
        self.pos = pos;
        self
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CursorPos {
    pub x: usize,
    pub y: usize,
    // pub x_as_characters: usize,
}
