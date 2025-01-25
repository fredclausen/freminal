// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::{io::Write, path::Path};

use super::{FreminalTermInputOutput, PtyRead, PtyWrite};
use anyhow::Result;
use crossbeam_channel::{Receiver, Sender};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use tempfile::TempDir;
use thiserror::Error;

pub struct FreminalPtyInputOutput {
    _termcaps: TempDir,
}

const TERMINFO: &[u8] = include_bytes!(std::concat!(std::env!("OUT_DIR"), "/terminfo.tar"));

fn extract_terminfo() -> Result<TempDir, ExtractTerminfoError> {
    let mut terminfo_tarball = tar::Archive::new(TERMINFO);
    let temp_dir = TempDir::new().map_err(ExtractTerminfoError::CreateTempDir)?;
    terminfo_tarball
        .unpack(temp_dir.path())
        .map_err(ExtractTerminfoError::Extraction)?;

    Ok(temp_dir)
}

#[derive(Error, Debug)]
enum ExtractTerminfoError {
    #[error("failed to extract")]
    Extraction(#[source] std::io::Error),
    #[error("failed to create temp dir")]
    CreateTempDir(#[source] std::io::Error),
}

pub fn run_terminal(
    write_rx: Receiver<PtyWrite>,
    send_tx: Sender<PtyRead>,
    recording_path: Option<String>,
    shell: Option<String>,
    termcaps: &Path,
) -> Result<()> {
    let pty_system = NativePtySystem::default();

    let pair = match pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    }) {
        Ok(pair) => pair,
        Err(e) => {
            error!("Failed to open pty: {e}");
            std::process::exit(1);
        }
    };

    let mut cmd = shell.map_or_else(CommandBuilder::new_default_prog, CommandBuilder::new);

    cmd.env("TERMINFO", termcaps);
    cmd.env("TERM", "freminal");
    let _child = pair.slave.spawn_command(cmd)?;

    // Release any handles owned by the slave: we don't need it now
    // that we've spawned the child.
    drop(pair.slave);

    // Read the output in another thread.
    // This is important because it is easy to encounter a situation
    // where read/write buffers fill and block either your process
    // or the spawned process.
    let mut reader = pair.master.try_clone_reader()?;

    std::thread::spawn(move || {
        let buf = &mut [0u8; 4096];
        let mut recording = None;
        // if recording path is some, open a file for writing
        if let Some(path) = &recording_path {
            recording = match std::fs::File::create(path) {
                Ok(file) => Some(file),
                Err(e) => {
                    error!("Failed to create recording file: {e}");
                    None
                }
            }
        }

        // Consume the output from the child
        while let Ok(amount_read) = reader.read(buf) {
            if amount_read == 0 {
                // PTY closed, exit(0)
                std::process::exit(0);
            }
            let data = buf[..amount_read].to_vec();

            // if recording is some, write to the file

            if let Some(file) = &mut recording {
                for byte in &data {
                    if let Err(e) = file.write_all(format!("{byte},").as_bytes()) {
                        error!("Failed to write to recording file: {e}");
                        // exit
                        std::process::exit(1);
                    }
                }
            }

            if let Err(e) = send_tx.send(PtyRead {
                buf: data,
                read_amount: amount_read,
            }) {
                error!("Failed to send data to terminal: {e}");
                // exit
                std::process::exit(1);
            }
        }
    });

    {
        std::thread::spawn(move || {
            if cfg!(target_os = "macos") {
                // macOS quirk: the child and reader must be started and
                // allowed a brief grace period to run before we allow
                // the writer to drop. Otherwise, the data we send to
                // the kernel to trigger EOF is interleaved with the
                // data read by the reader! WTF!?
                // This appears to be a race condition for very short
                // lived processes on macOS.
                // I'd love to find a more deterministic solution to
                // this than sleeping.
                std::thread::sleep(std::time::Duration::from_millis(20));
            }

            let mut writer = match pair.master.take_writer() {
                Ok(writer) => writer,
                Err(e) => {
                    error!("Failed to take writer: {e}");
                    std::process::exit(1);
                }
            };

            while let Ok(stuff_to_write) = write_rx.recv() {
                match stuff_to_write {
                    PtyWrite::Write(data) => match writer.write_all(&data) {
                        Ok(()) => {}
                        Err(e) => {
                            error!("Failed to write to pty: {e}");
                            continue;
                        }
                    },
                    PtyWrite::Resize(size) => {
                        let size: PtySize = match PtySize::try_from(size) {
                            Ok(size) => size,
                            Err(e) => {
                                error!("failed to convert size {e}");
                                continue;
                            }
                        };

                        debug!("resizing pty to {size:?}");

                        match pair.master.resize(size) {
                            Ok(()) => {}
                            Err(e) => {
                                error!("Failed to resize pty: {e}");
                                continue;
                            }
                        }
                    }
                }
            }
        });
    }

    Ok(())
}

impl FreminalTermInputOutput for FreminalPtyInputOutput {}

impl FreminalPtyInputOutput {
    /// Create a new `FreminalPtyInputOutput` instance.
    ///
    /// # Errors
    /// Will return an error if the terminal cannot be created.
    pub fn new(
        write_rx: Receiver<PtyWrite>,
        send_tx: Sender<PtyRead>,
        recording: Option<String>,
        shell: Option<String>,
    ) -> Result<Self> {
        let termcaps = extract_terminfo().unwrap_or_else(|e| {
            error!("Failed to extract terminfo: {e}");
            std::process::exit(1);
        });

        run_terminal(write_rx, send_tx, recording, shell, termcaps.path())?;
        Ok(Self {
            _termcaps: termcaps,
        })
    }
}
