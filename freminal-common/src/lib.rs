// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![deny(
    clippy::pedantic,
    //clippy::cargo,
    clippy::nursery,
    clippy::style,
    clippy::correctness,
    clippy::all,
    clippy::unwrap_used,
    clippy::expect_used
)]
// #![warn(missing_docs)]

pub mod args;
pub mod buffer_states;
pub mod colors;
pub mod config;
pub mod cursor;
pub mod scroll;
pub mod terminal_size;
pub mod terminfo;
pub mod window_manipulation;

#[macro_use]
extern crate tracing;
