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

use freminal_terminal_emulator::interface::TerminalEmulator;
use parking_lot::FairMutex;
use std::{process, sync::Arc};
use tracing::Level;
use tracing_subscriber::{
    fmt::{self, layer},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

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

    if args.write_logs_to_file {
        let std_out_layer = layer()
            .with_line_number(true)
            .with_span_events(fmt::format::FmtSpan::ACTIVE)
            .compact();

        let file_appender = tracing_appender::rolling::daily("./", "freminal.log");
        subscriber
            .with(layer().with_ansi(false).pretty().with_writer(file_appender))
            .with(std_out_layer)
            .init();
    } else {
        let std_out_layer = layer()
            .with_line_number(true)
            .with_span_events(fmt::format::FmtSpan::ACTIVE)
            .compact();

        subscriber.with(std_out_layer).init();
    }

    info!("Starting freminal");

    let res = match TerminalEmulator::new(&args) {
        Ok((terminal, rx)) => {
            let terminal = Arc::new(FairMutex::new(terminal));
            let terminal_clone = Arc::clone(&terminal);

            std::thread::spawn(move || loop {
                if let Ok(read) = rx.recv() {
                    terminal
                        .lock()
                        .internal
                        .handle_incoming_data(&read.buf[0..read.read_amount]);
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

    info!("Shutting down freminal");
}
