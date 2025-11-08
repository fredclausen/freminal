// Copyright (C) 2024-2025 Fred Clausen
// MIT license.

use freminal_terminal_emulator::ansi::*;

fn push_seq(seq: &str) -> Vec<TerminalOutput> {
    let mut parser = FreminalAnsiParser::default();
    parser.push(seq.as_bytes())
}

#[test]
fn sgr_reset_attributes() {
    let outs = push_seq("\x1b[0m\x1b[22m\x1b[23m\x1b[24m\x1b[27m\x1b[28m\x1b[29m");
    println!("SGR resets {:?}", outs);
    assert!(outs.iter().all(|o| matches!(o, TerminalOutput::Sgr { .. })));
}

#[test]
fn sgr_combined_truecolor_sequence() {
    let seq = "\x1b[1;38;2;255;0;128;48;2;0;64;255m";
    let outs = push_seq(seq);
    println!("combined truecolor -> {:?}", outs);
    assert!(outs.iter().any(|o| matches!(o, TerminalOutput::Sgr { .. })));
}

#[test]
fn sgr_partial_truecolor_graceful() {
    let seq = "\x1b[38;2;255;0m";
    let outs = push_seq(seq);
    println!("partial truecolor -> {:?}", outs);
    assert!(
        outs.is_empty()
            || outs.iter().any(|o| matches!(o, TerminalOutput::Invalid))
            || outs.iter().any(|o| matches!(o, TerminalOutput::Sgr { .. })),
        "expected graceful or tolerant handling, got {:?}",
        outs
    );
}
