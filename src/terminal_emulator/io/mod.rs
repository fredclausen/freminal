// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

mod pty;
// pub use pty::{CreatePtyIoError, FreminalPtyInputOutput};

// pub type TermIoErr = Box<dyn std::error::Error>;

// pub enum ReadResponse {
//     Success(usize),
//     Empty,
// }

// pub trait FreminalTermInputOutput {
//     fn read(&mut self, buf: &mut [u8]) -> Result<ReadResponse, TermIoErr>;
//     fn write(&mut self, buf: &[u8]) -> Result<usize, TermIoErr>;
//     fn set_win_size(&mut self, width: usize, height: usize) -> Result<(), TermIoErr>;
// }
