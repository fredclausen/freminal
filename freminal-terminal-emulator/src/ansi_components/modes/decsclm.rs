// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::fmt;

use crate::ansi_components::mode::SetMode;

use super::ReportMode;

/// Set number of columns (DECSCLM) ?4
#[derive(Debug, Eq, PartialEq, Default, Clone)]
pub enum Decsclm {
    SmoothScroll,
    #[default]
    FastScroll,
    Query,
}

impl ReportMode for Decsclm {
    fn report(&self, _override_mode: Option<SetMode>) -> String {
        "\x1b[?4;0$y".to_string()
    }
}

impl Decsclm {
    #[must_use]
    pub const fn new(mode: &SetMode) -> Self {
        match mode {
            SetMode::DecSet => Self::SmoothScroll,
            SetMode::DecRst => Self::FastScroll,
            SetMode::DecQuery => Self::Query,
        }
    }
}

impl fmt::Display for Decsclm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SmoothScroll => write!(f, "Smooth Scroll (DECSCLM)"),
            Self::FastScroll => write!(f, "Fast Scroll (DECSCLM)"),
            Self::Query => write!(f, "Query Scroll (DECSCLM)"),
        }
    }
}
