// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use anyhow::Result;

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum WindowManipulation {
    DeIconifyWindow,
    MinimizeWindow,
    MoveWindow(usize, usize),
    ResizeWindow(usize, usize),
    RaiseWindowToTopOfStackingOrder,
    LowerWindowToBottomOfStackingOrder,
    RefreshWindow,
    ResizeWindowToLinesAndColumns(usize, usize),
    MaximizeWindow,
    RestoreNonMaximizedWindow,
    NotFullScreen,
    FullScreen,
    ToggleFullScreen,
}

impl TryFrom<(usize, usize, usize)> for WindowManipulation {
    type Error = anyhow::Error;

    fn try_from((command, param_ps2, param_ps3): (usize, usize, usize)) -> Result<Self> {
        match (command, param_ps2, param_ps3) {
            (1, 0, 0) => Ok(Self::DeIconifyWindow),
            (2, 0, 0) => Ok(Self::MinimizeWindow),
            (3, x, y) => Ok(Self::MoveWindow(x, y)),
            (4, x, y) => Ok(Self::ResizeWindow(x, y)),
            (5, 0, 0) => Ok(Self::RaiseWindowToTopOfStackingOrder),
            (6, 0, 0) => Ok(Self::LowerWindowToBottomOfStackingOrder),
            (7, 0, 0) => Ok(Self::RefreshWindow),
            (8, x, y) => Ok(Self::ResizeWindowToLinesAndColumns(x, y)),
            (9, 1, 0) => Ok(Self::MaximizeWindow),
            (9, 0, 0) => Ok(Self::RestoreNonMaximizedWindow),
            (10, 0, 0) => Ok(Self::NotFullScreen),
            (10, 1, 0) => Ok(Self::FullScreen),
            (10, 2, 0) => Ok(Self::ToggleFullScreen),
            _ => Err(anyhow::anyhow!("Invalid WindowManipulation")),
        }
    }
}
