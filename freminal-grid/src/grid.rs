// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::line::{Cell, Line};

pub struct Grid<T> {
    inner: Vec<Line<T>>,
    max_height: usize,
}

impl<T: Cell> Grid<T> {
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            inner: (0..height).map(|_| Line::new(width)).collect(),
            max_height: height,
        }
    }
}
