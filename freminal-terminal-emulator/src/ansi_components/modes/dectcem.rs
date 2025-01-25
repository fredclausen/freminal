// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::fmt;

use crate::ansi_components::mode::SetMode;

use super::ReportMode;

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
    Query,
}

impl ReportMode for Dectcem {
    fn report(&self, override_mode: Option<SetMode>) -> String {
        override_mode.map_or_else(
            || match self {
                Self::Hide => "\x1b[?25;2$y".to_string(),
                Self::Show => "\x1b[?25;1$y".to_string(),
                Self::Query => "\x1b[?25;0$y".to_string(),
            },
            |override_mode| match override_mode {
                SetMode::DecSet => "\x1b[?25;1$y".to_string(),
                SetMode::DecRst => "\x1b[?25;2$y".to_string(),
                SetMode::DecQuery => "\x1b[?25;0$y".to_string(),
            },
        )
    }
}

impl Dectcem {
    #[must_use]
    pub const fn new(mode: &SetMode) -> Self {
        match mode {
            SetMode::DecSet => Self::Show,
            SetMode::DecRst => Self::Hide,
            SetMode::DecQuery => Self::Query,
        }
    }
}

impl fmt::Display for Dectcem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Show => write!(f, "Show Cursor (DECTCEM)"),
            Self::Hide => write!(f, "Hide Cursor (DECTCEM)"),
            Self::Query => write!(f, "Query Cursor (DECTCEM)"),
        }
    }
}
