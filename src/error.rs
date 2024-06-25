// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::{error::Error, fmt::Display};

pub struct FreminalErrorFormatted<'a>(&'a dyn Error);

impl Display for FreminalErrorFormatted<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let err = self.0;
        write!(f, "{err}")?;

        if err.source().is_none() {
            return Ok(());
        }

        write!(f, "\nCaused by:")?;

        let mut source = err.source();
        while let Some(err) = source {
            write!(f, "\n{err}")?;
            source = err.source();
        }

        Ok(())
    }
}

pub fn backtraced_err(err: &dyn Error) -> FreminalErrorFormatted<'_> {
    FreminalErrorFormatted(err)
}
