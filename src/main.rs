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

struct Args {}

impl Args {
    fn parse<It: Iterator<Item = String>>(mut it: It) -> Self {
        let program_name = it.next();

        for arg in it {
            match arg.as_str() {
                _ => {
                    println!("Invalid argument {arg}");
                    Self::help(program_name.as_deref())
                }
            }
        }

        Self {}
    }

    fn help(program_name: Option<&str>) -> ! {
        let program_name = program_name.unwrap_or("freminal");
        println!(
            "\
                 Usage:\n\
                 {program_name} [ARGS]\n\
                 \n\
                 Args:\n\
                 --recording-path: Optional, where to output recordings to
                 --replay: Replay a recording
                 "
        );
        std::process::exit(1);
    }
}

fn main() {
    log::init();
    // TODO: we have no args. Either pull this out fully or add some args
    let _args = Args::parse(std::env::args());
    let res = match TerminalEmulator::new() {
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
