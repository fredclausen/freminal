// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::io::{Read, Write};

use anyhow::Result;
use portable_pty::{native_pty_system, Child, CommandBuilder, PtyPair, PtySize, PtySystem};

use crate::Args;
use crossbeam_channel::Sender;
use tempfile::TempDir;

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

pub struct FreminalPtyInputOutput {
    _pty_system: Box<dyn PtySystem>,
    pair: PtyPair,
    writer: Box<dyn Write + Send>,
    _child: Box<dyn Child + Send + Sync>,
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
            _pty_system: pty_system,
            pair,
            writer,
            _child: child,
        })
    }

    pub fn get_reader(&self) -> Box<dyn Read + Send> {
        self.pair.master.try_clone_reader().unwrap()
    }
}

pub fn read(mut reader: Box<dyn Read>, channel: &Sender<TerminalRead>) {
    let mut buf = [0u8; 4096];
    while let Ok(read) = reader.read(&mut buf) {
        match read {
            0 => {
                continue;
            }
            read => match channel.send(TerminalRead { buf, read }) {
                Ok(()) => {}
                Err(e) => {
                    error!("Failed to send read data to channel: {e}");
                }
            },
        }
    }
}

impl FreminalTermInputOutput for FreminalPtyInputOutput {
    fn write(&mut self, buf: &[u8]) -> Result<usize, TermIoErr> {
        let output_string = std::str::from_utf8(buf)?;
        let output_length = output_string.len();

        self.writer.write_all(buf)?;

        Ok(output_length)
    }

    fn set_win_size(
        &mut self,
        width: usize,
        height: usize,
        font_width: usize,
        font_height: usize,
    ) -> Result<(), TermIoErr> {
        let new_pty_pair = PtySize {
            rows: u16::conv_approx(height),
            cols: u16::conv_approx(width),
            pixel_width: u16::conv_approx(font_width),
            pixel_height: u16::conv_approx(font_height),
        };

        self.pair.master.resize(new_pty_pair)?;

        Ok(())
    }
}
