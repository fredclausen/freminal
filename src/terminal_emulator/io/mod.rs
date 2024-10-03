// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

pub mod pty;
pub use pty::FreminalPtyInputOutput;

pub type TermIoErr = Box<dyn std::error::Error>;

pub enum ReadResponse {
    Success(usize),
    Empty,
}

pub struct TerminalRead {
    pub buf: [u8; 4096],
    pub read: usize,
}

pub trait FreminalTermInputOutput {
    //fn read(&mut self);
    fn write(&mut self, buf: &[u8]) -> Result<usize, TermIoErr>;
    fn set_win_size(
        &mut self,
        width: usize,
        height: usize,
        font_width: usize,
        font_height: usize,
    ) -> Result<(), TermIoErr>;
}
