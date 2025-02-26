// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::fmt;

use crate::ansi_components::mode::SetMode;

use super::ReportMode;

/// Show cursor Reverse Wrap Around ?45
#[derive(Debug, Eq, PartialEq, Default, Clone)]
pub enum ReverseWrapAround {
    #[default]
    /// Normal (Set) Mode
    /// Show cursor.
    WrapAround,
    /// Alternate (Reset) Mode
    /// Hide cursor.
    DontWrap,
    Query,
}

impl ReportMode for ReverseWrapAround {
    fn report(&self, override_mode: Option<SetMode>) -> String {
        override_mode.map_or_else(
            || match self {
                Self::DontWrap => "\x1b[?45;2$y".to_string(),
                Self::WrapAround => "\x1b[?45;1$y".to_string(),
                Self::Query => "\x1b[?45;0$y".to_string(),
            },
            |override_mode| match override_mode {
                SetMode::DecSet => "\x1b[?45;1$y".to_string(),
                SetMode::DecRst => "\x1b[?45;2$y".to_string(),
                SetMode::DecQuery => "\x1b[?45;0$y".to_string(),
            },
        )
    }
}

impl ReverseWrapAround {
    #[must_use]
    pub const fn new(mode: &SetMode) -> Self {
        match mode {
            SetMode::DecSet => Self::WrapAround,
            SetMode::DecRst => Self::DontWrap,
            SetMode::DecQuery => Self::Query,
        }
    }
}

impl fmt::Display for ReverseWrapAround {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrapAround => write!(f, "Wrap Around"),
            Self::DontWrap => write!(f, "No Wrap Around"),
            Self::Query => write!(f, "Query Wrap Around"),
        }
    }
}
