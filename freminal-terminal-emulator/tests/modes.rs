// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use freminal_terminal_emulator::ansi_components::{
    mode::SetMode,
    modes::{decckm::Decckm, ReportMode},
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
