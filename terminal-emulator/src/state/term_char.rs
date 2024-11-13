// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::error::ParserFailures;
use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TChar {
    Ascii(u8),
    Utf8(Vec<u8>),
    Space,
    NewLine,
}

impl TChar {
    #[must_use]
    pub const fn new_from_single_char(c: u8) -> Self {
        match c {
            32 => Self::Space,
            10 => Self::NewLine,
            _ => Self::Ascii(c),
        }
    }

    /// Create a new `TChar` from a vector of u8
    ///
    /// # Errors
    /// Will return an error if the vector is empty or is not a valid utf8 string
    pub fn new_from_many_chars(v: Vec<u8>) -> Result<Self> {
        // verify the vector is not empty and is a valid utf8 string
        if !v.is_empty() && std::str::from_utf8(&v).is_ok() {
            return Ok(Self::Utf8(v));
        }

        Err(ParserFailures::InvalidTChar(v).into())
    }

    // FIXME: this is fake news, it is used but clippy is not smart enough to see it
    #[allow(dead_code)]
    #[must_use]
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
        match Self::new_from_many_chars(v) {
            Ok(c) => c,
            Err(e) => {
                // FIXME: We should probably propagate the error instead of ignoring it
                error!("Error: {}. Will use ascii 0 character", e);
                Self::Ascii(0)
            }
        }
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
