use crate::terminal_emulator::TerminalColor;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectGraphicRendition {
    // NOTE: Non-exhaustive list
    Reset,
    Bold,
    Italic,
    Underline,
    Faint,
    SlowBlink,
    FastBlink,
    ReverseVideo,
    ResetReverseVideo,
    ResetBold,
    NormalIntensity,
    NotUnderlined,
    Foreground(TerminalColor),
    Background(TerminalColor),
    Unknown(usize),
}

impl SelectGraphicRendition {
    pub fn from_usize(val: usize) -> Self {
        match val {
            0 => Self::Reset,
            1 => Self::Bold,
            2 => Self::Faint,
            3 => Self::Italic,
            4 => Self::Underline,
            5 => Self::SlowBlink,
            6 => Self::FastBlink,
            7 => Self::ReverseVideo,
            21 => Self::ResetBold,
            22 => Self::NormalIntensity,
            24 => Self::NotUnderlined,
            27 => Self::ResetReverseVideo,
            30 | 39 => Self::Foreground(TerminalColor::Black),
            31 => Self::Foreground(TerminalColor::Red),
            32 => Self::Foreground(TerminalColor::Green),
            33 => Self::Foreground(TerminalColor::Yellow),
            34 => Self::Foreground(TerminalColor::Blue),
            35 => Self::Foreground(TerminalColor::Magenta),
            36 => Self::Foreground(TerminalColor::Cyan),
            37 => Self::Foreground(TerminalColor::White),
            38 => {
                error!("This is a custom foreground color. We shouldn't end up here! Setting custom foreground color to black");
                Self::Foreground(TerminalColor::Black)
            }
            40 => Self::Background(TerminalColor::Black),
            41 => Self::Background(TerminalColor::Red),
            42 => Self::Background(TerminalColor::Green),
            43 => Self::Background(TerminalColor::Yellow),
            44 => Self::Background(TerminalColor::Blue),
            45 => Self::Background(TerminalColor::Magenta),
            46 => Self::Background(TerminalColor::Cyan),
            47 => Self::Background(TerminalColor::White),
            48 => {
                error!("This is a custom background color. We shouldn't end up here! Setting custom background color to black");
                Self::Background(TerminalColor::DefaultBackground)
            }
            49 => Self::Background(TerminalColor::DefaultBackground),
            90 => Self::Foreground(TerminalColor::BrightBlack),
            91 => Self::Foreground(TerminalColor::BrightRed),
            92 => Self::Foreground(TerminalColor::BrightGreen),
            93 => Self::Foreground(TerminalColor::BrightYellow),
            94 => Self::Foreground(TerminalColor::BrightBlue),
            95 => Self::Foreground(TerminalColor::BrightMagenta),
            96 => Self::Foreground(TerminalColor::BrightCyan),
            97 => Self::Foreground(TerminalColor::BrightWhite),
            100 => Self::Background(TerminalColor::BrightBlack),
            101 => Self::Background(TerminalColor::BrightRed),
            102 => Self::Background(TerminalColor::BrightGreen),
            103 => Self::Background(TerminalColor::BrightYellow),
            104 => Self::Background(TerminalColor::BrightBlue),
            105 => Self::Background(TerminalColor::BrightMagenta),
            106 => Self::Background(TerminalColor::BrightCyan),
            107 => Self::Background(TerminalColor::BrightWhite),
            _ => Self::Unknown(val),
        }
    }

    pub fn from_usize_color(val: usize, r: usize, g: usize, b: usize) -> Self {
        let r = u8::try_from(r).unwrap();
        let g = u8::try_from(g).unwrap();
        let b = u8::try_from(b).unwrap();

        match val {
            38 => Self::Foreground(TerminalColor::Custom(r, g, b)),
            48 => Self::Background(TerminalColor::Custom(r, g, b)),
            _ => Self::Unknown(val),
        }
    }
}
