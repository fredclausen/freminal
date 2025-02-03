// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use freminal_terminal_emulator::ansi_components::{
    mode::{Mode, SetMode},
    modes::{
        decawm::Decawm,
        decckm::Decckm,
        dectcem::Dectcem,
        mouse::{MouseEncoding, MouseTrack},
        rl_bracket::RlBracket,
        sync_updates::SynchronizedUpdates,
        unknown::UnknownMode,
        xtcblink::XtCBlink,
        xtextscrn::XtExtscrn,
        xtmsewin::XtMseWin,
        MouseModeNumber, ReportMode,
    },
};
use test_log::test;

#[test]
fn test_decckm() {
    // Test the DECCKM mode
    let mode = Decckm::new(&SetMode::DecRst);
    assert_eq!(mode, Decckm::Ansi);
    assert_eq!(mode.to_string(), "Cursor Key Mode (DECCKM) ANSI");
    assert!(mode.report(None).contains("\x1b[?1;2$y"));
    assert!(mode.report(Some(SetMode::DecSet)).contains("\x1b[?1;1$y"));
    assert!(mode.report(Some(SetMode::DecRst)).contains("\x1b[?1;2$y"));
    assert!(mode.report(Some(SetMode::DecQuery)).contains("\x1b[?1;0$y"));

    let mode = Decckm::new(&SetMode::DecSet);
    assert_eq!(mode, Decckm::Application);
    assert_eq!(mode.to_string(), "Cursor Key Mode (DECCKM) Application");
    assert!(mode.report(None).contains("\x1b[?1;1$y"));
    assert!(mode.report(Some(SetMode::DecSet)).contains("\x1b[?1;1$y"));
    assert!(mode.report(Some(SetMode::DecRst)).contains("\x1b[?1;2$y"));
    assert!(mode.report(Some(SetMode::DecQuery)).contains("\x1b[?1;0$y"));

    let mode = Decckm::new(&SetMode::DecQuery);
    assert_eq!(mode, Decckm::Query);
    assert_eq!(mode.to_string(), "Cursor Key Mode (DECCKM) Query");
    assert!(mode.report(None).contains("\x1b[?1;0$y"));
    assert!(mode.report(Some(SetMode::DecSet)).contains("\x1b[?1;1$y"));
    assert!(mode.report(Some(SetMode::DecRst)).contains("\x1b[?1;2$y"));
    assert!(mode.report(Some(SetMode::DecQuery)).contains("\x1b[?1;0$y"));
}

#[test]
fn test_decawm() {
    // Test the DECAWM mode
    let mode = Decawm::new(&SetMode::DecRst);
    assert_eq!(mode, Decawm::NoAutoWrap);
    assert_eq!(mode.to_string(), "Autowrap Mode (DECAWM) Disabled");
    assert!(mode.report(None).contains("\x1b[?7;2$y"));
    assert!(mode.report(Some(SetMode::DecSet)).contains("\x1b[?7;1$y"));
    assert!(mode.report(Some(SetMode::DecRst)).contains("\x1b[?7;2$y"));
    assert!(mode.report(Some(SetMode::DecQuery)).contains("\x1b[?7;0$y"));

    let mode = Decawm::new(&SetMode::DecSet);
    assert_eq!(mode, Decawm::AutoWrap);
    assert_eq!(mode.to_string(), "Autowrap Mode (DECAWM) Enabled");
    assert!(mode.report(None).contains("\x1b[?7;1$y"));
    assert!(mode.report(Some(SetMode::DecSet)).contains("\x1b[?7;1$y"));
    assert!(mode.report(Some(SetMode::DecRst)).contains("\x1b[?7;2$y"));
    assert!(mode.report(Some(SetMode::DecQuery)).contains("\x1b[?7;0$y"));

    let mode = Decawm::new(&SetMode::DecQuery);
    assert_eq!(mode, Decawm::Query);
    assert_eq!(mode.to_string(), "Autowrap Mode (DECAWM) Query");
    assert!(mode.report(None).contains("\x1b[?7;0$y"));
    assert!(mode.report(Some(SetMode::DecSet)).contains("\x1b[?7;1$y"));
    assert!(mode.report(Some(SetMode::DecRst)).contains("\x1b[?7;2$y"));
    assert!(mode.report(Some(SetMode::DecQuery)).contains("\x1b[?7;0$y"));
}

#[test]
fn test_dectcem() {
    let mode = Dectcem::new(&SetMode::DecRst);
    assert_eq!(mode, Dectcem::Hide);
    assert_eq!(mode.to_string(), "Hide Cursor (DECTCEM)");
    assert!(mode.report(None).contains("\x1b[?25;2$y"));
    assert!(mode.report(Some(SetMode::DecSet)).contains("\x1b[?25;1$y"));
    assert!(mode.report(Some(SetMode::DecRst)).contains("\x1b[?25;2$y"));
    assert!(mode
        .report(Some(SetMode::DecQuery))
        .contains("\x1b[?25;0$y"));

    let mode = Dectcem::new(&SetMode::DecSet);
    assert_eq!(mode, Dectcem::Show);
    assert_eq!(mode.to_string(), "Show Cursor (DECTCEM)");
    assert!(mode.report(None).contains("\x1b[?25;1$y"));
    assert!(mode.report(Some(SetMode::DecSet)).contains("\x1b[?25;1$y"));
    assert!(mode.report(Some(SetMode::DecRst)).contains("\x1b[?25;2$y"));
    assert!(mode
        .report(Some(SetMode::DecQuery))
        .contains("\x1b[?25;0$y"));

    let mode = Dectcem::new(&SetMode::DecQuery);
    assert_eq!(mode, Dectcem::Query);
    assert_eq!(mode.to_string(), "Query Cursor (DECTCEM)");
    assert!(mode.report(None).contains("\x1b[?25;0$y"));
    assert!(mode.report(Some(SetMode::DecSet)).contains("\x1b[?25;1$y"));
    assert!(mode.report(Some(SetMode::DecRst)).contains("\x1b[?25;2$y"));
    assert!(mode
        .report(Some(SetMode::DecQuery))
        .contains("\x1b[?25;0$y"));
}

#[test]
fn test_rlbracket() {
    let mode = RlBracket::new(&SetMode::DecRst);
    assert_eq!(mode, RlBracket::Disabled);
    assert_eq!(mode.to_string(), "Bracketed Paste Mode (DEC 2004) Disabled");
    assert!(mode.report(None).contains("\x1b[?2004;2$y"));
    assert!(mode
        .report(Some(SetMode::DecSet))
        .contains("\x1b[?2004;1$y"));
    assert!(mode
        .report(Some(SetMode::DecRst))
        .contains("\x1b[?2004;2$y"));
    assert!(mode
        .report(Some(SetMode::DecQuery))
        .contains("\x1b[?2004;0$y"));

    let mode = RlBracket::new(&SetMode::DecSet);
    assert_eq!(mode, RlBracket::Enabled);
    assert_eq!(mode.to_string(), "Bracketed Paste Mode (DEC 2004) Enabled");
    assert!(mode.report(None).contains("\x1b[?2004;1$y"));
    assert!(mode
        .report(Some(SetMode::DecSet))
        .contains("\x1b[?2004;1$y"));
    assert!(mode
        .report(Some(SetMode::DecRst))
        .contains("\x1b[?2004;2$y"));
    assert!(mode
        .report(Some(SetMode::DecQuery))
        .contains("\x1b[?2004;0$y"));

    let mode = RlBracket::new(&SetMode::DecQuery);
    assert_eq!(mode, RlBracket::Query);
    assert_eq!(mode.to_string(), "Bracketed Paste Mode (DEC 2004) Query");
    assert!(mode.report(None).contains("\x1b[?2004;0$y"));
    assert!(mode
        .report(Some(SetMode::DecSet))
        .contains("\x1b[?2004;1$y"));
    assert!(mode
        .report(Some(SetMode::DecRst))
        .contains("\x1b[?2004;2$y"));
    assert!(mode
        .report(Some(SetMode::DecQuery))
        .contains("\x1b[?2004;0$y"));
}

#[test]
fn test_synchronized_updates() {
    let mode = SynchronizedUpdates::new(&SetMode::DecRst);
    assert_eq!(mode, SynchronizedUpdates::Draw);
    assert_eq!(
        mode.to_string(),
        "Synchronized Updates Mode (DEC 2026) Draw"
    );
    assert!(mode.report(None).contains("\x1b[?2026;2$y"));
    assert!(mode
        .report(Some(SetMode::DecSet))
        .contains("\x1b[?2026;1$y"));
    assert!(mode
        .report(Some(SetMode::DecRst))
        .contains("\x1b[?2026;2$y"));
    assert!(mode
        .report(Some(SetMode::DecQuery))
        .contains("\x1b[?2026;0$y"));

    let mode = SynchronizedUpdates::new(&SetMode::DecSet);
    assert_eq!(mode, SynchronizedUpdates::DontDraw);
    assert_eq!(
        mode.to_string(),
        "Synchronized Updates Mode (DEC 2026) Don't Draw"
    );
    assert!(mode.report(None).contains("\x1b[?2026;1$y"));
    assert!(mode
        .report(Some(SetMode::DecSet))
        .contains("\x1b[?2026;1$y"));
    assert!(mode
        .report(Some(SetMode::DecRst))
        .contains("\x1b[?2026;2$y"));
    assert!(mode
        .report(Some(SetMode::DecQuery))
        .contains("\x1b[?2026;0$y"));

    let mode = SynchronizedUpdates::new(&SetMode::DecQuery);
    assert_eq!(mode, SynchronizedUpdates::Query);
    assert_eq!(
        mode.to_string(),
        "Synchronized Updates Mode (DEC 2026) Query"
    );
    assert!(mode.report(None).contains("\x1b[?2026;0$y"));
    assert!(mode
        .report(Some(SetMode::DecSet))
        .contains("\x1b[?2026;1$y"));
    assert!(mode
        .report(Some(SetMode::DecRst))
        .contains("\x1b[?2026;2$y"));
    assert!(mode
        .report(Some(SetMode::DecQuery))
        .contains("\x1b[?2026;0$y"));
}

#[test]
fn test_xtcblink() {
    let mode = XtCBlink::new(&SetMode::DecRst);
    assert_eq!(mode, XtCBlink::Steady);
    assert_eq!(mode.to_string(), "XT_CBLINK (RESET) Cursor Steady");
    assert!(mode.report(None).contains("\x1b[?12;2$y"));
    assert!(mode.report(Some(SetMode::DecSet)).contains("\x1b[?12;1$y"));
    assert!(mode.report(Some(SetMode::DecRst)).contains("\x1b[?12;2$y"));
    assert!(mode
        .report(Some(SetMode::DecQuery))
        .contains("\x1b[?12;0$y"));

    let mode = XtCBlink::new(&SetMode::DecSet);
    assert_eq!(mode, XtCBlink::Blinking);
    assert_eq!(mode.to_string(), "XT_CBLINK (SET) Cursor Blinking");
    assert!(mode.report(None).contains("\x1b[?12;1$y"));
    assert!(mode.report(Some(SetMode::DecSet)).contains("\x1b[?12;1$y"));
    assert!(mode.report(Some(SetMode::DecRst)).contains("\x1b[?12;2$y"));
    assert!(mode
        .report(Some(SetMode::DecQuery))
        .contains("\x1b[?12;0$y"));

    let mode = XtCBlink::new(&SetMode::DecQuery);
    assert_eq!(mode, XtCBlink::Query);
    assert_eq!(mode.to_string(), "XT_CBLINK (QUERY)");
    assert!(mode.report(None).contains("\x1b[?12;0$y"));
    assert!(mode.report(Some(SetMode::DecSet)).contains("\x1b[?12;1$y"));
    assert!(mode.report(Some(SetMode::DecRst)).contains("\x1b[?12;2$y"));
    assert!(mode
        .report(Some(SetMode::DecQuery))
        .contains("\x1b[?12;0$y"));
}

#[test]
fn test_xtextscrn() {
    let mode = XtExtscrn::new(&SetMode::DecRst);
    assert_eq!(mode, XtExtscrn::Primary);
    assert_eq!(mode.to_string(), "XT_EXTSCRN (RESET) Primary Screen");
    assert!(mode.report(None).contains("\x1b[?1049;2$y"));
    assert!(mode
        .report(Some(SetMode::DecSet))
        .contains("\x1b[?1049;1$y"));
    assert!(mode
        .report(Some(SetMode::DecRst))
        .contains("\x1b[?1049;2$y"));
    assert!(mode
        .report(Some(SetMode::DecQuery))
        .contains("\x1b[?1049;0$y"));

    let mode = XtExtscrn::new(&SetMode::DecSet);
    assert_eq!(mode, XtExtscrn::Alternate);
    assert_eq!(mode.to_string(), "XT_EXTSCRN (SET) Alternate Screen");
    assert!(mode.report(None).contains("\x1b[?1049;1$y"));
    assert!(mode
        .report(Some(SetMode::DecSet))
        .contains("\x1b[?1049;1$y"));
    assert!(mode
        .report(Some(SetMode::DecRst))
        .contains("\x1b[?1049;2$y"));
    assert!(mode
        .report(Some(SetMode::DecQuery))
        .contains("\x1b[?1049;0$y"));

    let mode = XtExtscrn::new(&SetMode::DecQuery);
    assert_eq!(mode, XtExtscrn::Query);
    assert_eq!(mode.to_string(), "XT_EXTSCRN (QUERY)");
    assert!(mode.report(None).contains("\x1b[?1049;0$y"));
    assert!(mode
        .report(Some(SetMode::DecSet))
        .contains("\x1b[?1049;1$y"));
    assert!(mode
        .report(Some(SetMode::DecRst))
        .contains("\x1b[?1049;2$y"));
    assert!(mode
        .report(Some(SetMode::DecQuery))
        .contains("\x1b[?1049;0$y"));
}

#[test]
fn test_xtmsewin() {
    let mode = XtMseWin::new(&SetMode::DecRst);
    assert_eq!(mode, XtMseWin::Disabled);
    assert_eq!(
        mode.to_string(),
        "Focus Reporting Mode (XT_MSE_WIN) Disabled"
    );
    assert!(mode.report(None).contains("\x1b[?1004;2$y"));
    assert!(mode
        .report(Some(SetMode::DecSet))
        .contains("\x1b[?1004;1$y"));
    assert!(mode
        .report(Some(SetMode::DecRst))
        .contains("\x1b[?1004;2$y"));
    assert!(mode
        .report(Some(SetMode::DecQuery))
        .contains("\x1b[?1004;0$y"));

    let mode = XtMseWin::new(&SetMode::DecSet);
    assert_eq!(mode, XtMseWin::Enabled);
    assert_eq!(
        mode.to_string(),
        "Focus Reporting Mode (XT_MSE_WIN) Enabled"
    );
    assert!(mode.report(None).contains("\x1b[?1004;1$y"));
    assert!(mode
        .report(Some(SetMode::DecSet))
        .contains("\x1b[?1004;1$y"));
    assert!(mode
        .report(Some(SetMode::DecRst))
        .contains("\x1b[?1004;2$y"));
    assert!(mode
        .report(Some(SetMode::DecQuery))
        .contains("\x1b[?1004;0$y"));

    let mode = XtMseWin::new(&SetMode::DecQuery);
    assert_eq!(mode, XtMseWin::Query);
    assert_eq!(mode.to_string(), "Focus Reporting Mode (XT_MSE_WIN) Query");
    assert!(mode.report(None).contains("\x1b[?1004;0$y"));
    assert!(mode
        .report(Some(SetMode::DecSet))
        .contains("\x1b[?1004;1$y"));
    assert!(mode
        .report(Some(SetMode::DecRst))
        .contains("\x1b[?1004;2$y"));
    assert!(mode
        .report(Some(SetMode::DecQuery))
        .contains("\x1b[?1004;0$y"));
}

#[test]
fn test_unknown_mode() {
    let mode = UnknownMode::new(&[0x69]);
    let expected = UnknownMode {
        params: "i".to_string(),
    };
    assert_eq!(mode, expected);
    assert_eq!(mode.to_string(), "Unknown Mode(i)");
    assert!(mode.report(None).contains("\x1b[?i;0$y"));
}

#[test]
fn test_mouse_modes() {
    let mode = MouseTrack::NoTracking;
    assert_eq!(mode.mouse_mode_number(), 0);
    assert_eq!(mode.report(None), "\x1b[?0;0$y");
    assert_eq!(mode.report(Some(SetMode::DecSet)), "\x1b[?0;0$y");
    assert_eq!(mode.report(Some(SetMode::DecRst)), "\x1b[?0;0$y");
    assert_eq!(mode.report(Some(SetMode::DecQuery)), "\x1b[?0;0$y");
    assert_eq!(mode.to_string(), "NoTracking");
    assert_eq!(mode.get_encoding(), MouseEncoding::X11);

    let mode = MouseTrack::XtMsex10;
    assert_eq!(mode.mouse_mode_number(), 9);
    assert_eq!(mode.report(None), "\x1b[?9;2$y");
    assert_eq!(mode.report(Some(SetMode::DecSet)), "\x1b[?9;1$y");
    assert_eq!(mode.report(Some(SetMode::DecRst)), "\x1b[?9;2$y");
    assert_eq!(mode.report(Some(SetMode::DecQuery)), "\x1b[?9;0$y");
    assert_eq!(mode.to_string(), "XtMsex10");
    assert_eq!(mode.get_encoding(), MouseEncoding::X11);

    let mode = MouseTrack::XtMseX11;
    assert_eq!(mode.mouse_mode_number(), 1000);
    assert_eq!(mode.report(None), "\x1b[?1000;2$y");
    assert_eq!(mode.report(Some(SetMode::DecSet)), "\x1b[?1000;1$y");
    assert_eq!(mode.report(Some(SetMode::DecRst)), "\x1b[?1000;2$y");
    assert_eq!(mode.report(Some(SetMode::DecQuery)), "\x1b[?1000;0$y");
    assert_eq!(mode.to_string(), "XtMseX11");
    assert_eq!(mode.get_encoding(), MouseEncoding::X11);

    let mode = MouseTrack::XtMseBtn;
    assert_eq!(mode.mouse_mode_number(), 1002);
    assert_eq!(mode.report(None), "\x1b[?1002;2$y");
    assert_eq!(mode.report(Some(SetMode::DecSet)), "\x1b[?1002;1$y");
    assert_eq!(mode.report(Some(SetMode::DecRst)), "\x1b[?1002;2$y");
    assert_eq!(mode.report(Some(SetMode::DecQuery)), "\x1b[?1002;0$y");
    assert_eq!(mode.to_string(), "XtMseBtn");
    assert_eq!(mode.get_encoding(), MouseEncoding::X11);

    let mode = MouseTrack::XtMseAny;
    assert_eq!(mode.mouse_mode_number(), 1003);
    assert_eq!(mode.report(None), "\x1b[?1003;2$y");
    assert_eq!(mode.report(Some(SetMode::DecSet)), "\x1b[?1003;1$y");
    assert_eq!(mode.report(Some(SetMode::DecRst)), "\x1b[?1003;2$y");
    assert_eq!(mode.report(Some(SetMode::DecQuery)), "\x1b[?1003;0$y");
    assert_eq!(mode.to_string(), "XtMseAny");
    assert_eq!(mode.get_encoding(), MouseEncoding::X11);

    let mode = MouseTrack::XtMseUtf;
    assert_eq!(mode.mouse_mode_number(), 1005);
    assert_eq!(mode.report(None), "\x1b[?1005;2$y");
    assert_eq!(mode.report(Some(SetMode::DecSet)), "\x1b[?1005;1$y");
    assert_eq!(mode.report(Some(SetMode::DecRst)), "\x1b[?1005;2$y");
    assert_eq!(mode.report(Some(SetMode::DecQuery)), "\x1b[?1005;0$y");
    assert_eq!(mode.to_string(), "XtMseUtf");
    assert_eq!(mode.get_encoding(), MouseEncoding::X11);

    let mode = MouseTrack::XtMseSgr;
    assert_eq!(mode.mouse_mode_number(), 1006);
    assert_eq!(mode.report(None), "\x1b[?1006;2$y");
    assert_eq!(mode.report(Some(SetMode::DecSet)), "\x1b[?1006;1$y");
    assert_eq!(mode.report(Some(SetMode::DecRst)), "\x1b[?1006;2$y");
    assert_eq!(mode.report(Some(SetMode::DecQuery)), "\x1b[?1006;0$y");
    assert_eq!(mode.to_string(), "XtMseSgr");
    assert_eq!(mode.get_encoding(), MouseEncoding::Sgr);

    let mode = MouseTrack::XtMseUrXvt;
    assert_eq!(mode.mouse_mode_number(), 1015);
    assert_eq!(mode.report(None), "\x1b[?1015;0$y");
    assert_eq!(mode.report(Some(SetMode::DecSet)), "\x1b[?1015;0$y");
    assert_eq!(mode.report(Some(SetMode::DecRst)), "\x1b[?1015;0$y");
    assert_eq!(mode.report(Some(SetMode::DecQuery)), "\x1b[?1015;0$y");
    assert_eq!(mode.to_string(), "XtMseUrXvt");
    assert_eq!(mode.get_encoding(), MouseEncoding::X11);

    let mode = MouseTrack::XtMseSgrPixels;
    assert_eq!(mode.mouse_mode_number(), 1016);
    assert_eq!(mode.report(None), "\x1b[?1016;2$y");
    assert_eq!(mode.report(Some(SetMode::DecSet)), "\x1b[?1016;1$y");
    assert_eq!(mode.report(Some(SetMode::DecRst)), "\x1b[?1016;2$y");
    assert_eq!(mode.report(Some(SetMode::DecQuery)), "\x1b[?1016;0$y");
    assert_eq!(mode.to_string(), "XtMseSgrPixels");
    assert_eq!(mode.get_encoding(), MouseEncoding::Sgr);

    let mode = MouseTrack::Query(9);
    assert_eq!(mode.mouse_mode_number(), 9);
    assert_eq!(mode.report(None), "\x1b[?9;0$y");
    assert_eq!(mode.report(Some(SetMode::DecSet)), "\x1b[?9;0$y");
    assert_eq!(mode.report(Some(SetMode::DecRst)), "\x1b[?9;0$y");
    assert_eq!(mode.report(Some(SetMode::DecQuery)), "\x1b[?9;0$y");
    assert_eq!(mode.to_string(), "Query Mouse Tracking(9)");
    assert_eq!(mode.get_encoding(), MouseEncoding::X11);
}

#[test]
fn test_mode_none() {
    let params = b"?0";
    let mode = Mode::terminal_mode_from_params(params, &SetMode::DecSet);
    assert_eq!(mode, Mode::Unknown(UnknownMode::new(b"0")));
    assert_eq!(mode.report(None), "\x1b[?0;0$y");
    assert_eq!(mode.report(Some(SetMode::DecSet)), "\x1b[?0;0$y");
    assert_eq!(mode.report(Some(SetMode::DecRst)), "\x1b[?0;0$y");
    assert_eq!(mode.report(Some(SetMode::DecQuery)), "\x1b[?0;0$y");
    assert_eq!(mode.to_string(), "Unknown Mode(0)");

    let params = b"?1";
    let mode = Mode::terminal_mode_from_params(params, &SetMode::DecSet);
    assert_eq!(mode, Mode::Decckm(Decckm::Application));
    assert_eq!(mode.report(None), "\x1b[?1;1$y");
    assert_eq!(mode.report(Some(SetMode::DecSet)), "\x1b[?1;1$y");
    assert_eq!(mode.report(Some(SetMode::DecRst)), "\x1b[?1;2$y");
    assert_eq!(mode.report(Some(SetMode::DecQuery)), "\x1b[?1;0$y");
    assert_eq!(mode.to_string(), "Cursor Key Mode (DECCKM) Application");

    let params = b"?7";
    let mode = Mode::terminal_mode_from_params(params, &SetMode::DecSet);
    assert_eq!(mode, Mode::Decawm(Decawm::AutoWrap));
    assert_eq!(mode.report(None), "\x1b[?7;1$y");
    assert_eq!(mode.report(Some(SetMode::DecSet)), "\x1b[?7;1$y");
    assert_eq!(mode.report(Some(SetMode::DecRst)), "\x1b[?7;2$y");
    assert_eq!(mode.report(Some(SetMode::DecQuery)), "\x1b[?7;0$y");
    assert_eq!(mode.to_string(), "Autowrap Mode (DECAWM) Enabled");
}

#[test]
fn test_display_mode_for_setmode() {
    let mode = SetMode::DecSet;
    assert_eq!(mode.to_string(), "Mode Set");

    let mode = SetMode::DecRst;
    assert_eq!(mode.to_string(), "Mode Reset");

    let mode = SetMode::DecQuery;
    assert_eq!(mode.to_string(), "Mode Query");
}
