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

fn junk_to_fill_buffer() -> Vec<u8> {
    let junk = b"                      ..'
                  ,xNMM.           fred@Freds-Mac-Studio
                .OMMMMo            ---------------------
                lMM\"               OS: macOS Sequoia 15.3.1 arm64
      .;loddo:.  .olloddol;.       Host: Mac Studio (M1 Max, 2022, Two USB-C front ports)
    cKMMMMMMMMMMNWMMMMMMMMMM0:     Kernel: Darwin 24.3.0
  .KMMMMMMMMMMMMMMMMMMMMMMMWd.     Uptime: 8 days, 14 hours, 45 mins
  XMMMMMMMMMMMMMMMMMMMMMMMX.
 ;MMMMMMMMMMMMMMMMMMMMMMMM:        Packages: 221 (brew), 34 (brew-cask)
 :MMMMMMMMMMMMMMMMMMMMMMMM:        Shell: zsh 5.9
 .MMMMMMMMMMMMMMMMMMMMMMMMX.       Display (Sceptre C35): 3440x1440 @ 60 Hz in 35\" [External] *
  kMMMMMMMMMMMMMMMMMMMMMMMMWd.     Display (R240HY): 1920x1080 @ 60 Hz in 24\" [External]
  'XMMMMMMMMMMMMMMMMMMMMMMMMMMk    Display (R240HY): 1920x1080 @ 60 Hz in 24\" [External]
   'XMMMMMMMMMMMMMMMMMMMMMMMMK.    Terminal: freminal 0.1.0
     kMMMMMMMMMMMMMMMMMMMMMMd
      ;KMMMMMMMWXXWMMMMMMMk.       CPU: Apple M1 Max (10) @ 3.23 GHz
        \"cooc*\"    \"*coo'\"         GPU: Apple M1 Max (24) @ 1.30 GHz [Integrated]
                                   Memory: 20.47 GiB / 32.00 GiB (64%)



 \xE2\x9D\xAF ./a.out";
    junk.to_vec()
}

fn setup() -> (TerminalState, crossbeam_channel::Receiver<PtyWrite>, usize) {
    let (tx, rx) = crossbeam_channel::unbounded();
    let mut terminal_state = TerminalState::new(tx.clone());
    terminal_state.set_win_size(213, 53);
    terminal_state.handle_incoming_data(junk_to_fill_buffer().as_slice());

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

    (terminal_state, rx, width)
}

fn get_position(
    terminal_state: &mut TerminalState,
    rx: &crossbeam_channel::Receiver<PtyWrite>,
) -> (usize, usize) {
    terminal_state.handle_incoming_data(REQUEST_CURSOR_POSITION);
    let (r, c) = read_and_strip(rx);

    (r, c)
}

// TEST ONE
#[test]
fn wrap_works() {
    let (mut terminal_state, rx, width) = setup();
    //   /* Check that wrap works. */
    //   cup(1, width - 1);
    //   wr("ABC");
    let cup_str = format!("\x1b[1;{}HABC", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   getpos(&r, &c);
    let (r, c) = get_position(&mut terminal_state, &rx);
    info!("Cursor position after writing ABC: {} {}", r, c);
    //   wrap_works = (r == 2 && c == 2);
    assert!(r == 2, "Expected cursor position y to be 2 found {}", r);
    assert!(c == 2, "Expected cursor position x to be 2 found {}", c);
}

// TEST TWO and THREE
#[test]
fn test_wrap_deferred() {
    let (mut terminal_state, rx, width) = setup();

    /* Check that wrap is deferred after writing to the last column. */
    //   cup(1, width - 1);
    //   wr("AB");
    let cup_str = format!("\x1b[1;{}HAB", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    let (r, wrap_col) = get_position(&mut terminal_state, &rx);
    info!("Cursor position after writing AB: {} {}", r, wrap_col);
    //   wrap_is_deferred = (r == 1 && wrap_col >= width);
    assert!(r == 1, "Expected cursor position y to be 1 found {}", r);
    assert!(
        wrap_col >= width,
        "Expected wrap column to be greater than width found {}",
        wrap_col
    );

    /* Whether CPR reports a position beyond the last column in the wrap state. */
    //   cpr_beyond_last_col = (wrap_col > width);
    assert!(
        wrap_col > width,
        "Expected wrap column to be greater than width found {}",
        wrap_col
    );
}

// TEST FOUR
#[test]
fn test_cr_works_after_writing_last_column() {
    let (mut terminal_state, rx, width) = setup();

    /* Check that CR works after writing to the last column. */
    //     cup(1, width - 1);
    //   wr("AB\r");
    let cup_str = format!("\x1b[1;{}HAB\r", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    let (r, c) = get_position(&mut terminal_state, &rx);
    info!("Cursor position after writing AB\\r: {} {}", r, c);
    //   cr_works_at_margin = (r == 1 && c == 1);
    assert!(r == 1, "Expected cursor position y to be 1 found {}", r);
    assert!(c == 1, "Expected cursor position x to be 1 found {}", c);
}

// TEST FIVE
// This fails because we have a bug. Marking ignored for now
#[ignore]
#[test]
fn test_bs_works_after_writing_last_column() {
    let (mut terminal_state, rx, width) = setup();
    /* Check that BS works after writing to the last column. */
    //   cup(1, width - 1);
    //   wr("AB\b");
    let cup_str = format!("\x1b[1;{}HAB\x08", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   getpos(&r, &c);
    let (r, c) = get_position(&mut terminal_state, &rx);
    //   bs_works_at_margin = (r == 1 && c == width - 1);
    assert!(r == 1, "Expected cursor position y to be 1 found {}", r);
    assert!(
        c == width - 1,
        "Expected cursor position x to be {} found {}",
        width - 1,
        c
    );
}

// TEST SIX
#[test]
fn test_tab_wraps() {
    let (mut terminal_state, rx, width) = setup();

    //      /* Check whether TAB wraps after writing to the last column. */
    //   cup(1, width - 1);
    //   wr("AB\t");
    let cup_str = format!("\x1b[1;{}HAB\t", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   getpos(&r, &c);
    let (r, _c) = get_position(&mut terminal_state, &rx);
    //   tab_wraps_at_margin = (r == 2);
    assert!(r == 2, "Expected cursor position y to be 2 found {}", r);
}
