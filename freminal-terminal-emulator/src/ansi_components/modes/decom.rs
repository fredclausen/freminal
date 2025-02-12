// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::fmt;

use crate::ansi_components::mode::SetMode;

use super::ReportMode;

/// Show cursor (DECOM) ?6
#[derive(Debug, Eq, PartialEq, Default, Clone)]
pub enum Decom {
    #[default]
    NormalCursor,
    OriginMode,
    Query,
}

impl ReportMode for Decom {
    fn report(&self, override_mode: Option<SetMode>) -> String {
        override_mode.map_or_else(
            || match self {
                Self::NormalCursor => "\x1b[?6;2$y".to_string(),
                Self::OriginMode => "\x1b[?6;1$y".to_string(),
                Self::Query => "\x1b[?6;0$y".to_string(),
            },
            |override_mode| match override_mode {
                SetMode::DecSet => "\x1b[?6;1$y".to_string(),
                SetMode::DecRst => "\x1b[?6;2$y".to_string(),
                SetMode::DecQuery => "\x1b[?6;0$y".to_string(),
            },
        )
    }
}

impl Decom {
    #[must_use]
    pub const fn new(mode: &SetMode) -> Self {
        match mode {
            SetMode::DecSet => Self::OriginMode,
            SetMode::DecRst => Self::NormalCursor,
            SetMode::DecQuery => Self::Query,
        }
    }
}

impl fmt::Display for Decom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NormalCursor => write!(f, "Normal Cursor"),
            Self::OriginMode => write!(f, "Origin Mode"),
            Self::Query => write!(f, "Query"),
        }
    }
}
