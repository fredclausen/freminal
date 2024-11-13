// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#[macro_use]
extern crate tracing;
pub mod args;
pub mod gui;

pub mod terminal_emulator {
    // expose pub items in module root
    pub mod ansi;
    pub mod ansi_components;
    pub mod error;
    pub mod format_tracker;
    pub mod interface;
    pub mod io;
    pub mod state;
}

pub use crate::terminal_emulator::io::FreminalPtyInputOutput;
