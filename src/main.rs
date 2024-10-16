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

#[macro_use]
extern crate tracing;

// use smol_macros::main;
use terminal_emulator::TerminalEmulator;
use tracing::Level;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod gui;
mod terminal_emulator;

struct Args {
    recording: Option<String>,
    shell: Option<String>,
}

impl Args {
    fn parse<It: Iterator<Item = String>>(mut it: It) -> Self {
        trace!("Parsing args");

        let program_name = it.next();
        let mut recording_path = None;
        let mut shell = None;

        while let Some(arg) = it.next() {
            match arg {
                arg if arg.as_str() == "--recording-path" => {
                    recording_path = it.next().map_or_else(
                        || {
                            println!("Missing argument for --recording-path");
                            Self::help(program_name.as_deref());
                        },
                        Some,
                    );
                }
                arg if arg.as_str() == "--shell" => {
                    shell = it.next().map_or_else(
                        || {
                            println!("Missing argument for --shell");
                            Self::help(program_name.as_deref());
                        },
                        Some,
                    );
                }
                _ => {
                    println!("Invalid argument {arg}");
                    Self::help(program_name.as_deref())
                }
            }
        }

        Self {
            recording: recording_path,
            shell,
        }
    }

    fn help(program_name: Option<&str>) -> ! {
        trace!("Showing help");

        let program_name = program_name.unwrap_or("freminal");
        println!(
            "\
                 Usage:\n\
                 {program_name} [ARGS]\n\
                 \n\
                 Args:\n\
                 --recording-path: Optional, where to output recordings to
                 "
        );
        std::process::exit(1);
    }
}

fn main() {
    // use env for filtering
    // example
    // RUST_LOG=none,spectre_config=debug cargo run

    let env_filter = EnvFilter::builder()
        .with_default_directive(Level::INFO.into())
        .from_env_lossy()
        .add_directive("winit=off".parse().unwrap())
        .add_directive("wgpu=off".parse().unwrap())
        .add_directive("eframe=off".parse().unwrap())
        .add_directive("egui=off".parse().unwrap());

    let subscriber = tracing_subscriber::registry().with(env_filter);
    let fmt_layer = fmt::layer()
        .with_line_number(true)
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::ACTIVE)
        .compact();
    subscriber.with(fmt_layer).init();

    trace!("Starting freminal");
    debug!("Testing");
    info!("Starting freminal");

    let args = Args::parse(std::env::args());
    let res = match TerminalEmulator::new(&args) {
        Ok(v) => gui::run(v),
        Err(e) => {
            error!("Failed to create terminal emulator: {}", e);
            return;
        }
    };

    if let Err(e) = res {
        error!("Failed to run terminal emulator: {}", e);
    }
}
