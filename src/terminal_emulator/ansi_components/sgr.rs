#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectGraphicRendition {
    // NOTE: Non-exhaustive list
    Reset,
    Bold,
    Italic,
    Faint,
    SlowBlink,
    FastBlink,
    ReverseVideo,
    ResetReverseVideo,
    ResetBold,
    NormalIntensity,
    NotUnderlined,
    ForegroundBlack,
    ForegroundRed,
    ForegroundGreen,
    ForegroundYellow,
    ForegroundBlue,
    ForegroundMagenta,
    ForegroundCyan,
    ForegroundWhite,
    ForegroundCustom(usize, usize, usize),
    BackgroundCustom(usize, usize, usize),
    ForegroundBrightBlack,
    ForegroundBrightRed,
    ForegroundBrightGreen,
    ForegroundBrightYellow,
    ForegroundBrightBlue,
    ForegroundBrightMagenta,
    ForegroundBrightCyan,
    ForegroundBrightWhite,
    BackgroundBlack,
    BackgroundRed,
    BackgroundGreen,
    BackgroundYellow,
    BackgroundBlue,
    BackgroundMagenta,
    BackgroundCyan,
    BackgroundWhite,
    BackgroundBrightBlack,
    BackgroundBrightRed,
    BackgroundBrightGreen,
    BackgroundBrightYellow,
    BackgroundBrightBlue,
    BackgroundBrightMagenta,
    BackgroundBrightCyan,
    BackgroundBrightWhite,
    DefaultBackground,
    DefaultForeground,
    Unknown(usize),
}

impl SelectGraphicRendition {
    pub fn from_usize(val: usize) -> Self {
        match val {
            0 => Self::Reset,
            1 => Self::Bold,
            2 => Self::Faint,
            3 => Self::Italic,
            5 => Self::SlowBlink,
            6 => Self::FastBlink,
            7 => Self::ReverseVideo,
            21 => Self::ResetBold,
            22 => Self::NormalIntensity,
            24 => Self::NotUnderlined,
            27 => Self::ResetReverseVideo,
            30 => Self::ForegroundBlack,
            31 => Self::ForegroundRed,
            32 => Self::ForegroundGreen,
            33 => Self::ForegroundYellow,
            34 => Self::ForegroundBlue,
            35 => Self::ForegroundMagenta,
            36 => Self::ForegroundCyan,
            37 => Self::ForegroundWhite,
            38 => {
                error!("We shouldn't end up here! Setting custom foreground color to black");
                Self::ForegroundCustom(0, 0, 0)
            }
            39 => Self::DefaultForeground,
            40 => Self::BackgroundBlack,
            41 => Self::BackgroundRed,
            42 => Self::BackgroundGreen,
            43 => Self::BackgroundYellow,
            44 => Self::BackgroundBlue,
            45 => Self::BackgroundMagenta,
            46 => Self::BackgroundCyan,
            47 => Self::BackgroundWhite,
            48 => {
                error!("We shouldn't end up here! Setting custom background color to black");
                Self::ForegroundCustom(0, 0, 0)
            }
            49 => Self::DefaultBackground,
            90 => Self::ForegroundBrightBlack,
            91 => Self::ForegroundBrightRed,
            92 => Self::ForegroundBrightGreen,
            93 => Self::ForegroundBrightYellow,
            94 => Self::ForegroundBrightBlue,
            95 => Self::ForegroundBrightMagenta,
            96 => Self::ForegroundBrightCyan,
            97 => Self::ForegroundBrightWhite,
            100 => Self::BackgroundBrightBlack,
            101 => Self::BackgroundBrightRed,
            102 => Self::BackgroundBrightGreen,
            103 => Self::BackgroundBrightYellow,
            104 => Self::BackgroundBrightBlue,
            105 => Self::BackgroundBrightMagenta,
            106 => Self::BackgroundBrightCyan,
            107 => Self::BackgroundBrightWhite,
            _ => Self::Unknown(val),
        }
    }

    pub const fn from_usize_color(val: usize, r: usize, g: usize, b: usize) -> Self {
        match val {
            38 => Self::ForegroundCustom(r, g, b),
            48 => Self::BackgroundCustom(r, g, b),
            _ => Self::Unknown(val),
        }
    }
}
