// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::fmt;

use crate::ansi_components::mode::SetMode;

use super::ReportMode;

/// Autowrap Mode (DECAWM) ?7
#[derive(Eq, PartialEq, Debug, Default, Clone)]
pub enum Decawm {
    /// Normal (Reset) Mode
    /// Disables autowrap mode.
    NoAutoWrap,
    /// Alternate (Set) Mode
    /// Enables autowrap mode
    #[default]
    AutoWrap,
    Query,
}

impl Decawm {
    #[must_use]
    pub const fn new(mode: &SetMode) -> Self {
        match mode {
            SetMode::DecSet => Self::AutoWrap,
            SetMode::DecRst => Self::NoAutoWrap,
            SetMode::DecQuery => Self::Query,
        }
    }
}

impl ReportMode for Decawm {
    fn report(&self, override_mode: Option<SetMode>) -> String {
        override_mode.map_or_else(
            || match self {
                Self::NoAutoWrap => "\x1b[?7;2$y".to_string(),
                Self::AutoWrap => "\x1b[?7;1$y".to_string(),
                Self::Query => "\x1b[?7;0$y".to_string(),
            },
            |override_mode| match override_mode {
                SetMode::DecSet => "\x1b[?7;1$y".to_string(),
                SetMode::DecRst => "\x1b[?7;2$y".to_string(),
                SetMode::DecQuery => "\x1b[?7;0$y".to_string(),
            },
        )
    }
}

impl fmt::Display for Decawm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAutoWrap => write!(f, "Autowrap Mode (DECAWM) Disabled"),
            Self::AutoWrap => write!(f, "Autowrap Mode (DECAWM) Enabled"),
            Self::Query => write!(f, "Autowrap Mode (DECAWM) Query"),
        }
    }
}
