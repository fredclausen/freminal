// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::fmt;

use crate::ansi_components::mode::SetMode;

use super::ReportMode;

/// Line Feed (LNM) ?20
#[derive(Debug, Eq, PartialEq, Default, Clone)]
pub enum Lnm {
    NewLine,
    #[default]
    LineFeed,
    Query,
}

impl ReportMode for Lnm {
    fn report(&self, override_mode: Option<SetMode>) -> String {
        override_mode.map_or_else(
            || match self {
                Self::NewLine => "\x1b[?20;1$y".to_string(),
                Self::LineFeed => "\x1b[?20;2$y".to_string(),
                Self::Query => "\x1b[?20;0$y".to_string(),
            },
            |override_mode| match override_mode {
                SetMode::DecSet => "\x1b[?20;1$y".to_string(),
                SetMode::DecRst => "\x1b[?20;2$y".to_string(),
                SetMode::DecQuery => "\x1b[?20;0$y".to_string(),
            },
        )
    }
}

impl Lnm {
    #[must_use]
    pub const fn new(mode: &SetMode) -> Self {
        match mode {
            SetMode::DecSet => Self::NewLine,
            SetMode::DecRst => Self::LineFeed,
            SetMode::DecQuery => Self::Query,
        }
    }
}

impl fmt::Display for Lnm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NewLine => write!(f, "New Line Mode (LNM)"),
            Self::LineFeed => write!(f, "Line Feed Mode (LNM)"),
            Self::Query => write!(f, "Query Line Mode (LNM)"),
        }
    }
}
