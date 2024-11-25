// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::fmt;

use crate::ansi_components::mode::SetMode;

/// Autowrap Mode (DECAWM) ?7
#[derive(Eq, PartialEq, Debug, Default, Clone)]
pub enum Decawm {
    #[default]
    /// Normal (Reset) Mode
    /// Disables autowrap mode.
    NoAutoWrap,
    /// Alternate (Set) Mode
    /// Enables autowrap mode
    AutoWrap,
}

impl Decawm {
    #[must_use]
    pub const fn new(mode: &SetMode) -> Self {
        match mode {
            SetMode::DecSet => Self::AutoWrap,
            SetMode::DecRst => Self::NoAutoWrap,
        }
    }
}

impl fmt::Display for Decawm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAutoWrap => write!(f, "Autowrap Mode (DECAWM) Disabled"),
            Self::AutoWrap => write!(f, "Autowrap Mode (DECAWM) Enabled"),
        }
    }
}
