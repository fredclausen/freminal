// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::error::ParserFailures;
use anyhow::Result;
use unicode_segmentation::UnicodeSegmentation;

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

    /// Convert a vector of u8s to a vector of `TChars`
    /// The assumption here is that the vector of u8s will contain one or more `TChars`.
    /// If the byte vector is known to contain a single `TChar`, then use `TChar::from` instead.
    ///
    /// # Errors
    /// Will return an error if the vector is not a valid utf8 string, or if the vector contains characters that are not valid `TChar`
    ///
    pub fn from_vec(v: &[u8]) -> Result<Vec<Self>> {
        let data_as_string = std::str::from_utf8(v)?;
        let graphemes = data_as_string
            .graphemes(true)
            .collect::<Vec<&str>>()
            .clone();

        Self::from_vec_of_graphemes(&graphemes)
    }

    /// Convert a vector of graphemes to a vector of `TChar`
    /// The assumption here is that the vector of graphemes will contain one or more `TChars`.
    ///
    /// # Errors
    /// Will return an error if the vector contains characters that are not valid `TChar`
    ///
    pub fn from_vec_of_graphemes(v: &[&str]) -> Result<Vec<Self>> {
        v.iter()
            .map(|s| {
                Ok(if s.len() == 1 {
                    Self::new_from_single_char(s.as_bytes()[0])
                } else {
                    match Self::new_from_many_chars(s.as_bytes().to_vec()) {
                        Ok(c) => c,
                        Err(e) => {
                            return Err(e);
                        }
                    }
                })
            })
            .collect::<Result<Vec<Self>>>()
    }

    /// Convert a String to a vector of `TChar`
    ///
    /// # Errors
    /// Will return an error if the string is not valid utf8 or the string contains characters that are not valid `TChar`
    pub fn from_string(s: &str) -> Result<Vec<Self>> {
        let graphemes = s.graphemes(true).collect::<Vec<&str>>();

        graphemes
            .iter()
            .map(|s| {
                Ok(if s.len() == 1 {
                    Self::new_from_single_char(s.as_bytes()[0])
                } else {
                    match Self::new_from_many_chars(s.as_bytes().to_vec()) {
                        Ok(c) => c,
                        Err(e) => {
                            return Err(e);
                        }
                    }
                })
            })
            .collect::<Result<Vec<Self>>>()
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
