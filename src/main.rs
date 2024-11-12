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

use std::process;
// use smol_macros::main;
use terminal_emulator::TerminalEmulator;
use tracing::Level;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub mod args;
pub mod gui;
pub mod terminal_emulator;

use args::Args;

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

    let args = Args::parse(std::env::args()).unwrap_or_else(|_| {
        process::exit(1);
    });

    let res = match TerminalEmulator::new(&args) {
        Ok((terminal, rx)) => {
            let internal = terminal.internal.clone();

            let _ = std::thread::spawn(move || loop {
                if let Ok(read) = rx.recv() {
                    let incoming = &read.buf[0..read.read_amount];
                    internal.lock().unwrap().handle_incoming_data(incoming);
                }
            });

            gui::run(terminal)
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
