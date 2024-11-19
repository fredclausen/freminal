use std::fmt;

#[derive(Eq, PartialEq)]
pub enum Mode {
    // Cursor keys mode
    // https://vt100.net/docs/vt100-ug/chapter3.html
    Decckm,
    Decawm,
    Dectem,
    XtExtscrn,
    XtMseWin,
    BracketedPaste,
    Unknown(Vec<u8>),
}

/// Cursor Key Mode (DECCKM)
#[derive(Eq, PartialEq, Debug, Default, Clone)]
pub enum Decckm {
    #[default]
    /// Cursor keys send ANSI control codes
    Ansi,
    /// Cursor keys send application control codes
    Application,
}

/// Autowrap Mode (DECAWM)
#[derive(Eq, PartialEq, Debug, Default, Clone)]
pub enum Decawm {
    #[default]
    /// Cursor does not wrap to the next line
    NoAutoWrap,
    /// Cursor wraps to the next line
    AutoWrap,
}

/// Bracketed Paste Mode (DEC 2004)
#[derive(Debug, Default, Eq, PartialEq, Clone)]
pub enum BracketedPaste {
    #[default]
    /// Bracketed paste mode is disabled
    Disabled,
    /// Bracketed paste mode is enabled and the terminal will send ESC [200~ and ESC [201~ around pasted text
    Enabled,
}

/// Show cursor (DECTCEM)
#[derive(Debug, Eq, PartialEq, Default)]
pub enum Dectem {
    #[default]
    Show,
    Hide,
}

/// Alternate Screen (`XT_EXTSCRN`)
#[derive(Debug, Eq, PartialEq, Default)]
pub enum XtExtscrn {
    #[default]
    Primary,
    Alternate,
}

/// Focus reporting mode (`XT_MSE_WIN`)
#[derive(Debug, Eq, PartialEq, Default)]
pub enum XtMseWin {
    #[default]
    /// Focus reporting is disabled
    Disabled,
    /// Focus reporting is enabled
    Enabled,
}

#[derive(Debug, Eq, PartialEq, Default)]
pub struct TerminalModes {
    pub cursor_key: Decckm,
    pub bracketed_paste: BracketedPaste,
    pub focus_reporting: XtMseWin,
}

impl fmt::Debug for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decckm => f.write_str("Decckm"),
            Self::Decawm => f.write_str("Decawm"),
            Self::Dectem => f.write_str("Dectem"),
            Self::XtExtscrn => f.write_str("XtExtscrn"),
            Self::XtMseWin => f.write_str("XtMseWin"),
            Self::BracketedPaste => f.write_str("BracketedPasteMode"),
            Self::Unknown(params) => {
                let params_s = std::str::from_utf8(params)
                    .expect("parameter parsing should not allow non-utf8 characters here");
                f.write_fmt(format_args!("Unknown({params_s})"))
            }
        }
    }
}

#[must_use]
pub fn terminal_mode_from_params(params: &[u8]) -> Mode {
    match params {
        // https://vt100.net/docs/vt510-rm/DECCKM.html
        b"?1" => Mode::Decckm,
        b"?7" => Mode::Decawm,
        b"?25" => Mode::Dectem,
        b"?1004" => Mode::XtMseWin,
        b"?1049" => Mode::XtExtscrn,
        b"?2004" => Mode::BracketedPaste,
        _ => Mode::Unknown(params.to_vec()),
    }
}
