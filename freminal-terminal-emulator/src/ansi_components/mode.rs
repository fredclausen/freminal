// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::fmt;

use super::modes::{
    decawm::Decawm, decckm::Decckm, dectcem::Dectcem, mouse::MouseTrack, rl_bracket::RlBracket,
    sync_updates::SynchronizedUpdates, unknown::UnknownMode, xtcblink::XtCBlink,
    xtextscrn::XtExtscrn, xtmsewin::XtMseWin, ReportMode,
};

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Eq, PartialEq, Default)]
pub enum SetMode {
    DecSet,
    #[default]
    DecRst,
    DecQuery,
}

impl fmt::Display for SetMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DecSet => write!(f, "Mode Set"),
            Self::DecRst => write!(f, "Mode Reset"),
            Self::DecQuery => write!(f, "Mode Query"),
        }
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

impl Mode {
    #[must_use]
    pub fn terminal_mode_from_params(params: &[u8], mode: &SetMode) -> Self {
        match params {
            // https://vt100.net/docs/vt510-rm/DECCKM.html
            b"?1" => Self::Decckm(Decckm::new(mode)),
            b"?7" => Self::Decawm(Decawm::new(mode)),
            // TODO: Implement this
            b"?9" => {
                if mode == &SetMode::DecSet {
                    Self::MouseMode(MouseTrack::XtMsex10)
                } else if mode == &SetMode::DecRst {
                    Self::MouseMode(MouseTrack::NoTracking)
                } else {
                    Self::MouseMode(MouseTrack::Query(9))
                }
            }
            b"?12" => Self::XtCBlink(XtCBlink::new(mode)),
            b"?25" => Self::Dectem(Dectcem::new(mode)),
            b"?1000" => {
                if mode == &SetMode::DecSet {
                    Self::MouseMode(MouseTrack::XtMseX11)
                } else if mode == &SetMode::DecRst {
                    Self::MouseMode(MouseTrack::NoTracking)
                } else {
                    Self::MouseMode(MouseTrack::Query(1000))
                }
            }
            b"?1002" => {
                if mode == &SetMode::DecSet {
                    Self::MouseMode(MouseTrack::XtMseBtn)
                } else if mode == &SetMode::DecRst {
                    Self::MouseMode(MouseTrack::NoTracking)
                } else {
                    Self::MouseMode(MouseTrack::Query(1002))
                }
            }
            b"?1003" => {
                if mode == &SetMode::DecSet {
                    Self::MouseMode(MouseTrack::XtMseAny)
                } else if mode == &SetMode::DecRst {
                    Self::MouseMode(MouseTrack::NoTracking)
                } else {
                    Self::MouseMode(MouseTrack::Query(1003))
                }
            }
            b"?1004" => Self::XtMseWin(XtMseWin::new(mode)),
            // TODO: Implement this
            b"?1005" => {
                if mode == &SetMode::DecSet {
                    Self::MouseMode(MouseTrack::XtMseUtf)
                } else if mode == &SetMode::DecRst {
                    Self::MouseMode(MouseTrack::NoTracking)
                } else {
                    Self::MouseMode(MouseTrack::Query(1005))
                }
            }
            // TODO: Implement this
            b"?1006" => {
                if mode == &SetMode::DecSet {
                    Self::MouseMode(MouseTrack::XtMseSgr)
                } else if mode == &SetMode::DecRst {
                    Self::MouseMode(MouseTrack::NoTracking)
                } else {
                    Self::MouseMode(MouseTrack::Query(1006))
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
            //         Self::MouseMode(MouseTrack::XtMseUrXvt)
            //     } else if mode == &SetMode::DecRst {
            //         Self::MouseMode(MouseTrack::NoTracking)
            //     } else {
            //         Self::MouseMode(MouseTrack::Query(1015))
            //     }
            // }
            // TODO: Implement this
            b"?1016" => {
                if mode == &SetMode::DecSet {
                    Self::MouseMode(MouseTrack::XtMseSgrPixels)
                } else if mode == &SetMode::DecRst {
                    Self::MouseMode(MouseTrack::NoTracking)
                } else {
                    Self::MouseMode(MouseTrack::Query(1016))
                }
            }
            b"?1049" => Self::XtExtscrn(XtExtscrn::new(mode)),
            b"?2004" => Self::BracketedPaste(RlBracket::new(mode)),
            b"?2026" => Self::SynchronizedUpdates(SynchronizedUpdates::new(mode)),
            _ => {
                let output_params = params
                    .to_vec()
                    .iter()
                    .skip(usize::from(params[0] == b'?'))
                    .copied()
                    .collect::<Vec<u8>>();

                if mode == &SetMode::DecQuery {
                    Self::UnknownQuery(output_params)
                } else {
                    Self::Unknown(UnknownMode::new(&output_params))
                }
            }
        }
    }
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
