// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::fmt;

use crate::ansi_components::mode::SetMode;

// FIXME: I'm not sure we actually want to blink the cursor.
// Most terminals seem to either not do this, or give the user the option to disable it.
// For now, we'll track it and decide later.

/// Alternate Screen (`XT_EXTSCRN`)
#[derive(Debug, Eq, PartialEq, Default, Clone)]
pub enum XtCBlink {
    /// Reset mode. Default.
    /// Cursor is steady and not blinking.
    #[default]
    Steady,
    /// Set mode.
    /// Cursor is blinking.
    Blinking,
}

impl XtCBlink {
    #[must_use]
    pub const fn new(mode: &SetMode) -> Self {
        match mode {
            SetMode::DecSet => Self::Blinking,
            SetMode::DecRst => Self::Steady,
        }
    }
}

impl fmt::Display for XtCBlink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Steady => f.write_str("XT_CBLINK (RESET) Cursor Steady"),
            Self::Blinking => f.write_str("XT_CBLINK (SET) Cursor Blinking"),
        }
    }
}
