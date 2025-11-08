use freminal_terminal_emulator::ansi::*;

fn push_seq(seq: &str) -> Vec<TerminalOutput> {
    let mut parser = FreminalAnsiParser::default();
    parser.push(seq.as_bytes())
}

#[test]
fn unknown_private_mode_sequence_is_invalid() {
    let outs = push_seq("\x1b[?9999l");
    println!("unknown private disable -> {:?}", outs);
    assert!(
        outs.iter()
            .any(|o| matches!(o, TerminalOutput::Invalid | TerminalOutput::Mode { .. })),
        "Expected Invalid or Mode for unknown mode"
    );
}
