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

use anyhow::Result;
use std::process;
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
    fn parse<It: Iterator<Item = String>>(mut it: It) -> Result<Self> {
        trace!("Parsing args");

        let program_name = it.next();
        let mut recording_path = None;
        let mut shell = None;
        let mut error = false;

        while let Some(arg) = it.next() {
            match arg {
                arg if arg.as_str() == "--recording-path" => {
                    recording_path = it.next().map_or_else(
                        || {
                            println!("Missing argument for --recording-path");
                            Self::help(program_name.as_deref());
                            error = true;
                            None
                        },
                        Some,
                    );
                }
                arg if arg.as_str() == "--shell" => {
                    shell = it.next().map_or_else(
                        || {
                            println!("Missing argument for --shell");
                            Self::help(program_name.as_deref());
                            error = true;
                            None
                        },
                        Some,
                    );
                }
                arg if arg.as_str() == "--help" => Self::help(program_name.as_deref()),
                _ => {
                    println!("Invalid argument {arg}");
                    Self::help(program_name.as_deref());
                    error = true;
                }
            }
        }

        if error {
            return Err(anyhow::anyhow!("Invalid arguments"));
        }

        Ok(Self {
            recording: recording_path,
            shell,
        })
    }

    fn help(program_name: Option<&str>) {
        trace!("Showing help");

        let program_name = program_name.unwrap_or("freminal");
        println!(
            "\
                 Usage:\n\
                 {program_name} [ARGS]\n\
                 \n\
                 Args:\n\
                    --recording-path: Optional, where to output recordings to\n--shell: Optional, shell to run\n--help: Show this help message\n\
                 "
        );
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

    let args = Args::parse(std::env::args()).unwrap_or_else(|_| {
        process::exit(1);
    });

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

// tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_parse() {
        let args = Args::parse(vec!["freminal".to_string()].into_iter()).unwrap();
        assert_eq!(args.recording, None);
        assert_eq!(args.shell, None);

        let args = Args::parse(
            vec![
                "freminal".to_string(),
                "--recording-path".to_string(),
                "test".to_string(),
            ]
            .into_iter(),
        )
        .unwrap();
        assert_eq!(args.recording, Some("test".to_string()));
        assert_eq!(args.shell, None);

        let args = Args::parse(
            vec![
                "freminal".to_string(),
                "--shell".to_string(),
                "test".to_string(),
            ]
            .into_iter(),
        )
        .unwrap();
        assert_eq!(args.recording, None);
        assert_eq!(args.shell, Some("test".to_string()));

        let args = Args::parse(
            vec![
                "freminal".to_string(),
                "--recording-path".to_string(),
                "test".to_string(),
                "--shell".to_string(),
                "test".to_string(),
            ]
            .into_iter(),
        )
        .unwrap();
        assert_eq!(args.recording, Some("test".to_string()));
        assert_eq!(args.shell, Some("test".to_string()));
    }

    #[test]
    fn test_invalid_arg() {
        let args = Args::parse(
            vec![
                "freminal".to_string(),
                "--recording-path".to_string(),
                "test".to_string(),
                "--invalid".to_string(),
            ]
            .into_iter(),
        );
        assert!(args.is_err());
    }
}
