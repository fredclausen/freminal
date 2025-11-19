// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use freminal_common::buffer_states::{format_tag::FormatTag, tchar::TChar};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    value: TChar,
    format: FormatTag,
    is_wide_head: bool,
    is_wide_continuation: bool,
}

impl Cell {
    #[must_use]
    pub fn new(value: TChar, format: FormatTag) -> Self {
        let width = value.display_width();

        Self {
            value,
            format,
            is_wide_head: width > 1,
            is_wide_continuation: false,
        }
    }

    #[must_use]
    pub fn wide_continuation() -> Self {
        Self {
            value: TChar::Space, // filler glyph
            format: FormatTag::default(),
            is_wide_continuation: true,
            is_wide_head: false,
        }
    }

    #[must_use]
    pub const fn is_head(&self) -> bool {
        self.is_wide_head
    }

    #[must_use]
    pub const fn tchar(&self) -> &TChar {
        &self.value
    }

    #[must_use]
    pub fn display_width(&self) -> usize {
        self.value.display_width()
    }

    #[must_use]
    pub fn into_utf8(&self) -> String {
        match self.value {
            TChar::Ascii(c) => (c as char).to_string(),
            TChar::Utf8(ref bytes) => String::from_utf8_lossy(bytes).to_string(),
            TChar::Space => " ".to_string(),
            TChar::NewLine => "\n".to_string(),
        }
    }

    #[must_use]
    pub const fn is_continuation(&self) -> bool {
        self.is_wide_continuation
    }
}
