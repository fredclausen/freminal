// Copyright (C) 2024–2025 Fred Clausen
// Licensed under the MIT license (https://opensource.org/licenses/MIT).

use freminal_terminal_emulator::ansi_components::mode::SetMode;
use freminal_terminal_emulator::ansi_components::modes::ReportMode;
use freminal_terminal_emulator::ansi_components::modes::{
    allow_column_mode_switch::AllowColumnModeSwitch,
    decarm::Decarm,
    decckm::Decckm,
    decom::Decom,
    decsclm::Decsclm,
    decscnm::Decscnm,
    lnm::Lnm,
    reverse_wrap_around::ReverseWrapAround,
    rl_bracket::RlBracket,
    sync_updates::SynchronizedUpdates,
    // The “X” and “Sync” modes use non-standard capitalization; import with aliases if they exist.
    xtcblink::XtCBlink as Xtcblink,
    xtextscrn::XtExtscrn as Xtextscrn,
    xtmsewin::XtMseWin as Xtmsewin,
};

/// Generic validator for any ReportMode implementation.
/// Works for both cloneable and non-cloneable types.
fn verify_mode<T>(name: &str, mode: &T)
where
    T: ReportMode + core::fmt::Debug + core::fmt::Display + Default,
{
    let report = mode.report(None);
    let dbg = format!("{:?}", mode);
    let disp = format!("{}", mode);

    assert!(
        report.starts_with("\x1b[?"),
        "{name}: expected report to start with CSI '?', got {report:?}"
    );
    assert!(
        !disp.is_empty(),
        "{name}: Display string should not be empty"
    );
    assert!(!dbg.is_empty(), "{name}: Debug string should not be empty");

    for sm in [SetMode::DecSet, SetMode::DecRst, SetMode::DecQuery] {
        let rep = mode.report(Some(sm));
        assert!(
            rep.starts_with("\x1b[?"),
            "{name}: override report should start with CSI '?', got {rep:?}"
        );
    }
}

#[test]
fn all_modes_generate_valid_reports() {
    // Each entry: (label, constructor)
    #[allow(clippy::type_complexity)]
    let matrix: Vec<(&str, fn() -> Box<dyn core::any::Any>)> = vec![
        ("AllowColumnModeSwitch", || {
            Box::new(AllowColumnModeSwitch::default())
        }),
        ("Decarm", || Box::new(Decarm::default())),
        ("Decckm", || Box::new(Decckm::default())),
        ("Decom", || Box::new(Decom::default())),
        ("Decsclm", || Box::new(Decsclm::default())),
        ("Decscnm", || Box::new(Decscnm::default())),
        ("Lnm", || Box::new(Lnm::default())),
        ("ReverseWrapAround", || {
            Box::new(ReverseWrapAround::default())
        }),
        ("RlBracket", || Box::new(RlBracket::default())),
        ("SyncUpdates", || Box::new(SynchronizedUpdates::default())),
        ("Xtcblink", || Box::new(Xtcblink::default())),
        ("Xtextscrn", || Box::new(Xtextscrn::default())),
        ("Xtmsewin", || Box::new(Xtmsewin::default())),
    ];

    for (name, _ctor) in matrix {
        match name {
            "AllowColumnModeSwitch" => verify_mode(name, &AllowColumnModeSwitch::default()),
            "Decarm" => verify_mode(name, &Decarm::default()),
            "Decckm" => verify_mode(name, &Decckm::default()),
            "Decom" => verify_mode(name, &Decom::default()),
            "Decsclm" => verify_mode(name, &Decsclm::default()),
            "Decscnm" => verify_mode(name, &Decscnm::default()),
            "Lnm" => verify_mode(name, &Lnm::default()),
            "ReverseWrapAround" => verify_mode(name, &ReverseWrapAround::default()),
            "RlBracket" => verify_mode(name, &RlBracket::default()),
            "SyncUpdates" => verify_mode(name, &SynchronizedUpdates::default()),
            "Xtcblink" => verify_mode(name, &Xtcblink::default()),
            "Xtextscrn" => verify_mode(name, &Xtextscrn::default()),
            "Xtmsewin" => verify_mode(name, &Xtmsewin::default()),
            _ => {
                panic!("Unexpected mode: {name}");
            }
        }
    }
}
