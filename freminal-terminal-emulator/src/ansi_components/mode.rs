use std::fmt;

use super::modes::{
    decawm::Decawm, decckm::Decckm, dectcem::Dectcem, rl_bracket::RlBracket, srm::Srm,
    xtextscrn::XtExtscrn, xtmsewin::XtMseWin, xtmsex11::XtMseX11,
};

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Eq, PartialEq, Default)]
pub enum SetMode {
    DecSet,
    #[default]
    DecRst,
}

#[derive(Eq, PartialEq, Debug)]
pub enum Mode {
    // Cursor keys mode
    // https://vt100.net/docs/vt100-ug/chapter3.html
    Decckm(Decckm),
    Decawm(Decawm),
    Dectem(Dectcem),
    XtExtscrn(XtExtscrn),
    XtMseWin(XtMseWin),
    XTMseX11(XtMseX11),
    BracketedPaste(RlBracket),
    Srm(Srm),
    Unknown(Vec<u8>),
}

#[derive(Debug, Eq, PartialEq, Default)]
pub struct TerminalModes {
    pub cursor_key: Decckm,
    pub bracketed_paste: RlBracket,
    pub focus_reporting: XtMseWin,
    pub send_receive_mode: Srm,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decckm(decckm) => write!(f, "{decckm}"),
            Self::Decawm(decawm) => write!(f, "{decawm}"),
            Self::Dectem(dectem) => write!(f, "{dectem}"),
            Self::Srm(srm) => write!(f, "{srm}"),
            Self::XTMseX11(xt_mse_x11) => write!(f, "{xt_mse_x11}"),
            Self::XtMseWin(xt_mse_win) => write!(f, "{xt_mse_win}"),
            Self::XtExtscrn(xt_extscrn) => write!(f, "{xt_extscrn}"),
            Self::BracketedPaste(bracketed_paste) => write!(f, "{bracketed_paste}"),
            Self::Unknown(params) => {
                let params_s = std::str::from_utf8(params)
                    .expect("parameter parsing should not allow non-utf8 characters here");
                f.write_fmt(format_args!("Unknown Mode({params_s})"))
            }
        }
    }
}

#[must_use]
pub fn terminal_mode_from_params(params: &[u8], mode: &SetMode) -> Mode {
    info!(
        "Terminal Mode: {:?}, {:?}",
        String::from_utf8(params.to_vec()),
        mode
    );
    match params {
        // https://vt100.net/docs/vt510-rm/DECCKM.html
        b"?1" => Mode::Decckm(Decckm::new(mode)),
        b"?7" => Mode::Decawm(Decawm::new(mode)),
        b"?12" => Mode::Srm(Srm::new(mode)),
        b"?25" => Mode::Dectem(Dectcem::new(mode)),
        b"?1000" => Mode::XTMseX11(XtMseX11::new(mode)),
        b"?1004" => Mode::XtMseWin(XtMseWin::new(mode)),
        b"?1049" => Mode::XtExtscrn(XtExtscrn::new(mode)),
        b"?2004" => Mode::BracketedPaste(RlBracket::new(mode)),
        _ => Mode::Unknown(params.to_vec()),
    }
}
