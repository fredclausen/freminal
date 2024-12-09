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
}

impl TryFrom<(usize, usize, usize)> for WindowManipulation {
    type Error = anyhow::Error;

    fn try_from((command, param_ps2, param_ps3): (usize, usize, usize)) -> Result<Self> {
        match (command, param_ps2, param_ps3) {
            (1, 0, 0) => Ok(Self::DeIconifyWindow),
            (2, 0, 0) => Ok(Self::MinimizeWindow),
            (3, x, y) => Ok(Self::MoveWindow(x, y)),
            (4, x, y) => Ok(Self::ResizeWindow(x, y)),
            _ => Err(anyhow::anyhow!("Invalid WindowManipulation")),
        }
    }
}