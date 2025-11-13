// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::fmt;

use crate::ansi_components::mode::SetMode;

use super::ReportMode;

// FIXME: We should handle timeouts here.
// The spec doesn't give a timeout, but gives guidance.
// https://gist.github.com/christianparpart/d8a62cc1ab659194337d73e399004036
// https://gitlab.com/gnachman/iterm2/-/wikis/synchronized-updates-spec

/// Synchronized Updates Mode ?2026
#[derive(Debug, Default, Eq, PartialEq, Clone)]
pub enum SynchronizedUpdates {
    #[default]
    /// Normal (Reset) Mode
    Draw,
    /// Alternate (Set) Mode
    DontDraw,
    Query,
}

impl SynchronizedUpdates {
    #[must_use]
    pub const fn new(mode: &SetMode) -> Self {
        match mode {
            SetMode::DecSet => Self::DontDraw,
            SetMode::DecRst => Self::Draw,
            SetMode::DecQuery => Self::Query,
        }
    }
}

impl ReportMode for SynchronizedUpdates {
    fn report(&self, override_mode: Option<SetMode>) -> String {
        override_mode.map_or_else(
            || match self {
                Self::Draw => String::from("\x1b[?2026;2$y"),
                Self::DontDraw => String::from("\x1b[?2026;1$y"),
                Self::Query => String::from("\x1b[?2026;0$y"),
            },
            |override_mode| match override_mode {
                SetMode::DecSet => String::from("\x1b[?2026;1$y"),
                SetMode::DecRst => String::from("\x1b[?2026;2$y"),
                SetMode::DecQuery => String::from("\x1b[?2026;0$y"),
            },
        )
    }
}

impl fmt::Display for SynchronizedUpdates {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Draw => write!(f, "Synchronized Updates Mode (DEC 2026) Draw"),
            Self::DontDraw => write!(f, "Synchronized Updates Mode (DEC 2026) Don't Draw"),
            Self::Query => write!(f, "Synchronized Updates Mode (DEC 2026) Query"),
        }
    }
}

/// Synchronized Updates Mode ?2027
#[derive(Debug, Default, Eq, PartialEq, Clone)]
pub enum SynchronizedUpdatesAlternative {
    #[default]
    /// Normal (Reset) Mode
    Draw,
    /// Alternate (Set) Mode
    DontDraw,
    Query,
}

impl From<SynchronizedUpdates> for SynchronizedUpdatesAlternative {
    fn from(value: SynchronizedUpdates) -> Self {
        match value {
            SynchronizedUpdates::Draw => Self::Draw,
            SynchronizedUpdates::DontDraw => Self::DontDraw,
            SynchronizedUpdates::Query => Self::Query,
        }
    }
}

impl SynchronizedUpdatesAlternative {
    #[must_use]
    pub const fn new(mode: &SetMode) -> Self {
        match mode {
            SetMode::DecSet => Self::DontDraw,
            SetMode::DecRst => Self::Draw,
            SetMode::DecQuery => Self::Query,
        }
    }
}

impl ReportMode for SynchronizedUpdatesAlternative {
    fn report(&self, override_mode: Option<SetMode>) -> String {
        override_mode.map_or_else(
            || match self {
                Self::Draw => String::from("\x1b[?2027;2$y"),
                Self::DontDraw => String::from("\x1b[?2027;1$y"),
                Self::Query => String::from("\x1b[?2027;0$y"),
            },
            |override_mode| match override_mode {
                SetMode::DecSet => String::from("\x1b[?2027;1$y"),
                SetMode::DecRst => String::from("\x1b[?2027;2$y"),
                SetMode::DecQuery => String::from("\x1b[?2027;0$y"),
            },
        )
    }
}

impl fmt::Display for SynchronizedUpdatesAlternative {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Draw => write!(f, "Synchronized Updates Mode (DEC 2026) Draw"),
            Self::DontDraw => write!(f, "Synchronized Updates Mode (DEC 2026) Don't Draw"),
            Self::Query => write!(f, "Synchronized Updates Mode (DEC 2026) Query"),
        }
    }
}
