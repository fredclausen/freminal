// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::{
    io::{Read, Write},
    sync::{Arc, Mutex},
};

use anyhow::Result;
use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize};

use crate::Args;
use crossbeam_channel::{Receiver, Sender};

use super::{FreminalTermInputOutput, TermIoErr, TerminalRead};
use easy_cast::ConvApprox;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct TerminalSize {
    pub rows: usize,
    pub cols: usize,
    pub pixel_width: usize,
    pub pixel_height: usize,
}

impl std::fmt::Display for TerminalSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TerminalSize {{ rows: {rows}, cols: {cols}, pixel_width: {pixel_width}, pixel_height: {pixel_height} }}",
            rows = self.rows,
            cols = self.cols,
            pixel_width = self.pixel_width,
            pixel_height = self.pixel_height
        )
    }
}

impl From<TerminalSize> for PtySize {
    fn from(size: TerminalSize) -> Self {
        Self {
            rows: u16::conv_approx(size.rows),
            cols: u16::conv_approx(size.cols),
            pixel_width: u16::conv_approx(size.pixel_width),
            pixel_height: u16::conv_approx(size.pixel_height),
        }
    }
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self {
            rows: 38,
            cols: 112,
            pixel_width: 7,
            pixel_height: 15,
        }
    }
}

impl TerminalSize {
    #[must_use]
    pub const fn get_rows(&self) -> usize {
        self.rows
    }

    #[must_use]
    pub const fn get_cols(&self) -> usize {
        self.cols
    }
}

#[derive(Debug)]
pub enum TerminalWriteCommand {
    Write(Vec<u8>),
    Resize(TerminalSize),
}

pub struct FreminalPtyInputOutput {
    pair: Arc<Mutex<PtyPair>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    terminal_size: Arc<Mutex<TerminalSize>>,
}

impl FreminalPtyInputOutput {
    pub fn new(args: &Args) -> Result<Self> {
        //let terminfo_dir = extract_terminfo().map_err(CreatePtyIoErrorKind::ExtractTerminfo)?;
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            // Not all systems support pixel_width, pixel_height,
            // but it is good practice to set it to something
            // that matches the size of the selected font.  That
            // is more complex than can be shown here in this
            // brief example though!
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let cmd = args
            .shell
            .as_ref()
            .map_or_else(CommandBuilder::new_default_prog, CommandBuilder::new);

        pair.slave.spawn_command(cmd)?;
        let writer = pair.master.take_writer()?;

        Ok(Self {
            pair: Arc::new(Mutex::new(pair)),
            writer: Arc::new(Mutex::new(writer)),
            terminal_size: Arc::new(Mutex::new(TerminalSize::default())),
        })
    }

    pub fn get_reader(&self) -> Box<dyn Read + Send> {
        self.pair.lock().unwrap().master.try_clone_reader().unwrap()
    }

    pub fn pty_handler(
        &mut self,
        channel: &Receiver<TerminalWriteCommand>,
        send_channel: &Sender<TerminalRead>,
    ) {
        info!("Starting pty loop");

        let mut buf = [0u8; 4096];
        let mut reader = self.get_reader();

        loop {
            if let Ok(data) = channel.try_recv() {
                match data {
                    TerminalWriteCommand::Resize(size) => {
                        info!("pty_handler Resizing terminal to: {size}");
                        if let Err(e) = self.set_win_size(size) {
                            error!("Failed to set win size: {e}");
                        }
                    }
                    TerminalWriteCommand::Write(data) => {
                        let mut writer = self.writer.lock().unwrap();
                        match writer.write_all(&data) {
                            Ok(()) => {}
                            Err(e) => {
                                error!("Failed to write data to pty: {e}");
                            }
                        }
                    }
                }
            }

            if let Ok(read) = reader.read(&mut buf) {
                match read {
                    0 => {
                        continue;
                    }
                    read => match send_channel.send(TerminalRead { buf, read }) {
                        Ok(()) => {}
                        Err(e) => {
                            error!("Failed to send read data to channel: {e}");
                        }
                    },
                }
            }
        }
    }
}

impl FreminalTermInputOutput for FreminalPtyInputOutput {
    fn set_win_size(&mut self, terminal_size: TerminalSize) -> Result<(), TermIoErr> {
        debug!("PTY setting win size to: {terminal_size}");
        let old_size = self.terminal_size.lock().unwrap().clone();
        if old_size == terminal_size {
            return Ok(());
        }

        self.pair
            .lock()
            .unwrap()
            .master
            .resize(terminal_size.clone().into())?;
        self.terminal_size = Arc::new(Mutex::new(terminal_size));

        Ok(())
    }
}
