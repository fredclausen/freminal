// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::fmt;

use crate::ansi_components::mode::SetMode;

use super::ReportMode;

/// Show cursor (DECSCNM) ?5
#[derive(Debug, Eq, PartialEq, Default, Clone)]
pub enum Decscnm {
    #[default]
    NormalDisplay,
    ReverseDisplay,
    Query,
}

impl ReportMode for Decscnm {
    fn report(&self, override_mode: Option<SetMode>) -> String {
        override_mode.map_or_else(
            || match self {
                Self::NormalDisplay => "\x1b[?5;2$y".to_string(),
                Self::ReverseDisplay => "\x1b[?5;1$y".to_string(),
                Self::Query => "\x1b[?5;0$y".to_string(),
            },
            |override_mode| match override_mode {
                SetMode::DecSet => "\x1b[?5;1$y".to_string(),
                SetMode::DecRst => "\x1b[?5;2$y".to_string(),
                SetMode::DecQuery => "\x1b[?5;0$y".to_string(),
            },
        )
    }
}

impl Decscnm {
    #[must_use]
    pub const fn new(mode: &SetMode) -> Self {
        match mode {
            SetMode::DecSet => Self::ReverseDisplay,
            SetMode::DecRst => Self::NormalDisplay,
            SetMode::DecQuery => Self::Query,
        }
    }

    #[must_use]
    pub const fn is_normal_display(&self) -> bool {
        matches!(self, Self::NormalDisplay)
    }
}

impl fmt::Display for Decscnm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NormalDisplay => write!(f, "Normal Display"),
            Self::ReverseDisplay => write!(f, "Reverse Display"),
            Self::Query => write!(f, "Query"),
        }
    }
}
