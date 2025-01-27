// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::fmt;

use super::modes::{
    decawm::Decawm, decckm::Decckm, dectcem::Dectcem, rl_bracket::RlBracket,
    sync_updates::SynchronizedUpdates, xtcblink::XtCBlink, xtextscrn::XtExtscrn,
    xtmsewin::XtMseWin, MouseModeNumber, ReportMode,
};

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Eq, PartialEq, Default)]
pub enum SetMode {
    DecSet,
    #[default]
    DecRst,
    DecQuery,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum MouseEncoding {
    X11,
    Sgr,
}

// https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking
#[derive(Debug, Eq, PartialEq, Default, Clone)]
pub enum MouseTrack {
    #[default]
    NoTracking,
    XtMsex10,       // ?9
    XtMseX11,       // ?1000
    XtMseBtn,       // ?1002
    XtMseAny,       // ?1003
    XtMseUtf,       // ?1005
    XtMseSgr,       // ?1006
    XtMseUrXvt,     // ?1015
    XtMseSgrPixels, // ?1016
    Query(usize),
}

impl MouseModeNumber for MouseTrack {
    fn mouse_mode_number(&self) -> usize {
        match self {
            Self::NoTracking => 0,
            Self::XtMsex10 => 9,
            Self::XtMseX11 => 1000,
            Self::XtMseBtn => 1002,
            Self::XtMseAny => 1003,
            Self::XtMseUtf => 1005,
            Self::XtMseSgr => 1006,
            Self::XtMseUrXvt => 1015,
            Self::XtMseSgrPixels => 1016,
            Self::Query(v) => *v,
        }
    }
}

impl ReportMode for MouseTrack {
    fn report(&self, override_mode: Option<SetMode>) -> String {
        let mode_number = match self {
            Self::NoTracking | Self::Query(_) => 0,
            Self::XtMsex10 => 9,
            Self::XtMseX11 => 1000,
            Self::XtMseBtn => 1002,
            Self::XtMseAny => 1003,
            Self::XtMseUtf => 1005,
            Self::XtMseSgr => 1006,
            Self::XtMseUrXvt => 1015,
            Self::XtMseSgrPixels => 1016,
        };

        let set_mode = match override_mode {
            Some(SetMode::DecSet) => 1,
            Some(SetMode::DecRst) => 2,
            Some(SetMode::DecQuery) | None => 0,
        };
        format!("\x1b[?{mode_number};{set_mode}$y")
    }
}

impl MouseTrack {
    #[must_use]
    pub fn get_encoding(&self) -> MouseEncoding {
        if self == &Self::XtMseSgr {
            MouseEncoding::Sgr
        } else {
            MouseEncoding::X11
        }
    }

    // #[must_use]
    // pub const fn should_scroll(&self) -> bool {
    //     match self {
    //         Self::NoTracking | Self::XtMsex10 | Self::XtMseX11 => false,
    //         Self::XtMseBtn
    //         | Self::XtMseAny
    //         | Self::XtMseUtf
    //         | Self::XtMseSgr
    //         | Self::XtMseUrXvt
    //         | Self::XtMseSgrPixels => true,
    //     }
    // }

    // /// Function to determine if motion is require to be reported
    // #[must_use]
    // pub const fn should_report_motion(&self) -> bool {
    //     match self {
    //         Self::NoTracking | Self::XtMsex10 | Self::XtMseX11 => false,
    //         Self::XtMseBtn
    //         | Self::XtMseAny
    //         | Self::XtMseUtf
    //         | Self::XtMseSgr
    //         | Self::XtMseUrXvt
    //         | Self::XtMseSgrPixels => true,
    //     }
    // }

    // /// Function to determine if button presses should be tracked. x10 only wants button presses, everybody else
    // /// cares if the button is pressed or released.
    // #[must_use]
    // pub const fn should_retain_position_with_button_down(&self) -> bool {
    //     match self {
    //         Self::NoTracking | Self::XtMsex10 => false,
    //         Self::XtMseX11
    //         | Self::XtMseBtn
    //         | Self::XtMseAny
    //         | Self::XtMseUtf
    //         | Self::XtMseSgr
    //         | Self::XtMseUrXvt
    //         | Self::XtMseSgrPixels => true,
    //     }
    // }
}

impl fmt::Display for MouseTrack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::XtMseX11 => write!(f, "XtMseX11"),
            Self::NoTracking => write!(f, "NoTracking"),
            Self::XtMsex10 => write!(f, "XtMsex10"),
            Self::XtMseBtn => write!(f, "XtMseBtn"),
            Self::XtMseAny => write!(f, "XtMseAny"),
            Self::XtMseUtf => write!(f, "XtMseUtf"),
            Self::XtMseSgr => write!(f, "XtMseSgr"),
            Self::XtMseUrXvt => write!(f, "XtMseUrXvt"),
            Self::XtMseSgrPixels => write!(f, "XtMseSgrPixels"),
            Self::Query(v) => write!(f, "Query Mouse Tracking({v})"),
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
pub enum Mode {
    // Cursor keys mode
    // https://vt100.net/docs/vt100-ug/chapter3.html
    Decckm(Decckm),
    Decawm(Decawm),
    Dectem(Dectcem),
    XtCBlink(XtCBlink),
    XtExtscrn(XtExtscrn),
    XtMseWin(XtMseWin),
    BracketedPaste(RlBracket),
    MouseMode(MouseTrack),
    SynchronizedUpdates(SynchronizedUpdates),
    UnknownQuery(Vec<u8>),
    Unknown(UnknownMode),
}

impl ReportMode for Mode {
    fn report(&self, override_mode: Option<SetMode>) -> String {
        match self {
            Self::Decckm(decckm) => decckm.report(override_mode),
            Self::Decawm(decawm) => decawm.report(override_mode),
            Self::Dectem(dectem) => dectem.report(override_mode),
            Self::XtCBlink(xt_cblink) => xt_cblink.report(override_mode),
            Self::XtExtscrn(xt_extscrn) => xt_extscrn.report(override_mode),
            Self::XtMseWin(xt_mse_win) => xt_mse_win.report(override_mode),
            Self::BracketedPaste(rl_bracket) => rl_bracket.report(override_mode),
            Self::MouseMode(mouse_mode) => mouse_mode.report(override_mode),
            Self::SynchronizedUpdates(sync_updates) => sync_updates.report(override_mode),
            Self::Unknown(mode) => mode.report(override_mode),
            Self::UnknownQuery(v) => {
                // convert each digit to a char
                let digits = v.iter().map(|&x| x as char).collect::<String>();
                format!("\x1b[?{digits};0$y")
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct UnknownMode {
    pub params: String,
}

impl UnknownMode {
    #[must_use]
    pub fn new(params: &[u8]) -> Self {
        let params_s = std::str::from_utf8(params).unwrap_or("Unknown");

        Self {
            params: params_s.to_string(),
        }
    }
}

impl ReportMode for UnknownMode {
    // FIXME: we may need to get specific about DEC vs ANSI here. For now....we'll just report DEC
    fn report(&self, _override_mode: Option<SetMode>) -> String {
        format!("\x1b[?{};0;$y", self.params)
    }
}

impl fmt::Display for UnknownMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unknown Mode({})", self.params)
    }
}

#[derive(Debug, Eq, PartialEq, Default)]
pub struct TerminalModes {
    pub cursor_key: Decckm,
    pub bracketed_paste: RlBracket,
    pub focus_reporting: XtMseWin,
    pub cursor_blinking: XtCBlink,
    pub mouse_tracking: MouseTrack,
    pub synchronized_updates: SynchronizedUpdates,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decckm(decckm) => write!(f, "{decckm}"),
            Self::Decawm(decawm) => write!(f, "{decawm}"),
            Self::Dectem(dectem) => write!(f, "{dectem}"),
            Self::XtCBlink(xt_cblink) => write!(f, "{xt_cblink}"),
            Self::MouseMode(mouse_mode) => write!(f, "{mouse_mode}"),
            Self::XtMseWin(xt_mse_win) => write!(f, "{xt_mse_win}"),
            Self::XtExtscrn(xt_extscrn) => write!(f, "{xt_extscrn}"),
            Self::BracketedPaste(bracketed_paste) => write!(f, "{bracketed_paste}"),
            Self::SynchronizedUpdates(sync_updates) => write!(f, "{sync_updates}"),
            Self::Unknown(params) => write!(f, "{params}"),
            Self::UnknownQuery(v) => write!(f, "Unknown Query({v:?})"),
        }
    }
}

#[must_use]
pub fn terminal_mode_from_params(params: &[u8], mode: &SetMode) -> Mode {
    match params {
        // https://vt100.net/docs/vt510-rm/DECCKM.html
        b"?1" => Mode::Decckm(Decckm::new(mode)),
        b"?7" => Mode::Decawm(Decawm::new(mode)),
        // TODO: Implement this
        b"?9" => {
            if mode == &SetMode::DecSet {
                Mode::MouseMode(MouseTrack::XtMsex10)
            } else if mode == &SetMode::DecRst {
                Mode::MouseMode(MouseTrack::NoTracking)
            } else {
                Mode::MouseMode(MouseTrack::Query(9))
            }
        }
        b"?12" => Mode::XtCBlink(XtCBlink::new(mode)),
        b"?25" => Mode::Dectem(Dectcem::new(mode)),
        b"?1000" => {
            if mode == &SetMode::DecSet {
                Mode::MouseMode(MouseTrack::XtMseX11)
            } else if mode == &SetMode::DecRst {
                Mode::MouseMode(MouseTrack::NoTracking)
            } else {
                Mode::MouseMode(MouseTrack::Query(1000))
            }
        }
        b"?1002" => {
            if mode == &SetMode::DecSet {
                Mode::MouseMode(MouseTrack::XtMseBtn)
            } else if mode == &SetMode::DecRst {
                Mode::MouseMode(MouseTrack::NoTracking)
            } else {
                Mode::MouseMode(MouseTrack::Query(1002))
            }
        }
        b"?1003" => {
            if mode == &SetMode::DecSet {
                Mode::MouseMode(MouseTrack::XtMseAny)
            } else if mode == &SetMode::DecRst {
                Mode::MouseMode(MouseTrack::NoTracking)
            } else {
                Mode::MouseMode(MouseTrack::Query(1003))
            }
        }
        b"?1004" => Mode::XtMseWin(XtMseWin::new(mode)),
        // TODO: Implement this
        b"?1005" => {
            if mode == &SetMode::DecSet {
                Mode::MouseMode(MouseTrack::XtMseUtf)
            } else if mode == &SetMode::DecRst {
                Mode::MouseMode(MouseTrack::NoTracking)
            } else {
                Mode::MouseMode(MouseTrack::Query(1005))
            }
        }
        // TODO: Implement this
        b"?1006" => {
            if mode == &SetMode::DecSet {
                Mode::MouseMode(MouseTrack::XtMseSgr)
            } else if mode == &SetMode::DecRst {
                Mode::MouseMode(MouseTrack::NoTracking)
            } else {
                Mode::MouseMode(MouseTrack::Query(1006))
            }
        }
        // For now, we'll ignore this. Reading documentation it seems like this is
        // a pretty terrible format to use for mouse tracking.
        // From the documentation:
        // However, CSI M  can be mistaken for DL (delete lines), while
        //   the highlight tracking CSI T  can be mistaken for SD (scroll
        //   down), and the Window manipulation controls.  For these
        //   reasons, the 1015 control is not recommended; it is not an
        //  improvement over 1006.
        // b"?1015" => {
        //     if mode == &SetMode::DecSet {
        //         Mode::MouseMode(MouseTrack::XtMseUrXvt)
        //     } else if mode == &SetMode::DecRst {
        //         Mode::MouseMode(MouseTrack::NoTracking)
        //     } else {
        //         Mode::MouseMode(MouseTrack::Query(1015))
        //     }
        // }
        // TODO: Implement this
        b"?1016" => {
            if mode == &SetMode::DecSet {
                Mode::MouseMode(MouseTrack::XtMseSgrPixels)
            } else if mode == &SetMode::DecRst {
                Mode::MouseMode(MouseTrack::NoTracking)
            } else {
                Mode::MouseMode(MouseTrack::Query(1016))
            }
        }
        b"?1049" => Mode::XtExtscrn(XtExtscrn::new(mode)),
        b"?2004" => Mode::BracketedPaste(RlBracket::new(mode)),
        b"?2026" => Mode::SynchronizedUpdates(SynchronizedUpdates::new(mode)),
        _ => {
            if mode == &SetMode::DecQuery {
                let output_params = params
                    .to_vec()
                    .iter()
                    .skip(usize::from(params[0] == b'?'))
                    .copied()
                    .collect::<Vec<u8>>();

                Mode::UnknownQuery(output_params)
            } else {
                Mode::Unknown(UnknownMode::new(params))
            }
        }
    }
}
