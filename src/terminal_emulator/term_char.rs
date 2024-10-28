// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TChar {
    Ascii(u8),
    Utf8(Vec<u8>),
    Space,
    NewLine,
}

impl TChar {
    pub const fn new_from_single_char(c: u8) -> Self {
        match c {
            32 => Self::Space,
            10 => Self::NewLine,
            _ => Self::Ascii(c),
        }
    }

    pub const fn new_from_many_chars(v: Vec<u8>) -> Self {
        Self::Utf8(v)
    }

    pub const fn to_u8(&self) -> u8 {
        match self {
            Self::Ascii(c) => *c,
            _ => 0,
        }
    }
}

impl From<u8> for TChar {
    fn from(c: u8) -> Self {
        Self::new_from_single_char(c)
    }
}

impl From<Vec<u8>> for TChar {
    fn from(v: Vec<u8>) -> Self {
        Self::new_from_many_chars(v)
    }
}

impl PartialEq<u8> for TChar {
    fn eq(&self, other: &u8) -> bool {
        match self {
            Self::Ascii(c) => c == other,
            Self::Space => *other == 32,
            Self::NewLine => *other == 10,
            Self::Utf8(_) => false,
        }
    }
}

impl PartialEq<Vec<u8>> for TChar {
    fn eq(&self, other: &Vec<u8>) -> bool {
        match self {
            Self::Utf8(v) => v == other,
            _ => false,
        }
    }
}
