// Copyright (C) 2024–2025 Fred Clausen
// Licensed under the MIT license (https://opensource.org/licenses/MIT).

use freminal_terminal_emulator::ansi_components::mode::SetMode;
use freminal_terminal_emulator::ansi_components::modes::{
    decawm::Decawm, deccolm::Deccolm, dectcem::Dectcem, ReportMode,
};

/// --- BASIC CONSTRUCTION ----------------------------------------------------

#[test]
fn setmode_debug_and_display_are_stable() {
    let modes = [SetMode::DecSet, SetMode::DecRst, SetMode::DecQuery];

    for mode in modes {
        let dbg = format!("{:?}", mode);
        let disp = format!("{}", mode);
        assert!(
            dbg.contains("Dec"),
            "expected debug string to contain 'Dec' for {:?}",
            mode
        );
        assert!(
            !disp.is_empty(),
            "Display impl should produce non-empty string for {:?}",
            mode
        );
    }
}

/// --- REPORTING -------------------------------------------------------------

#[test]
fn report_mode_reflects_internal_state() {
    let on = Decawm::AutoWrap;
    let off = Decawm::NoAutoWrap;

    assert_eq!(on.report(None), "\x1b[?7;1$y");
    assert_eq!(off.report(None), "\x1b[?7;2$y");

    // Override report should take precedence
    assert_eq!(off.report(Some(SetMode::DecSet)), "\x1b[?7;1$y");
    assert_eq!(on.report(Some(SetMode::DecRst)), "\x1b[?7;2$y");
    assert_eq!(on.report(Some(SetMode::DecQuery)), "\x1b[?7;0$y");
}

/// --- MULTIPLE MODES TOGGLING ----------------------------------------------

#[test]
fn deccolm_and_dectcem_modes_report_consistently() {
    // Deccolm: 80 vs 132 column modes
    let wide = Deccolm::Column132;
    let narrow = Deccolm::Column80;
    assert_ne!(wide, narrow);

    let wide_report = wide.report(None);
    let narrow_report = narrow.report(None);

    // We accept either 0/1/2 semantically; just ensure correct CSI and stable response.
    assert!(
        wide_report.starts_with("\x1b[?3;"),
        "expected CSI for column 132 mode, got {:?}",
        wide_report
    );
    assert!(
        narrow_report.starts_with("\x1b[?3;"),
        "expected CSI for column 80 mode, got {:?}",
        narrow_report
    );
    assert_ne!(
        wide_report, narrow_report,
        "reports for 80/132 modes must differ"
    );

    // Dectcem: cursor show/hide
    let visible = Dectcem::Show;
    let hidden = Dectcem::Hide;
    assert_ne!(visible, hidden);

    let vis = visible.report(None);
    let hid = hidden.report(None);

    assert!(
        vis.starts_with("\x1b[?25;"),
        "expected cursor show report to start with ?25;, got {:?}",
        vis
    );
    assert!(
        hid.starts_with("\x1b[?25;"),
        "expected cursor hide report to start with ?25;, got {:?}",
        hid
    );
    assert_ne!(vis, hid, "reports for show/hide must differ");
}

/// --- CLONE & EQUALITY SEMANTICS -------------------------------------------

#[test]
fn mode_clone_and_equality_semantics_hold() {
    let a = SetMode::DecSet;
    let b = a.clone();
    assert_eq!(a, b);
    assert_ne!(a, SetMode::DecRst);
}

/// --- UNKNOWN MODE IMPLEMENTATION EXAMPLE -----------------------------------

#[test]
fn custom_reportmode_can_override_behavior() {
    struct Dummy;
    impl ReportMode for Dummy {
        fn report(&self, override_mode: Option<SetMode>) -> String {
            override_mode.map_or_else(
                || "\x1b[?999;0$y".to_string(),
                |_| "\x1b[?999;1$y".to_string(),
            )
        }
    }

    let dummy = Dummy;
    assert_eq!(dummy.report(None), "\x1b[?999;0$y");
    assert_eq!(dummy.report(Some(SetMode::DecSet)), "\x1b[?999;1$y");
}
