// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::fmt;

use crate::ansi_components::mode::SetMode;

/// Normal Mouse Tracking (`XT_MSE_X11`)
#[derive(Debug, Eq, PartialEq, Default)]
pub enum XtMseX11 {
    #[default]
    /// Normal mouse tracking is disabled
    Disabled,
    /// Normal mouse tracking is enabled
    Enabled,
}

impl XtMseX11 {
    #[must_use]
    pub const fn new(mode: &SetMode) -> Self {
        match mode {
            SetMode::DecSet => Self::Enabled,
            SetMode::DecRst => Self::Disabled,
        }
    }
}

impl fmt::Display for XtMseX11 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => f.write_str("Normal Mouse Tracking (XT_MSE_X11) Disabled"),
            Self::Enabled => f.write_str("Normal Mouse Tracking (XT_MSE_X11) Enabled"),
        }
    }
}
