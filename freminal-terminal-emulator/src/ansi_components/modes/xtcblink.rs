// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::fmt;

use crate::ansi_components::mode::SetMode;

use super::ReportMode;

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
    Query,
}

impl XtCBlink {
    #[must_use]
    pub const fn new(mode: &SetMode) -> Self {
        match mode {
            SetMode::DecSet => Self::Blinking,
            SetMode::DecRst => Self::Steady,
            SetMode::DecQuery => Self::Query,
        }
    }
}

impl ReportMode for XtCBlink {
    fn report(&self, override_mode: Option<SetMode>) -> String {
        override_mode.map_or_else(
            || match self {
                Self::Steady => "\x1b[?12;2$y".to_string(),
                Self::Blinking => "\x1b[?12;1$y".to_string(),
                Self::Query => "\x1b[?12;0$y".to_string(),
            },
            |override_mode| match override_mode {
                SetMode::DecSet => "\x1b[?12;1$y".to_string(),
                SetMode::DecRst => "\x1b[?12;2$y".to_string(),
                SetMode::DecQuery => "\x1b[?12;0$y".to_string(),
            },
        )
    }
}

impl fmt::Display for XtCBlink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Steady => f.write_str("XT_CBLINK (RESET) Cursor Steady"),
            Self::Blinking => f.write_str("XT_CBLINK (SET) Cursor Blinking"),
            Self::Query => f.write_str("XT_CBLINK (QUERY)"),
        }
    }
}
