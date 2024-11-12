// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::io::Write;

use super::{FreminalTermInputOutput, PtyRead, PtyWrite};
use anyhow::Result;
use crossbeam_channel::{Receiver, Sender};
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};

pub struct FreminalPtyInputOutput;

pub fn run_terminal(
    write_rx: Receiver<PtyWrite>,
    send_tx: Sender<PtyRead>,
    recording_path: Option<String>,
    shell: Option<String>,
) -> Result<()> {
    let pty_system = NativePtySystem::default();

    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();

    let cmd = shell.map_or_else(CommandBuilder::new_default_prog, CommandBuilder::new);
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
                continue;
            }
            let data = buf[..amount_read].to_vec();

            // if recording is some, write to the file

            if let Some(file) = &mut recording {
                for byte in &data {
                    file.write_all(format!("{byte},").as_bytes()).unwrap();
                }
            }

            send_tx
                .send(PtyRead {
                    buf: data,
                    read_amount: amount_read,
                })
                .unwrap();
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

            let mut writer = pair.master.take_writer().unwrap();

            while let Ok(stuff_to_write) = write_rx.recv() {
                match stuff_to_write {
                    PtyWrite::Write(data) => {
                        writer.write_all(&data).unwrap();
                    }
                    PtyWrite::Resize(size) => {
                        let size: PtySize = match PtySize::try_from(size) {
                            Ok(size) => size,
                            Err(e) => {
                                error!("failed to convert size {e}");
                                continue;
                            }
                        };

                        debug!("resizing pty to {size:?}");

                        pair.master.resize(size).unwrap();
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
        run_terminal(write_rx, send_tx, recording, shell)?;
        Ok(Self)
    }
}
