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

use terminal_emulator::TerminalEmulator;

#[macro_use]
mod log;
mod error;
mod gui;
mod terminal_emulator;

struct Args {
    recording: Option<String>,
}

impl Args {
    fn parse<It: Iterator<Item = String>>(mut it: It) -> Self {
        trace!("Parsing args");

        let program_name = it.next();
        let mut recording_path = None;

        while let Some(arg) = it.next() {
            if arg.as_str() == "--recording-path" {
                recording_path = it.next().map_or_else(
                    || {
                        println!("Missing argument for --recording-path");
                        Self::help(program_name.as_deref());
                    },
                    Some,
                );
            } else {
                println!("Invalid argument {arg}");
                Self::help(program_name.as_deref())
            }
        }

        Self {
            recording: recording_path,
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
    log::init();

    trace!("Starting freminal");
    debug!("Testing");
    info!("Starting freminal");

    let args = Args::parse(std::env::args());
    let res = match TerminalEmulator::new(&args.recording) {
        Ok(v) => gui::run(v),
        Err(e) => {
            error!(
                "Failed to create terminal emulator: {}",
                error::backtraced_err(&e)
            );
            return;
        }
    };

    if let Err(e) = res {
        error!("Failed to run gui: {}", error::backtraced_err(&*e));
    }
}
