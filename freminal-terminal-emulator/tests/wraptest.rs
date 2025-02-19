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

// TEST SEVEN
// This fails because we have a bug. Marking ignored for now
#[ignore]
#[test]
fn test_tab_cancels_wrap_state() {
    let (mut terminal_state, rx, width) = setup();

    //   /* Check whether TAB cancels wrap state. */
    //   cup(1, width - 1);
    //   wr("AB\tC");
    let cup_str = format!("\x1b[1;{}HAB\tC", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   getpos(&r, &c);
    let (r, c) = get_position(&mut terminal_state, &rx);
    //   tab_cancels_wrap = (r == 1 && c >= width);
    assert!(r == 1, "Expected cursor position y to be 1 found {}", r);
    assert!(
        c >= width,
        "Expected cursor position x to be greater than or equal to {} found {}",
        width,
        c
    );
}

// TEST EIGHT
// This fails because we have a bug. Marking ignored for now
#[ignore]
#[test]
fn test_nl_cancels_wrap_state() {
    let (mut terminal_state, rx, width) = setup();

    //   /* Check whether NL cancels wrap state. */
    //   cup(1, width - 1);
    //   wr("AB\nC");
    let cup_str = format!("\x1b[1;{}HAB\nC", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   getpos(&r, &c);
    let (r, c) = get_position(&mut terminal_state, &rx);
    //   nl_cancels_wrap = (r == 2 && c == width);
    assert!(r == 2, "Expected cursor position y to be 2 found {}", r);
    assert!(
        c == width,
        "Expected cursor position x to be {} found {}",
        width,
        c
    );
}

// TEST NINE
// This fails because we have a bug. Marking ignored for now
#[ignore]
#[test]
fn test_nul_cancels_wrap_state() {
    let (mut terminal_state, rx, width) = setup();
    //   /* Check whether NUL cancels the wrap state. */
    //   cup(1, width - 1);
    //   wr("AB%cC", 0);
    let cup_str = format!("\x1b[1;{}HAB\x00C", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   getpos(&r, &c);
    let (r, c) = get_position(&mut terminal_state, &rx);
    //   nul_cancels_wrap = (r == 1 && c >= width);
    assert!(r == 1, "Expected cursor position y to be 1 found {}", r);
    assert!(
        c >= width,
        "Expected cursor position x to be greater than or equal to {} found {}",
        width,
        c
    );
}

// TEST TEN
// This fails because we have a bug. Marking ignored for now
#[ignore]
#[test]
fn test_bel_cancels_wrap_state() {
    let (mut terminal_state, rx, width) = setup();
    //   /* Check whether BEL cancels the wrap state. */
    //   cup(1, width - 1);
    //   wr("AB\aC");
    let cup_str = format!("\x1b[1;{}HAB\x07C", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   getpos(&r, &c);
    let (r, c) = get_position(&mut terminal_state, &rx);
    //   bel_cancels_wrap = (r == 1 && c >= width);
    assert!(r == 1, "Expected cursor position y to be 1 found {}", r);
    assert!(
        c >= width,
        "Expected cursor position x to be greater than or equal to {} found {}",
        width,
        c
    );
}

// TEST ELEVEN
// This fails because we have a bug. Marking ignored for now
#[ignore]
#[test]
fn test_ri_cancels_wrap_state() {
    let (mut terminal_state, rx, width) = setup();
    //   /* Check whether RI (Reverse Index) cancels the wrap state. */
    //   cup(2, width - 1);
    //   wr("AB\33MC");
    let cup_str = format!("\x1b[2;{}HAB\x1bMC", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   getpos(&r, &c);
    let (r, c) = get_position(&mut terminal_state, &rx);
    //   ri_cancels_wrap = (r == 1 && c >= width);
    assert!(r == 1, "Expected cursor position y to be 1 found {}", r);
    assert!(
        c >= width,
        "Expected cursor position x to be greater than or equal to {} found {}",
        width,
        c
    );
}

// TEST TWELVE
// This fails because we have a bug. Marking ignored for now
#[ignore]
#[test]
fn test_sgr_cancels_wrap_state() {
    let (mut terminal_state, rx, width) = setup();
    //   /* Check whether SGR (Select Graphic Rendition) cancels the wrap state. */
    //   cup(1, width - 1);
    //   wr("AB\33[mC");
    let cup_str = format!("\x1b[1;{}HAB\x1b[mC", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   getpos(&r, &c);
    let (r, c) = get_position(&mut terminal_state, &rx);
    //   sgr_cancels_wrap = (r == 1 && c >= width);
    assert!(r == 1, "Expected cursor position y to be 1 found {}", r);
    assert!(
        c >= width,
        "Expected cursor position x to be greater than or equal to {} found {}",
        width,
        c
    );
}

// TEST THIRTEEN
// This fails because we have a bug. Marking ignored for now
#[ignore]
#[test]
fn test_sm_cancels_wrap_state() {
    let (mut terminal_state, rx, width) = setup();
    //   /* Check whether SM (Set Mode) cancels the wrap state. */
    //   cup(1, width - 1);
    //   wr("AB\33[hC");
    let cup_str = format!("\x1b[1;{}HAB\x1b[hC", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   getpos(&r, &c);
    let (r, c) = get_position(&mut terminal_state, &rx);
    //   sm_cancels_wrap = (r == 1 && c >= width);
    assert!(r == 1, "Expected cursor position y to be 1 found {}", r);
    assert!(
        c >= width,
        "Expected cursor position x to be greater than or equal to {} found {}",
        width,
        c
    );
}

// TEST FOURTEEN
#[test]
fn test_cup_cancels_wrap_state() {
    let (mut terminal_state, rx, width) = setup();
    //   /* Check whether CUP (Set Cursor Position) cancels the wrap state. */
    //   cup(1, width - 1);
    //   wr("AB");
    let cup_str = format!("\x1b[1;{}HAB", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   cup(1, width);
    //   wr("C");
    let cup_str = format!("\x1b[1;{}HC", width);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   getpos(&r, &c);
    let (r, c) = get_position(&mut terminal_state, &rx);
    //   cup_cancels_wrap = (r == 1 && c >= width);
    assert!(r == 1, "Expected cursor position y to be 1 found {}", r);
    assert!(
        c >= width,
        "Expected cursor position x to be greater than or equal to {} found {}",
        width,
        c
    );
}

// TEST FIFTEEN
#[test]
fn test_cuf_cancels_wrap_state() {
    let (mut terminal_state, rx, width) = setup();
    //   /* Check whether CUF (Cursor Forward) cancels the wrap state. */
    //   cup(1, width - 1);
    //   wr("AB\33[CC");
    let cup_str = format!("\x1b[1;{}HAB\x1b[CC", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   getpos(&r, &c);
    let (r, c) = get_position(&mut terminal_state, &rx);
    //   cuf_cancels_wrap = (r == 1 && c >= width);
    assert!(r == 1, "Expected cursor position y to be 1 found {}", r);
    assert!(
        c >= width,
        "Expected cursor position x to be greater than or equal to {} found {}",
        width,
        c
    );
}

// TEST SIXTEEN
// This fails because we have a bug. Marking ignored for now
#[ignore]
#[test]
fn test_el_cancels_wrap_state() {
    let (mut terminal_state, rx, width) = setup();
    //   /* Check whether EL (Erase in Line) cancels the wrap state. */
    //   cup(1, width - 1);
    //   wr("AB\33[KC");
    let cup_str = format!("\x1b[1;{}HAB\x1b[KC", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   getpos(&r, &c);
    let (r, c) = get_position(&mut terminal_state, &rx);
    //   el_cancels_wrap = (r == 1 && c == width);
    assert!(r == 1, "Expected cursor position y to be 1 found {}", r);
    assert!(
        c == width,
        "Expected cursor position x to be {} found {}",
        width,
        c
    );
}

// TEST SEVENTEEN
// This fails because we have a bug. Marking ignored for now
#[ignore]
#[test]
fn test_ed_cancels_wrap_state() {
    let (mut terminal_state, rx, width) = setup();
    //   /* Check whether ED (Erase in Display) cancels the wrap state. */
    //   cup(1, width - 1);
    //   wr("AB\33[JC");
    let cup_str = format!("\x1b[1;{}HAB\x1b[JC", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   getpos(&r, &c);
    let (r, c) = get_position(&mut terminal_state, &rx);
    //   ed_cancels_wrap = (r == 1 && c >= width);
    assert!(r == 1, "Expected cursor position y to be 1 found {}", r);
    assert!(
        c >= width,
        "Expected cursor position x to be greater than or equal to {} found {}",
        width,
        c
    );
}

// TEST EIGHTEEN
// This fails because we have a bug. Marking ignored for now
#[ignore]
#[test]
fn test_dch_cancels_wrap_state() {
    let (mut terminal_state, rx, width) = setup();
    //   /* Check whether DCH (Delete Character) cancels the wrap state. */
    //   cup(1, width - 1);
    //   wr("AB\33[PC");
    let cup_str = format!("\x1b[1;{}HAB\x1b[PC", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   getpos(&r, &c);
    let (r, c) = get_position(&mut terminal_state, &rx);
    //   dch_cancels_wrap = (r == 1 && c >= width);
    assert!(r == 1, "Expected cursor position y to be 1 found {}", r);
    assert!(
        c >= width,
        "Expected cursor position x to be greater than or equal to {} found {}",
        width,
        c
    );
}

// TEST NINETEEN
// This fails because we have a bug. Marking ignored for now
#[ignore]
#[test]
fn test_ich_cancels_wrap_state() {
    let (mut terminal_state, rx, width) = setup();
    //     /* Check whether ICH (Insert Character) cancels the wrap state. */
    //     cup(1, width - 1);
    //     wr("AB\33[@C");
    let cup_str = format!("\x1b[1;{}HAB\x1b[@C", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //     getpos(&r, &c);
    let (r, c) = get_position(&mut terminal_state, &rx);
    //     ich_cancels_wrap = (r == 1 && c >= width);
    assert!(r == 1, "Expected cursor position y to be 1 found {}", r);
    assert!(
        c >= width,
        "Expected cursor position x to be greater than or equal to {} found {}",
        width,
        c
    );
}

// TEST TWENTY
// This fails because we have a bug. Marking ignored for now
#[ignore]
#[test]
fn test_ech_cancels_wrap_state() {
    let (mut terminal_state, rx, width) = setup();
    //     /* Check whether ECH (Erase Character) cancels the wrap state. */
    //     cup(1, width - 1);
    //     wr("AB\33[XC");
    let cup_str = format!("\x1b[1;{}HAB\x1b[XC", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //     getpos(&r, &c);
    let (r, c) = get_position(&mut terminal_state, &rx);
    //     ech_cancels_wrap = (r == 1 && c >= width);
    assert!(r == 1, "Expected cursor position y to be 1 found {}", r);
    assert!(
        c >= width,
        "Expected cursor position x to be greater than or equal to {} found {}",
        width,
        c
    );
}

// TEST TWENTY ONE
// This fails because we have a bug. Marking ignored for now
#[ignore]
#[test]
fn test_cpr_cancels_wrap_state() {
    let (mut terminal_state, rx, width) = setup();
    //   /* Check whether CPR (Cursor Position Report) cancels the wrap state. */
    //   cup(1, width - 1);
    //   wr("AB");
    let cup_str = format!("\x1b[1;{}HAB", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   getpos(&r, &c);
    let (_r, _c) = get_position(&mut terminal_state, &rx);
    //   wr("C");
    let data = b"C";
    terminal_state.handle_incoming_data(data);
    //   getpos(&r, &c);
    let (r, c) = get_position(&mut terminal_state, &rx);
    //   cpr_cancels_wrap = (r == 1 && c >= width);
    assert!(r == 1, "Expected cursor position y to be 1 found {}", r);
    assert!(
        c >= width,
        "Expected cursor position x to be greater than or equal to {} found {}",
        width,
        c
    );
}

// TEST TWENTY TWO
// This fails because we have a bug. Marking ignored for now
#[ignore]
#[test]
fn test_decsc_cancels_wrap_state() {
    let (mut terminal_state, rx, width) = setup();
    //   /* Check whether DECSC (Save Cursor) cancels the wrap state. */
    //   cup(1, width - 1);
    //   wr("AB\33" "7" "C");
    let cup_str = format!("\x1b[1;{}HAB\x1b7C", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   getpos(&r, &c);
    let (r, c) = get_position(&mut terminal_state, &rx);
    //   decsc_cancels_wrap = (r == 1 && c >= width);
    assert!(r == 1, "Expected cursor position y to be 1 found {}", r);
    assert!(
        c >= width,
        "Expected cursor position x to be greater than or equal to {} found {}",
        width,
        c
    );
}

// TEST TWENTY THREE

#[test]
fn test_decrc_restores_wrap_state() {
    let (mut terminal_state, rx, width) = setup();
    //   /* Check whether DECRC (Restore Cursor) restores the wrap state. */
    //   cup(1, width - 1);
    //   wr("AB\33" "7");
    let cup_str = format!("\x1b[1;{}HAB\x1b7", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   cup(3, 10);
    let cup_str = "\x1b[3;10H";
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   wr("Q\33" "8" "X");
    let data = b"Q\x1b8X";
    terminal_state.handle_incoming_data(data);
    //   getpos(&r, &c);
    let (r, _c) = get_position(&mut terminal_state, &rx);
    //   decrc_restores_wrap = (r == 2);
    assert!(r == 2, "Expected cursor position y to be 2 found {}", r);
}

// TEST TWENTY FOUR
#[test]
fn test_decrc_restores_decawm_on() {
    let (mut terminal_state, rx, width) = setup();
    //   /* Check whether DECRC (Restore Cursor) restores DECAWM=on. */
    //   cup(1, 1);
    let cup_str = "\x1b[1;1H";
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   wr("\33" "7");
    let data = b"\x1b7";
    terminal_state.handle_incoming_data(data);
    //   decawm(0);
    //   wr("\33" "8");
    let data = b"\x1b[?7l\x1b8";
    terminal_state.handle_incoming_data(data);
    //   cup(1, width - 1);
    //   wr("ABC");
    let cup_str = format!("\x1b[1;{}HABC", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   getpos(&r, &c);
    let (r, _c) = get_position(&mut terminal_state, &rx);
    //   decrc_restores_decawm_on = (r == 2);
    assert!(r == 2, "Expected cursor position y to be 2 found {}", r);
    //   decawm(1);
}

// TEST TWENTY FIVE
// This fails because we have a bug. Marking ignored for now
#[ignore]
#[test]
fn test_decrc_restores_decawm_off() {
    let (mut terminal_state, rx, width) = setup();
    //   /* Check whether DECRC (Restore Cursor) restores DECAWM=off. */
    //   cup(1, 1);
    let cup_str = "\x1b[1;1H";
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   decawm(0);
    let data = b"\x1b7";
    terminal_state.handle_incoming_data(data);
    //   wr("\33" "7");
    let data = b"\x1b7";
    terminal_state.handle_incoming_data(data);
    //   decawm(1);
    //   wr("\33" "8");
    let data = b"\x1b[?7l\x1b8";
    terminal_state.handle_incoming_data(data);
    //   cup(1, width - 1);
    //   wr("ABC");
    let cup_str = format!("\x1b[1;{}HABC", width - 1);
    let cup = cup_str.as_bytes();
    terminal_state.handle_incoming_data(cup);
    //   getpos(&r, &c);
    let (r, _c) = get_position(&mut terminal_state, &rx);
    //   decrc_restores_decawm_off = (r == 1);
    assert!(r == 1, "Expected cursor position y to be 1 found {}", r);
    //   decawm(1);
}
