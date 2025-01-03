use std::fmt;

use super::modes::{
    decawm::Decawm, decckm::Decckm, dectcem::Dectcem, rl_bracket::RlBracket, xtcblink::XtCBlink,
    xtextscrn::XtExtscrn, xtmsewin::XtMseWin,
};

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Eq, PartialEq, Default)]
pub enum SetMode {
    DecSet,
    #[default]
    DecRst,
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
    Unknown(Vec<u8>),
}

#[derive(Debug, Eq, PartialEq, Default)]
pub struct TerminalModes {
    pub cursor_key: Decckm,
    pub bracketed_paste: RlBracket,
    pub focus_reporting: XtMseWin,
    pub cursor_blinking: XtCBlink,
    pub mouse_tracking: MouseTrack,
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
            Self::Unknown(params) => {
                let params_s = std::str::from_utf8(params).unwrap_or("Unknown");

                f.write_fmt(format_args!("Unknown Mode({params_s})"))
            }
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
            } else {
                Mode::MouseMode(MouseTrack::NoTracking)
            }
        }
        b"?12" => Mode::XtCBlink(XtCBlink::new(mode)),
        b"?25" => Mode::Dectem(Dectcem::new(mode)),
        b"?1000" => {
            if mode == &SetMode::DecSet {
                Mode::MouseMode(MouseTrack::XtMseX11)
            } else {
                Mode::MouseMode(MouseTrack::NoTracking)
            }
        }
        b"?1002" => {
            if mode == &SetMode::DecSet {
                Mode::MouseMode(MouseTrack::XtMseBtn)
            } else {
                Mode::MouseMode(MouseTrack::NoTracking)
            }
        }
        b"?1003" => {
            if mode == &SetMode::DecSet {
                Mode::MouseMode(MouseTrack::XtMseAny)
            } else {
                Mode::MouseMode(MouseTrack::NoTracking)
            }
        }
        b"?1004" => Mode::XtMseWin(XtMseWin::new(mode)),
        // TODO: Implement this
        b"?1005" => {
            if mode == &SetMode::DecSet {
                Mode::MouseMode(MouseTrack::XtMseUtf)
            } else {
                Mode::MouseMode(MouseTrack::NoTracking)
            }
        }
        // TODO: Implement this
        b"?1006" => {
            if mode == &SetMode::DecSet {
                Mode::MouseMode(MouseTrack::XtMseSgr)
            } else {
                Mode::MouseMode(MouseTrack::NoTracking)
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
        b"?1015" => {
            if mode == &SetMode::DecSet {
                Mode::MouseMode(MouseTrack::XtMseUrXvt)
            } else {
                Mode::MouseMode(MouseTrack::NoTracking)
            }
        }
        // TODO: Implement this
        b"?1016" => {
            if mode == &SetMode::DecSet {
                Mode::MouseMode(MouseTrack::XtMseSgrPixels)
            } else {
                Mode::MouseMode(MouseTrack::NoTracking)
            }
        }
        b"?1049" => Mode::XtExtscrn(XtExtscrn::new(mode)),
        b"?2004" => Mode::BracketedPaste(RlBracket::new(mode)),
        _ => Mode::Unknown(params.to_vec()),
    }
}
