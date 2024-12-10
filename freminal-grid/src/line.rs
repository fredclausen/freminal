// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

pub trait Cell: Sized {
    fn is_empty(&self) -> bool;
    fn reset(&mut self);
}

pub struct Line<T> {
    inner: Vec<T>,
    max_length: usize,
}

impl<T: Cell> Line<T> {
    pub fn new(length: usize) -> Self {
        Self {
            inner: Vec::with_capacity(length),
            max_length: length,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.inner.iter().all(|cell| cell.is_empty())
    }

    pub fn reset(&mut self) {
        for cell in &mut self.inner {
            cell.reset();
        }
    }
}
