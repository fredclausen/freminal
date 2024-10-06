// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::{
    io::{Read, Write},
    rc::Rc,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use portable_pty::{native_pty_system, Child, CommandBuilder, PtyPair, PtySize, PtySystem};

use crate::{terminal_emulator, Args};
use crossbeam_channel::{Receiver, Sender};

use super::{FreminalTermInputOutput, TermIoErr, TerminalRead};
use easy_cast::ConvApprox;

/// Spawn a shell in a child process and return the file descriptor used for I/O
// fn spawn_shell(terminfo_dir: &Path) -> Result<OwnedFd, SpawnShellError> {
//     unsafe {
//         let res = nix::pty::forkpty(None, None).map_err(SpawnShellErrorKind::Fork)?;
//         match res {
//             ForkptyResult::Parent {
//                 child: _child,
//                 master,
//             } => Ok(master),
//             ForkptyResult::Child => {
//                 // FIXME: grab the shell from $SHELL
//                 let shell_name = c"zsh";
//                 let args: &[&[u8]] = &[b"bash\0", b"--noprofile\0", b"--norc\0"];

//                 let args: Vec<&'static CStr> = args
//                     .iter()
//                     .map(|v| {
//                         CStr::from_bytes_with_nul(v).expect("Should always have null terminator")
//                     })
//                     .collect::<Vec<_>>();

//                 // FIXME: Temporary workaround to avoid rendering issues
//                 std::env::remove_var("PROMPT_COMMAND");
//                 std::env::set_var("TERMINFO", terminfo_dir);
//                 std::env::set_var("TERM", "freminal");
//                 std::env::set_var("PS1", "$ ");
//                 nix::unistd::execvp(shell_name, &args).map_err(SpawnShellErrorKind::Exec)?;
//                 // Should never run
//                 std::process::exit(1);
//             }
//         }
//     }
// }

// #[derive(Error, Debug)]
// enum SetNonblockError {
//     #[error("failed to get current fcntl args")]
//     GetCurrent(#[source] Errno),
//     #[error("failed to parse retrieved oflags")]
//     ParseFlags,
//     #[error("failed to set new fcntl args")]
//     SetNew(#[source] Errno),
// }

// fn set_nonblock(fd: &OwnedFd) -> Result<(), SetNonblockError> {
//     let flags = nix::fcntl::fcntl(fd.as_raw_fd(), nix::fcntl::FcntlArg::F_GETFL)
//         .map_err(SetNonblockError::GetCurrent)?;
//     let mut flags = nix::fcntl::OFlag::from_bits(flags & nix::fcntl::OFlag::O_ACCMODE.bits())
//         .ok_or(SetNonblockError::ParseFlags)?;
//     flags.set(nix::fcntl::OFlag::O_NONBLOCK, true);

//     nix::fcntl::fcntl(fd.as_raw_fd(), nix::fcntl::FcntlArg::F_SETFL(flags))
//         .map_err(SetNonblockError::SetNew)?;
//     Ok(())
// }
// #[derive(Debug, Error)]
// enum SetWindowSizeErrorKind {
//     #[error("height too large")]
//     HeightTooLarge(#[source] std::num::TryFromIntError),
//     #[error("width too large")]
//     WidthTooLarge(#[source] std::num::TryFromIntError),
//     #[error("failed to execute ioctl")]
//     IoctlFailed(#[source] Errno),
// }

// #[derive(Debug, Error)]
// enum PtyIoErrKind {
//     #[error("failed to set win size")]
//     SetWinSize(#[from] SetWindowSizeErrorKind),
//     #[error("failed to read from file descriptor")]
//     Read(#[source] Errno),
//     #[error("failed to write to file descriptor")]
//     Write(#[source] Errno),
// }

// #[derive(Debug, Error)]
// #[error(transparent)]
// pub struct FreminalPtyIoErr(#[from] PtyIoErrKind);

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct TerminalSize {
    pub rows: usize,
    pub cols: usize,
    pub pixel_width: usize,
    pub pixel_height: usize,
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
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}

#[derive(Debug)]
pub enum TerminalWriteCommand {
    Write(Vec<u8>),
    Resize(TerminalSize),
}

pub struct FreminalPtyInputOutput {
    _pty_system: Rc<Mutex<Box<dyn PtySystem>>>,
    pair: Arc<Mutex<PtyPair>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    _child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
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

        let child = pair.slave.spawn_command(cmd)?;
        let writer = pair.master.take_writer()?;

        Ok(Self {
            _pty_system: Rc::new(Mutex::new(pty_system)),
            pair: Arc::new(Mutex::new(pair)),
            writer: Arc::new(Mutex::new(writer)),
            terminal_size: Arc::new(Mutex::new(TerminalSize::default())),
            _child: Arc::new(Mutex::new(child)),
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

unsafe impl Send for FreminalPtyInputOutput {
    // This is safe because PtyPair is Send
}
