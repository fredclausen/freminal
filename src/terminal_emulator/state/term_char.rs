// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::terminal_emulator::error::ParserFailures;
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

// tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_from_single_char() {
        let c = TChar::new_from_single_char(32);
        assert_eq!(c, TChar::Space);

        let c = TChar::new_from_single_char(10);
        assert_eq!(c, TChar::NewLine);

        let c = TChar::new_from_single_char(65);
        assert_eq!(c, TChar::Ascii(65));
    }

    #[test]
    fn test_new_from_many_chars() {
        let c = TChar::new_from_many_chars(vec![65, 66, 67]).unwrap();
        assert_eq!(c, TChar::Utf8(vec![65, 66, 67]));
    }

    #[test]
    fn test_to_u8() {
        let c = TChar::Ascii(65);
        assert_eq!(c.to_u8(), 65);

        let c = TChar::Space;
        assert_eq!(c.to_u8(), 0);

        let c = TChar::NewLine;
        assert_eq!(c.to_u8(), 0);

        let c = TChar::Utf8(vec![65, 66, 67]);
        assert_eq!(c.to_u8(), 0);
    }

    #[test]
    fn test_from_u8() {
        let c: TChar = 65.into();
        assert_eq!(c, TChar::Ascii(65));
    }

    #[test]
    fn test_from_vec() {
        let c: TChar = vec![65, 66, 67].into();
        assert_eq!(c, TChar::Utf8(vec![65, 66, 67]));
    }

    #[test]
    fn test_eq_u8() {
        let c = TChar::Ascii(65);
        assert_eq!(c, 65);

        let c = TChar::Space;
        assert_eq!(c, 32);

        let c = TChar::NewLine;
        assert_eq!(c, 10);

        let c = TChar::Utf8(vec![65, 66, 67]);
        assert_ne!(c, 65);
    }

    #[test]
    fn test_eq_vec() {
        let c = TChar::Utf8(vec![65, 66, 67]);
        assert_eq!(c, vec![65, 66, 67]);

        let c = TChar::Ascii(65);
        assert_ne!(c, vec![65, 66, 67]);
    }

    #[test]
    fn test_invalid_utf8() {
        // Ѐ in to bytes
        let s = "\u{0400}".as_bytes();
        assert!(std::str::from_utf8(s).is_ok());

        // make sure the TChar::Utf8 will not panic
        assert!(std::panic::catch_unwind(|| TChar::new_from_many_chars(s.to_vec())).is_ok());

        // drop the last byte to make it invalid utf8
        let s = &s[..s.len() - 1];
        assert!(std::str::from_utf8(s).is_err());

        // now make sure the TChar::Utf8 will panic
        assert!(TChar::new_from_many_chars(s.to_vec()).is_err());
    }
}
