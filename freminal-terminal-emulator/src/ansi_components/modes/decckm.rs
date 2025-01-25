// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::fmt;

use crate::ansi_components::mode::SetMode;

use super::ReportMode;

/// Cursor Key Mode (DECCKM) ?1
#[derive(Eq, PartialEq, Debug, Default, Clone)]
pub enum Decckm {
    #[default]
    /// Normal (Reset) Mode
    /// Normal cursor keys in ANSI mode.
    Ansi,
    /// Alternate (Set) Mode
    /// Application cursor keys.
    Application,
    Query,
}

impl Decckm {
    #[must_use]
    pub const fn new(mode: &SetMode) -> Self {
        match mode {
            SetMode::DecSet => Self::Application,
            SetMode::DecRst => Self::Ansi,
            SetMode::DecQuery => Self::Query,
        }
    }
}

impl ReportMode for Decckm {
    fn report(&self, override_mode: Option<SetMode>) -> String {
        override_mode.map_or_else(
            || match self {
                Self::Ansi => "\x1b[?1;2$y".to_string(),
                Self::Application => "\x1b[?1;1$y".to_string(),
                Self::Query => "\x1b[?1;0$y".to_string(),
            },
            |override_mode| match override_mode {
                SetMode::DecSet => "\x1b[?1;1$y".to_string(),
                SetMode::DecRst => "\x1b[?1;2$y".to_string(),
                SetMode::DecQuery => "\x1b[?1;0$y".to_string(),
            },
        )
    }
}

impl fmt::Display for Decckm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ansi => write!(f, "Cursor Key Mode (DECCKM) ANSI"),
            Self::Application => write!(f, "Cursor Key Mode (DECCKM) Application"),
            Self::Query => write!(f, "Cursor Key Mode (DECCKM) Query"),
        }
    }
}
