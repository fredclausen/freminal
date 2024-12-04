// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::fmt;

use crate::ansi_components::mode::SetMode;

/// Bracketed Paste (`SRM`) Mode ?12
#[derive(Debug, Default, Eq, PartialEq, Clone)]
pub enum Srm {
    #[default]
    /// Normal (Set) Mode
    /// SRM is disabled and the terminal will echo characters as they are typed
    NoLocalEcho,
    /// Alternate (Reset) Mode
    /// SRM is enabled and the terminal will not echo characters as they are typed
    /// Terminal will have to insert characters itself
    LocalEcho,
}

impl Srm {
    #[must_use]
    pub const fn new(mode: &SetMode) -> Self {
        match mode {
            SetMode::DecSet => Self::NoLocalEcho,
            SetMode::DecRst => Self::LocalEcho,
        }
    }
}

impl fmt::Display for Srm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoLocalEcho => write!(f, "Send Receive Mode (SRM) Disabled"),
            Self::LocalEcho => write!(f, "Send Receive Mode (SRM) Enabled"),
        }
    }
}
