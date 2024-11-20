// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::fmt;

use crate::ansi_components::mode::SetMode;

/// Focus reporting mode (`XT_MSE_WIN`)
#[derive(Debug, Eq, PartialEq, Default)]
pub enum XtMseWin {
    #[default]
    /// Focus reporting is disabled
    Disabled,
    /// Focus reporting is enabled
    Enabled,
}

impl XtMseWin {
    #[must_use]
    pub const fn new(mode: &SetMode) -> Self {
        match mode {
            SetMode::Set => Self::Enabled,
            SetMode::Reset => Self::Disabled,
        }
    }
}

impl fmt::Display for XtMseWin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => f.write_str("Focus Reporting Mode (XT_MSE_WIN) Disabled"),
            Self::Enabled => f.write_str("Focus Reporting Mode (XT_MSE_WIN) Enabled"),
        }
    }
}
