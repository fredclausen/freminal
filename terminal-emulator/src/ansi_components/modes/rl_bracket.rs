// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::fmt;

use crate::ansi_components::mode::SetMode;

/// Bracketed Paste (`RL_BRACKET`) Mode ?2004
#[derive(Debug, Default, Eq, PartialEq, Clone)]
pub enum RlBracket {
    #[default]
    /// Normal (Reset) Mode
    /// Bracketed paste mode is disabled
    Disabled,
    /// Alternate (Set) Mode
    /// Bracketed paste mode is enabled and the terminal will send ESC [200~ and ESC [201~ around pasted text
    Enabled,
}

impl RlBracket {
    #[must_use]
    pub const fn new(mode: &SetMode) -> Self {
        match mode {
            SetMode::DecSet => Self::Enabled,
            SetMode::DecRst => Self::Disabled,
        }
    }
}

impl fmt::Display for RlBracket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => write!(f, "Bracketed Paste Mode (DEC 2004) Disabled"),
            Self::Enabled => write!(f, "Bracketed Paste Mode (DEC 2004) Enabled"),
        }
    }
}
