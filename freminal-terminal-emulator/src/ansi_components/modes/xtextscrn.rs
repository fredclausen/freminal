// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::fmt;

use crate::ansi_components::mode::SetMode;

use super::ReportMode;

/// Alternate Screen (`XT_EXTSCRN`) ?1049
#[derive(Debug, Eq, PartialEq, Default)]
pub enum XtExtscrn {
    /// Primary screen
    /// Clear screen, switch to normal screen buffer, and restore cursor position.
    #[default]
    Primary,
    /// Save cursor position, switch to alternate screen buffer, and clear screen.
    /// Also known as the "alternate screen".
    Alternate,
    Query,
}

impl XtExtscrn {
    #[must_use]
    pub const fn new(mode: &SetMode) -> Self {
        match mode {
            SetMode::DecSet => Self::Alternate,
            SetMode::DecRst => Self::Primary,
            SetMode::DecQuery => Self::Query,
        }
    }
}

impl ReportMode for XtExtscrn {
    fn report(&self, override_mode: Option<SetMode>) -> String {
        override_mode.map_or_else(
            || match self {
                Self::Primary => "\x1b[?1049;2$y".to_string(),
                Self::Alternate => "\x1b[?1049;1$y".to_string(),
                Self::Query => "\x1b[?1049;0$y".to_string(),
            },
            |override_mode| match override_mode {
                SetMode::DecSet => "\x1b[?1049;1$y".to_string(),
                SetMode::DecRst => "\x1b[?1049;2$y".to_string(),
                SetMode::DecQuery => "\x1b[?1049;0$y".to_string(),
            },
        )
    }
}

impl fmt::Display for XtExtscrn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Primary => f.write_str("XT_EXTSCRN (RESET) Primary Screen"),
            Self::Alternate => f.write_str("XT_EXTSCRN (SET) Alternate Screen"),
            Self::Query => f.write_str("XT_EXTSCRN (QUERY)"),
        }
    }
}
