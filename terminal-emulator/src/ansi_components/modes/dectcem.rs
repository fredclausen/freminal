// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::fmt;

use crate::ansi_components::mode::SetMode;

/// Show cursor (DECTCEM) ?25
#[derive(Debug, Eq, PartialEq, Default, Clone)]
pub enum Dectcem {
    #[default]
    /// Normal (Set) Mode
    /// Show cursor.
    Show,
    /// Alternate (Reset) Mode
    /// Hide cursor.
    Hide,
}

impl Dectcem {
    #[must_use]
    pub const fn new(mode: &SetMode) -> Self {
        match mode {
            SetMode::DecSet => Self::Show,
            SetMode::DecRst => Self::Hide,
        }
    }
}

impl fmt::Display for Dectcem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Show => write!(f, "Show Cursor (DECTCEM)"),
            Self::Hide => write!(f, "Hide Cursor (DECTCEM)"),
        }
    }
}
