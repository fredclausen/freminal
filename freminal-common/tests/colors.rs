// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.
use test_log::test;

use freminal_common::colors::{cube_component, TerminalColor};

#[test]
fn test_cube_component() {
    let result = cube_component(16, 36);
    assert_eq!(result, 0);

    let result = cube_component(16, 6);
    assert_eq!(result, 0);

    let result = cube_component(16, 1);
    assert_eq!(result, 0);

    let result = cube_component(100, 36);
    assert_eq!(result, 135);

    let result = cube_component(100, 6);
    assert_eq!(result, 135);

    let result = cube_component(100, 1);
    assert_eq!(result, 0);
}

fn generate_256_color_table() -> Vec<(usize, usize, usize)> {
    vec![
        (0, 0, 0),
        (128, 0, 0),
        (0, 128, 0),
        (128, 128, 0),
        (0, 0, 128),
        (128, 0, 128),
        (0, 128, 128),
        (192, 192, 192),
        (128, 128, 128),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (0, 0, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
        (0, 0, 0),
        (0, 0, 95),
        (0, 0, 135),
        (0, 0, 175),
        (0, 0, 215),
        (0, 0, 255),
        (0, 95, 0),
        (0, 95, 95),
        (0, 95, 135),
        (0, 95, 175),
        (0, 95, 215),
        (0, 95, 255),
        (0, 135, 0),
        (0, 135, 95),
        (0, 135, 135),
        (0, 135, 175),
        (0, 135, 215),
        (0, 135, 255),
        (0, 175, 0),
        (0, 175, 95),
        (0, 175, 135),
        (0, 175, 175),
        (0, 175, 215),
        (0, 175, 255),
        (0, 215, 0),
        (0, 215, 95),
        (0, 215, 135),
        (0, 215, 175),
        (0, 215, 215),
        (0, 215, 255),
        (0, 255, 0),
        (0, 255, 95),
        (0, 255, 135),
        (0, 255, 175),
        (0, 255, 215),
        (0, 255, 255),
        (95, 0, 0),
        (95, 0, 95),
        (95, 0, 135),
        (95, 0, 175),
        (95, 0, 215),
        (95, 0, 255),
        (95, 95, 0),
        (95, 95, 95),
        (95, 95, 135),
        (95, 95, 175),
        (95, 95, 215),
        (95, 95, 255),
        (95, 135, 0),
        (95, 135, 95),
        (95, 135, 135),
        (95, 135, 175),
        (95, 135, 215),
        (95, 135, 255),
        (95, 175, 0),
        (95, 175, 95),
        (95, 175, 135),
        (95, 175, 175),
        (95, 175, 215),
        (95, 175, 255),
        (95, 215, 0),
        (95, 215, 95),
        (95, 215, 135),
        (95, 215, 175),
        (95, 215, 215),
        (95, 215, 255),
        (95, 255, 0),
        (95, 255, 95),
        (95, 255, 135),
        (95, 255, 175),
        (95, 255, 215),
        (95, 255, 255),
        (135, 0, 0),
        (135, 0, 95),
        (135, 0, 135),
        (135, 0, 175),
        (135, 0, 215),
        (135, 0, 255),
        (135, 95, 0),
        (135, 95, 95),
        (135, 95, 135),
        (135, 95, 175),
        (135, 95, 215),
        (135, 95, 255),
        (135, 135, 0),
        (135, 135, 95),
        (135, 135, 135),
        (135, 135, 175),
        (135, 135, 215),
        (135, 135, 255),
        (135, 175, 0),
        (135, 175, 95),
        (135, 175, 135),
        (135, 175, 175),
        (135, 175, 215),
        (135, 175, 255),
        (135, 215, 0),
        (135, 215, 95),
        (135, 215, 135),
        (135, 215, 175),
        (135, 215, 215),
        (135, 215, 255),
        (135, 255, 0),
        (135, 255, 95),
        (135, 255, 135),
        (135, 255, 175),
        (135, 255, 215),
        (135, 255, 255),
        (175, 0, 0),
        (175, 0, 95),
        (175, 0, 135),
        (175, 0, 175),
        (175, 0, 215),
        (175, 0, 255),
        (175, 95, 0),
        (175, 95, 95),
        (175, 95, 135),
        (175, 95, 175),
        (175, 95, 215),
        (175, 95, 255),
        (175, 135, 0),
        (175, 135, 95),
        (175, 135, 135),
        (175, 135, 175),
        (175, 135, 215),
        (175, 135, 255),
        (175, 175, 0),
        (175, 175, 95),
        (175, 175, 135),
        (175, 175, 175),
        (175, 175, 215),
        (175, 175, 255),
        (175, 215, 0),
        (175, 215, 95),
        (175, 215, 135),
        (175, 215, 175),
        (175, 215, 215),
        (175, 215, 255),
        (175, 255, 0),
        (175, 255, 95),
        (175, 255, 135),
        (175, 255, 175),
        (175, 255, 215),
        (175, 255, 255),
        (215, 0, 0),
        (215, 0, 95),
        (215, 0, 135),
        (215, 0, 175),
        (215, 0, 215),
        (215, 0, 255),
        (215, 95, 0),
        (215, 95, 95),
        (215, 95, 135),
        (215, 95, 175),
        (215, 95, 215),
        (215, 95, 255),
        (215, 135, 0),
        (215, 135, 95),
        (215, 135, 135),
        (215, 135, 175),
        (215, 135, 215),
        (215, 135, 255),
        (215, 175, 0),
        (215, 175, 95),
        (215, 175, 135),
        (215, 175, 175),
        (215, 175, 215),
        (215, 175, 255),
        (215, 215, 0),
        (215, 215, 95),
        (215, 215, 135),
        (215, 215, 175),
        (215, 215, 215),
        (215, 215, 255),
        (215, 255, 0),
        (215, 255, 95),
        (215, 255, 135),
        (215, 255, 175),
        (215, 255, 215),
        (215, 255, 255),
        (255, 0, 0),
        (255, 0, 95),
        (255, 0, 135),
        (255, 0, 175),
        (255, 0, 215),
        (255, 0, 255),
        (255, 95, 0),
        (255, 95, 95),
        (255, 95, 135),
        (255, 95, 175),
        (255, 95, 215),
        (255, 95, 255),
        (255, 135, 0),
        (255, 135, 95),
        (255, 135, 135),
        (255, 135, 175),
        (255, 135, 215),
        (255, 135, 255),
        (255, 175, 0),
        (255, 175, 95),
        (255, 175, 135),
        (255, 175, 175),
        (255, 175, 215),
        (255, 175, 255),
        (255, 215, 0),
        (255, 215, 95),
        (255, 215, 135),
        (255, 215, 175),
        (255, 215, 215),
        (255, 215, 255),
        (255, 255, 0),
        (255, 255, 95),
        (255, 255, 135),
        (255, 255, 175),
        (255, 255, 215),
        (255, 255, 255),
        (8, 8, 8),
        (18, 18, 18),
        (28, 28, 28),
        (38, 38, 38),
        (48, 48, 48),
        (58, 58, 58),
        (68, 68, 68),
        (78, 78, 78),
        (88, 88, 88),
        (98, 98, 98),
        (108, 108, 108),
        (118, 118, 118),
        (128, 128, 128),
        (138, 138, 138),
        (148, 148, 148),
        (158, 158, 158),
        (168, 168, 168),
        (178, 178, 178),
        (188, 188, 188),
        (198, 198, 198),
        (208, 208, 208),
        (218, 218, 218),
        (228, 228, 228),
        (238, 238, 238),
    ]
}

#[test]
fn test_lookup_256_color_by_index() {
    let expected = generate_256_color_table();
    for (index, color) in expected.iter().enumerate() {
        let result = freminal_common::colors::lookup_256_color_by_index(index);
        assert_eq!(result, *color);
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
    assert!(color.is_err());

    let color = "default_cursor_color".parse::<TerminalColor>().unwrap();
    assert_eq!(color, TerminalColor::DefaultCursorColor);
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

    let color = TerminalColor::DefaultCursorColor;
    assert_eq!(format!("{color}"), "default cursor color");

    let color = TerminalColor::Custom(255, 255, 255);
    assert_eq!(format!("{color}"), "rgb(255, 255, 255)");
}

#[test]
fn default_to_regular() {
    let color = TerminalColor::Default;
    let result = color.default_to_regular();
    assert_eq!(result, TerminalColor::White);

    let color = TerminalColor::DefaultBackground;
    let result = color.default_to_regular();
    assert_eq!(result, TerminalColor::Black);

    let color = TerminalColor::DefaultUnderlineColor;
    let result = color.default_to_regular();
    assert_eq!(result, TerminalColor::White);

    let color = TerminalColor::DefaultCursorColor;
    let result = color.default_to_regular();
    assert_eq!(result, TerminalColor::White);

    let color = TerminalColor::Black;
    let result = color.default_to_regular();
    assert_eq!(result, TerminalColor::Black);
}
