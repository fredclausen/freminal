// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use freminal_common::buffer_states::{format_tag::FormatTag, tchar::TChar};

pub struct Cell {
    value: TChar,
    format: FormatTag,
    continuation: bool,
}

impl Cell {
    #[must_use]
    pub const fn new(value: TChar, format: FormatTag) -> Self {
        Self {
            value,
            format,
            continuation: false,
        }
    }

    #[must_use]
    pub fn wide_continuation() -> Self {
        Self {
            value: TChar::Space, // filler glyph
            format: FormatTag::default(),
            continuation: true,
        }
    }

    #[must_use]
    pub const fn get_character(&self) -> &TChar {
        &self.value
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
}
