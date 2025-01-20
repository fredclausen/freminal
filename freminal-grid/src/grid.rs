// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::{
    cursor::{Decorations, FreminalCursor, Position},
    line::{Cell, Line},
};

pub struct Grid<T, C, P, D> {
    inner: Vec<Line<T>>,
    max_height: usize,
    cursor: C,
    _marker: std::marker::PhantomData<P>,
    _marker2: std::marker::PhantomData<D>,
}

impl<T: Cell, C, P: Position, D: Decorations> Grid<T, C, P, D>
where
    C: FreminalCursor<P, D>,
{
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            inner: (0..height).map(|_| Line::new(width)).collect(),
            max_height: height,
            cursor: C::default(),
            _marker: std::marker::PhantomData,
            _marker2: std::marker::PhantomData,
        }
    }
}
