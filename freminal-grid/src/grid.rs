// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::{
    cursor::Cursor,
    line::{Cell, Line},
};

pub struct Grid<T, U> {
    inner: Vec<Line<T>>,
    max_height: usize,
    cursor: U,
}

impl<T: Cell, U: Cursor> Grid<T> {
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            inner: (0..height).map(|_| Line::new(width)).collect(),
            max_height: height,
        }
    }
}
