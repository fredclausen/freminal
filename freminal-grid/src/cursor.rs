// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

pub trait FreminalCursor<U, P>: Sized + Default + Clone + Eq + PartialEq {
    fn current_position(&self) -> P;
    fn current_decorations(&self) -> U;
}

pub trait Position {}
pub trait Decorations {}
