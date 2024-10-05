// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

pub mod pty;
pub use pty::FreminalPtyInputOutput;

use crate::gui::terminal;

pub type TermIoErr = Box<dyn std::error::Error>;

pub struct TerminalRead {
    pub buf: [u8; 4096],
    pub read: usize,
}

pub trait FreminalTermInputOutput {
    //fn read(&mut self);
    // fn write(&mut self, buf: &[u8]) -> Result<usize, TermIoErr>;
    fn set_win_size(
        &mut self,
        terminal_size: pty::TerminalSize,
    ) -> Result<(), TermIoErr>;
}
