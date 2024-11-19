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

use std::{
    process,
    sync::{Arc, Mutex},
};
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
            let terminal = Arc::new(Mutex::new(terminal));
            let terminal_clone = Arc::clone(&terminal);

            std::thread::spawn(move || loop {
                if let Ok(read) = rx.recv() {
                    let incoming = &read.buf[0..read.read_amount];
                    match &mut terminal.clone().lock() {
                        Ok(terminal) => {
                            terminal.internal.handle_incoming_data(incoming);
                        }
                        Err(e) => {
                            error!("Failed to lock terminal: {}", e);
                            std::process::exit(1);
                        }
                    }
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
