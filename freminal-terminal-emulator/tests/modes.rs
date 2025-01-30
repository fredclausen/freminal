// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use freminal_terminal_emulator::ansi_components::{
    mode::SetMode,
    modes::{decawm::Decawm, decckm::Decckm, dectcem::Dectcem, rl_bracket::RlBracket, ReportMode},
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
