// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

pub mod pty;
use anyhow::Result;
pub use pty::FreminalPtyInputOutput;

#[derive(Debug)]
pub struct TerminalRead {
    pub buf: [u8; 4096],
    pub read: usize,
}

impl TerminalRead {
    pub fn get_buffer(&self) -> &[u8] {
        &self.buf[..self.read]
    }
}

pub trait FreminalTermInputOutput {
    //fn read(&mut self);
    // fn write(&mut self, buf: &[u8]) -> Result<usize, TermIoErr>;
    fn set_win_size(&mut self, terminal_size: pty::TerminalSize) -> Result<()>;
}
