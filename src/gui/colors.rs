// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::fmt;

use eframe::egui::Color32;

pub const fn lookup_256_color_by_index(index: usize) -> (usize, usize, usize) {
    // https://stackoverflow.com/questions/69138165/how-to-get-the-rgb-values-of-a-256-color-palette-terminal-color
    match index {
        // standard colors 0 -15, as well as their bright counterparts 8-15
        // And the other values that map to them further up the color table
        1 => (128, 0, 0),
        2 => (0, 128, 0),
        3 => (128, 128, 0),
        4 => (0, 0, 128),
        5 => (128, 0, 128),
        6 => (0, 128, 128),
        7 => (192, 192, 192),
        8 | 244 => (128, 128, 128),
        9 | 196 => (255, 0, 0),
        10 | 46 => (0, 255, 0),
        11 | 226 => (255, 255, 0),
        12 | 21 => (0, 0, 255),
        13 | 201 => (255, 0, 255),
        14 | 51 => (0, 255, 255),
        15 | 231 => (255, 255, 255),
        // gray scale
        232..=255 => {
            let value = (2056 + 2570 * (index - 232)) / 256;

            (value, value, value)
        }
        // the blacks
        0 | 16 | 256.. => (0, 0, 0),
        // programtic colors
        _ => {
            let r = cube_component(index, 36);
            let g = cube_component(index, 6);
            let b = cube_component(index, 1);
            (r, g, b)
        }
    }
}

const fn cube_component(value: usize, modifier: usize) -> usize {
    let i = ((value - 16) / modifier) % 6;

    if i == 0 {
        0
    } else {
        (14135 + 10280 * i) / 256
    }
}

pub fn internal_color_to_egui(
    default_foreground_color: Color32,
    default_background_color: Color32,
    color: TerminalColor,
    make_faint: bool,
) -> Color32 {
    let color_before_faint = match color {
        TerminalColor::Default | TerminalColor::DefaultUnderlineColor => default_foreground_color,
        TerminalColor::DefaultBackground => default_background_color,
        TerminalColor::Black => Color32::from_rgb(0, 0, 0),
        TerminalColor::Red => Color32::from_rgb(205, 0, 0),
        TerminalColor::Green => Color32::from_rgb(0, 205, 0),
        TerminalColor::Yellow => Color32::from_rgb(205, 205, 0),
        TerminalColor::Blue => Color32::from_rgb(0, 0, 238),
        TerminalColor::Magenta => Color32::from_rgb(205, 0, 205),
        TerminalColor::Cyan => Color32::from_rgb(0, 205, 205),
        TerminalColor::White => Color32::from_rgb(229, 229, 229),
        TerminalColor::BrightYellow => Color32::from_rgb(255, 255, 0),
        TerminalColor::BrightRed => Color32::from_rgb(255, 0, 0),
        TerminalColor::BrightGreen => Color32::from_rgb(0, 255, 0),
        TerminalColor::BrightBlue => Color32::from_rgb(92, 92, 255),
        TerminalColor::BrightMagenta => Color32::from_rgb(255, 0, 255),
        TerminalColor::BrightCyan => Color32::from_rgb(0, 255, 255),
        TerminalColor::BrightWhite => Color32::from_rgb(255, 255, 255),
        TerminalColor::BrightBlack => Color32::from_rgb(127, 127, 127),
        TerminalColor::Custom(r, g, b) => Color32::from_rgb(r, g, b),
    };

    if make_faint {
        color_before_faint.gamma_multiply(0.5)
    } else {
        color_before_faint
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalColor {
    Default,
    DefaultBackground,
    DefaultUnderlineColor,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightYellow,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    Custom(u8, u8, u8),
}

impl fmt::Display for TerminalColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Default => "default",
            Self::Black => "black",
            Self::Red => "red",
            Self::Green => "green",
            Self::Yellow => "yellow",
            Self::Blue => "blue",
            Self::Magenta => "magenta",
            Self::Cyan => "cyan",
            Self::White => "white",
            Self::BrightYellow => "bright yellow",
            Self::BrightBlack => "bright black",
            Self::BrightRed => "bright red",
            Self::BrightGreen => "bright green",
            Self::BrightBlue => "bright blue",
            Self::BrightMagenta => "bright magenta",
            Self::BrightCyan => "bright cyan",
            Self::BrightWhite => "bright white",
            Self::DefaultUnderlineColor => "default underline color",
            Self::DefaultBackground => "default background",
            Self::Custom(r, g, b) => {
                return write!(f, "rgb({r}, {g}, {b})");
            }
        };

        f.write_str(s)
    }
}

impl std::str::FromStr for TerminalColor {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ret = match s {
            "default" => Self::Default,
            "default_background" => Self::DefaultBackground,
            "default_underline_color" => Self::DefaultUnderlineColor,
            "black" => Self::Black,
            "red" => Self::Red,
            "green" => Self::Green,
            "yellow" => Self::Yellow,
            "blue" => Self::Blue,
            "magenta" => Self::Magenta,
            "cyan" => Self::Cyan,
            "white" => Self::White,
            "bright yellow" => Self::BrightYellow,
            "bright black" => Self::BrightBlack,
            "bright red" => Self::BrightRed,
            "bright green" => Self::BrightGreen,
            "bright blue" => Self::BrightBlue,
            "bright magenta" => Self::BrightMagenta,
            "bright cyan" => Self::BrightCyan,
            "bright white" => Self::BrightWhite,
            _ => return Err(()),
        };
        Ok(ret)
    }
}