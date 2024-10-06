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

use terminal_emulator::TerminalEmulator;
use tracing::Level;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod error;
mod gui;
mod terminal_emulator;

pub struct Args {
    recording: Option<String>,
    shell: Option<String>,
    start_maximized: bool,
}

impl Args {
    fn parse<It: Iterator<Item = String>>(mut it: It) -> Self {
        trace!("Parsing args");

        let program_name = it.next();
        let mut recording_path = None;
        let mut shell = None;
        let mut start_maximized = false;

        while let Some(arg) = it.next() {
            if arg.as_str() == "--recording-path" {
                recording_path = it.next().map_or_else(
                    || {
                        println!("Missing argument for --recording-path");
                        Self::help(program_name.as_deref());
                    },
                    Some,
                );
            } else if arg.as_str() == "--shell" {
                shell = it.next().map_or_else(
                    || {
                        println!("Missing argument for --shell");
                        Self::help(program_name.as_deref());
                    },
                    Some,
                );
            } else if arg.as_str() == "--start-maximized" {
                start_maximized = true;
            } else if arg.as_str() == "--help" {
                Self::help(program_name.as_deref());
            } else {
                println!("Invalid argument {arg}");
                Self::help(program_name.as_deref())
            }
        }

        Self {
            recording: recording_path,
            shell,
            start_maximized,
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
                 --shell: Optional, the shell to use\n\
                 --start-maximized: Optional, start maximized\n\
                 --help: Optional, show this help message\n\
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

    info!("Starting freminal");

    // spawn a thread to

    let args = Args::parse(std::env::args());
    let res =
        match TerminalEmulator::<terminal_emulator::io::pty::FreminalPtyInputOutput>::new(&args) {
            Ok(v) => gui::run(v, args),
            Err(e) => {
                error!("Failed to create terminal emulator: {e}",);
                return;
            }
        };

    if let Err(e) = res {
        error!("Failed to run gui: {}", error::backtraced_err(&*e));
    }
}
