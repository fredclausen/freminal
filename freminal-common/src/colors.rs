// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::fmt;

#[must_use]
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

#[must_use]
pub const fn cube_component(value: usize, modifier: usize) -> usize {
    let i = ((value - 16) / modifier) % 6;

    if i == 0 {
        0
    } else {
        (14135 + 10280 * i) / 256
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

#[test]
fn test_color_from_string() {
    let color = "default".parse::<TerminalColor>().unwrap();
    assert_eq!(color, TerminalColor::Default);

    let color = "default_background".parse::<TerminalColor>().unwrap();
    assert_eq!(color, TerminalColor::DefaultBackground);

    let color = "default_underline_color".parse::<TerminalColor>().unwrap();
    assert_eq!(color, TerminalColor::DefaultUnderlineColor);

    let color = "black".parse::<TerminalColor>().unwrap();
    assert_eq!(color, TerminalColor::Black);

    let color = "red".parse::<TerminalColor>().unwrap();
    assert_eq!(color, TerminalColor::Red);

    let color = "green".parse::<TerminalColor>().unwrap();
    assert_eq!(color, TerminalColor::Green);

    let color = "yellow".parse::<TerminalColor>().unwrap();
    assert_eq!(color, TerminalColor::Yellow);

    let color = "blue".parse::<TerminalColor>().unwrap();
    assert_eq!(color, TerminalColor::Blue);

    let color = "magenta".parse::<TerminalColor>().unwrap();
    assert_eq!(color, TerminalColor::Magenta);

    let color = "cyan".parse::<TerminalColor>().unwrap();
    assert_eq!(color, TerminalColor::Cyan);

    let color = "white".parse::<TerminalColor>().unwrap();
    assert_eq!(color, TerminalColor::White);

    let color = "bright yellow".parse::<TerminalColor>().unwrap();
    assert_eq!(color, TerminalColor::BrightYellow);

    let color = "bright black".parse::<TerminalColor>().unwrap();
    assert_eq!(color, TerminalColor::BrightBlack);

    let color = "bright red".parse::<TerminalColor>().unwrap();
    assert_eq!(color, TerminalColor::BrightRed);

    let color = "bright green".parse::<TerminalColor>().unwrap();
    assert_eq!(color, TerminalColor::BrightGreen);

    let color = "bright blue".parse::<TerminalColor>().unwrap();
    assert_eq!(color, TerminalColor::BrightBlue);

    let color = "bright magenta".parse::<TerminalColor>().unwrap();
    assert_eq!(color, TerminalColor::BrightMagenta);

    let color = "bright cyan".parse::<TerminalColor>().unwrap();
    assert_eq!(color, TerminalColor::BrightCyan);

    let color = "bright white".parse::<TerminalColor>().unwrap();
    assert_eq!(color, TerminalColor::BrightWhite);

    let color = "sucks".parse::<TerminalColor>();
    assert_eq!(color, Err(()));
}

#[test]
fn test_fmt_display() {
    let color = TerminalColor::Default;
    assert_eq!(format!("{color}"), "default");

    let color = TerminalColor::DefaultBackground;
    assert_eq!(format!("{color}"), "default background");

    let color = TerminalColor::DefaultUnderlineColor;
    assert_eq!(format!("{color}"), "default underline color");

    let color = TerminalColor::Black;
    assert_eq!(format!("{color}"), "black");

    let color = TerminalColor::Red;
    assert_eq!(format!("{color}"), "red");

    let color = TerminalColor::Green;
    assert_eq!(format!("{color}"), "green");

    let color = TerminalColor::Yellow;
    assert_eq!(format!("{color}"), "yellow");

    let color = TerminalColor::Blue;
    assert_eq!(format!("{color}"), "blue");

    let color = TerminalColor::Magenta;
    assert_eq!(format!("{color}"), "magenta");

    let color = TerminalColor::Cyan;
    assert_eq!(format!("{color}"), "cyan");

    let color = TerminalColor::White;
    assert_eq!(format!("{color}"), "white");

    let color = TerminalColor::BrightYellow;
    assert_eq!(format!("{color}"), "bright yellow");

    let color = TerminalColor::BrightBlack;
    assert_eq!(format!("{color}"), "bright black");

    let color = TerminalColor::BrightRed;
    assert_eq!(format!("{color}"), "bright red");

    let color = TerminalColor::BrightGreen;
    assert_eq!(format!("{color}"), "bright green");

    let color = TerminalColor::BrightBlue;
    assert_eq!(format!("{color}"), "bright blue");

    let color = TerminalColor::BrightMagenta;
    assert_eq!(format!("{color}"), "bright magenta");

    let color = TerminalColor::BrightCyan;
    assert_eq!(format!("{color}"), "bright cyan");

    let color = TerminalColor::BrightWhite;
    assert_eq!(format!("{color}"), "bright white");

    let color = TerminalColor::Custom(255, 255, 255);
    assert_eq!(format!("{color}"), "rgb(255, 255, 255)");
}
