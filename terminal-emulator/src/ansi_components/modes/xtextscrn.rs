// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::fmt;

use crate::ansi_components::mode::SetMode;

/// Alternate Screen (`XT_EXTSCRN`)
#[derive(Debug, Eq, PartialEq, Default)]
pub enum XtExtscrn {
    /// Primary screen
    /// Clear screen, switch to normal screen buffer, and restore cursor position.
    #[default]
    Primary,
    /// Save cursor position, switch to alternate screen buffer, and clear screen.
    /// Also known as the "alternate screen".
    Alternate,
}

impl XtExtscrn {
    #[must_use]
    pub const fn new(mode: &SetMode) -> Self {
        match mode {
            SetMode::DecSet => Self::Alternate,
            SetMode::DecRst => Self::Primary,
        }
    }
}

impl fmt::Display for XtExtscrn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Primary => f.write_str("XT_EXTSCRN (RESET) Primary Screen"),
            Self::Alternate => f.write_str("XT_EXTSCRN (SET) Alternate Screen"),
        }
    }
}
