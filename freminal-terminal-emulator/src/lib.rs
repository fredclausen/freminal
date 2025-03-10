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
#![allow(clippy::range_plus_one)]
// #![warn(missing_docs)]

pub mod ansi;
pub mod ansi_components;
pub mod error;
pub mod format_tracker;
pub mod interface;
pub mod io;
// pub mod playback;
pub mod state;

#[macro_use]
extern crate tracing;
