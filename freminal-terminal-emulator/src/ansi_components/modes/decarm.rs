// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::fmt;

use crate::ansi_components::mode::SetMode;

use super::ReportMode;

/// Show cursor (DECTCEM) ?25
#[derive(Debug, Eq, PartialEq, Default, Clone)]
pub enum Decarm {
    #[default]
    /// Normal (Set) Mode
    /// Repeat key presses.
    RepeatKey,
    /// Alternate (Reset) Mode
    /// Do not repeat keys.
    NoRepeatKey,
    Query,
}

impl ReportMode for Decarm {
    fn report(&self, override_mode: Option<SetMode>) -> String {
        override_mode.map_or_else(
            || match self {
                Self::NoRepeatKey => "\x1b[?8;2$y".to_string(),
                Self::RepeatKey => "\x1b[?8;1$y".to_string(),
                Self::Query => "\x1b[?8;0$y".to_string(),
            },
            |override_mode| match override_mode {
                SetMode::DecSet => "\x1b[?8;1$y".to_string(),
                SetMode::DecRst => "\x1b[?8;2$y".to_string(),
                SetMode::DecQuery => "\x1b[?8;0$y".to_string(),
            },
        )
    }
}

impl Decarm {
    #[must_use]
    pub const fn new(mode: &SetMode) -> Self {
        match mode {
            SetMode::DecSet => Self::RepeatKey,
            SetMode::DecRst => Self::NoRepeatKey,
            SetMode::DecQuery => Self::Query,
        }
    }
}

impl fmt::Display for Decarm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RepeatKey => write!(f, "Repeat Key (DECARM)"),
            Self::NoRepeatKey => write!(f, "No Repeat Key (DECARM)"),
            Self::Query => write!(f, "Query Repeat Key (DECARM)"),
        }
    }
}
