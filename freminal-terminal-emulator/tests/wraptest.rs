use freminal_terminal_emulator::{io::PtyWrite, state::internal::TerminalState};
use test_log::test;
use tracing::info;

const REQUEST_CURSOR_POSITION: &[u8] = b"\x1b[6n";
// tests derive from https://github.com/mattiase/wraptest/blob/master/wraptest.c

fn read_and_strip(rx: &crossbeam_channel::Receiver<PtyWrite>) -> (usize, usize) {
    let mut cursor_pos = vec![];
    while !rx.is_empty() {
        let read = rx.recv().unwrap();
        match read {
            PtyWrite::Write(value) => {
                cursor_pos.extend_from_slice(&value);
            }
            PtyWrite::Resize(_) => {
                panic!("Unexpected resize event");
            }
        }
    }
    // strip the \x1b from the start of the string
    let cursor_pos = String::from_utf8_lossy(&cursor_pos);
    let cursor_pos = cursor_pos
        .strip_prefix("\x1b[")
        .unwrap_or(&cursor_pos)
        .strip_suffix("R");

    // split the output by the semicolon
    let cursor_pos = cursor_pos.unwrap_or("").split(';').collect::<Vec<_>>();
    // parse the output as usize
    let cursor_pos = cursor_pos
        .iter()
        .map(|s| s.parse::<usize>().unwrap_or(0))
        .collect::<Vec<_>>();

    (cursor_pos[0], cursor_pos[1])
}

#[test]
fn test_wrap() {
    let (tx, rx) = crossbeam_channel::unbounded();
    let mut terminal_state = TerminalState::new(tx.clone());
    terminal_state.set_win_size(213, 53);
    info!(
        "Terminal width/height: {:?}",
        terminal_state
            .get_current_buffer()
            .terminal_buffer
            .get_win_size()
    );
    // TEST ONE
    //     decawm(1);
    let decawm = b"\x1b[?7h";
    terminal_state.handle_incoming_data(decawm);
    //   wr("\33[20l");		/* Turn off LNM (automatic newline on CR). */
    let turn_off_lnm = b"\x1b[20l";
    terminal_state.handle_incoming_data(turn_off_lnm);
    let clear_screen = b"\x1b[2J";
    //   wr("\33[2J");			/* Clear screen. */
    terminal_state.handle_incoming_data(clear_screen);
    //   cup(1, 999);
    let cup = b"\x1b[1;999H";
    terminal_state.handle_incoming_data(cup);
    //   getpos(&r, &width);
    terminal_state.handle_incoming_data(REQUEST_CURSOR_POSITION);
    let (r, width) = read_and_strip(&rx);
    info!("Cursor position: {} {}", r, width);
    //   /* Check that wrap works. */
    //   cup(1, width - 1);
    //   wr("ABC");
    let cup_str = format!("\x1b[1;{}HABC", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   getpos(&r, &c);
    terminal_state.handle_incoming_data(REQUEST_CURSOR_POSITION);
    let (r, c) = read_and_strip(&rx);
    info!("Cursor position after writing ABC: {} {}", r, c);
    //   wrap_works = (r == 2 && c == 2);
    assert!(r == 2, "Expected cursor position y to be 2 found {}", r);
    assert!(c == 2, "Expected cursor position x to be 2 found {}", c);
}
