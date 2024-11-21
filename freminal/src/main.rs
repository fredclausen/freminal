// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![deny(
    clippy::pedantic,
    //clippy::cargo,
    clippy::nursery,
    clippy::style,
    clippy::correctness,
    clippy::all
)]
// #![warn(missing_docs)]

#[macro_use]
extern crate tracing;

use parking_lot::FairMutex;
use std::{process, sync::Arc};
use terminal_emulator::interface::TerminalEmulator;
use tracing::Level;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub mod gui;

use freminal_common::args::Args;

fn main() {
    // use env for filtering
    // example
    // RUST_LOG=none,freminal=debug cargo run

    let args = Args::parse(std::env::args()).unwrap_or_else(|_| {
        process::exit(1);
    });

    let env_filter = if args.show_all_debug {
        EnvFilter::builder()
            .with_default_directive(Level::INFO.into())
            .from_env_lossy()
    } else {
        EnvFilter::builder()
            .with_default_directive(Level::INFO.into())
            .from_env_lossy()
            .add_directive("winit=off".parse().unwrap())
            .add_directive("wgpu=off".parse().unwrap())
            .add_directive("eframe=off".parse().unwrap())
            .add_directive("egui=off".parse().unwrap())
    };

    let subscriber = tracing_subscriber::registry().with(env_filter);
    let fmt_layer = fmt::layer()
        .with_line_number(true)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::ACTIVE)
        .compact();
    subscriber.with(fmt_layer).init();

    trace!("Starting freminal");

    let res = match TerminalEmulator::new(&args) {
        Ok((terminal, rx)) => {
            let terminal = Arc::new(FairMutex::new(terminal));
            let terminal_clone = Arc::clone(&terminal);
            let mut send_over: Vec<u8> = Vec::new();

            std::thread::spawn(move || loop {
                if let Ok(read) = rx.recv() {
                    // FIXME: This may or may not be actually smart.
                    // We're going to try to buffer reads and send them over
                    // This may end up slowing the UI down on slow PTYs. I don't know.

                    // we want to see if we can send multiple reads at once
                    // up to a maximum of 1000 bytes

                    // Case where there were previous reads we've buffered
                    // We want to send the data if we have no more reads or if we have more than 1000 bytes
                    if !send_over.is_empty()
                        && (rx.is_empty() || send_over.len() + read.read_amount > 1000)
                    {
                        debug!("Sending buffered read");
                        send_over.extend_from_slice(&read.buf[0..read.read_amount]);
                    } else if rx.is_empty() {
                        // We have no pending reads, so send it over
                        debug!("Sending read");
                        terminal
                            .lock()
                            .internal
                            .handle_incoming_data(&read.buf[0..read.read_amount]);
                        continue;
                    } else {
                        debug!("Buffering read");
                        // We have more reads, so buffer it
                        send_over.extend_from_slice(&read.buf[0..read.read_amount]);
                        continue;
                    }

                    terminal.lock().internal.handle_incoming_data(&send_over);
                    send_over.clear();
                }
            });

            gui::run(terminal_clone)
        }
        Err(e) => {
            error!("Failed to create terminal emulator: {}", e);
            return;
        }
    };

    if let Err(e) = res {
        error!("Failed to run terminal emulator: {}", e);
    }
}
