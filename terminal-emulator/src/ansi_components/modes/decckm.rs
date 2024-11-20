// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::fmt;

use crate::ansi_components::mode::SetMode;

/// Cursor Key Mode (DECCKM)
#[derive(Eq, PartialEq, Debug, Default, Clone)]
pub enum Decckm {
    #[default]
    /// Normal (Reset) Mode
    /// Cursor keys send ANSI control codes
    Ansi,
    /// Alternate (Set) Mode
    /// Cursor keys send application control codes
    Application,
}

impl Decckm {
    #[must_use]
    pub const fn new(mode: &SetMode) -> Self {
        match mode {
            SetMode::Set => Self::Application,
            SetMode::Reset => Self::Ansi,
        }
    }
}

impl fmt::Display for Decckm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ansi => write!(f, "Cursor Key Mode (DECCKM) ANSI"),
            Self::Application => write!(f, "Cursor Key Mode (DECCKM) Application"),
        }
    }
}
