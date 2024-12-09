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
}

impl TryFrom<(usize, usize, usize)> for WindowManipulation {
    type Error = anyhow::Error;

    fn try_from((a, b, c): (usize, usize, usize)) -> Result<Self> {
        match (a, b, c) {
            (1, 0, 0) => Ok(Self::DeIconifyWindow),
            (2, 0, 0) => Ok(Self::MinimizeWindow),
            _ => Err(anyhow::anyhow!("Invalid WindowManipulation")),
        }
    }
}
