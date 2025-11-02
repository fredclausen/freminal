// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use freminal_common::colors::{cube_component, lookup_256_color_by_index, TerminalColor};
use proptest::prelude::*;
use std::fmt::Write;
use std::str::FromStr;

//
// ---------- Deterministic Unit Tests ----------
//

#[test]
fn lookup_standard_colors_complete() {
    // Standard colors 0–15
    assert_eq!(lookup_256_color_by_index(0), (0, 0, 0));
    assert_eq!(lookup_256_color_by_index(1), (128, 0, 0));
    assert_eq!(lookup_256_color_by_index(2), (0, 128, 0));
    assert_eq!(lookup_256_color_by_index(3), (128, 128, 0));
    assert_eq!(lookup_256_color_by_index(4), (0, 0, 128));
    assert_eq!(lookup_256_color_by_index(5), (128, 0, 128));
    assert_eq!(lookup_256_color_by_index(6), (0, 128, 128));
    assert_eq!(lookup_256_color_by_index(7), (192, 192, 192));
    assert_eq!(lookup_256_color_by_index(8), (128, 128, 128));
    assert_eq!(lookup_256_color_by_index(9), (255, 0, 0));
    assert_eq!(lookup_256_color_by_index(10), (0, 255, 0));
    assert_eq!(lookup_256_color_by_index(11), (255, 255, 0));
    assert_eq!(lookup_256_color_by_index(12), (0, 0, 255));
    assert_eq!(lookup_256_color_by_index(13), (255, 0, 255));
    assert_eq!(lookup_256_color_by_index(14), (0, 255, 255));
    assert_eq!(lookup_256_color_by_index(15), (255, 255, 255));

    // Aliases
    assert_eq!(lookup_256_color_by_index(244), (128, 128, 128));
    assert_eq!(lookup_256_color_by_index(196), (255, 0, 0));
    assert_eq!(lookup_256_color_by_index(46), (0, 255, 0));
    assert_eq!(lookup_256_color_by_index(226), (255, 255, 0));
    assert_eq!(lookup_256_color_by_index(21), (0, 0, 255));
    assert_eq!(lookup_256_color_by_index(201), (255, 0, 255));
    assert_eq!(lookup_256_color_by_index(51), (0, 255, 255));
    assert_eq!(lookup_256_color_by_index(231), (255, 255, 255));
}

#[test]
fn lookup_grayscale_range() {
    assert_eq!(lookup_256_color_by_index(232), (8, 8, 8));
    assert_eq!(lookup_256_color_by_index(255), (238, 238, 238));
}

#[test]
fn lookup_black_and_out_of_range() {
    assert_eq!(lookup_256_color_by_index(0), (0, 0, 0));
    assert_eq!(lookup_256_color_by_index(16), (0, 0, 0));
    assert_eq!(lookup_256_color_by_index(300), (0, 0, 0));
}

#[test]
fn lookup_programmatic_color_branch() {
    let idx = 40;
    let expected = (
        cube_component(idx, 36),
        cube_component(idx, 6),
        cube_component(idx, 1),
    );
    assert_eq!(lookup_256_color_by_index(idx), expected);
}

#[test]
fn cube_component_values_basic() {
    assert_eq!(cube_component(16, 36), 0);
    assert_eq!(cube_component(52, 36), ((14135 + 10280) / 256));
    assert_eq!(cube_component(88, 36), ((14135 + 10280 * 2) / 256));
}

#[test]
fn default_colors_to_regular() {
    assert_eq!(
        TerminalColor::Default.default_to_regular(),
        TerminalColor::White
    );
    assert_eq!(
        TerminalColor::DefaultUnderlineColor.default_to_regular(),
        TerminalColor::White
    );
    assert_eq!(
        TerminalColor::DefaultCursorColor.default_to_regular(),
        TerminalColor::White
    );
    assert_eq!(
        TerminalColor::DefaultBackground.default_to_regular(),
        TerminalColor::Black
    );
    assert_eq!(TerminalColor::Red.default_to_regular(), TerminalColor::Red);
}

#[test]
fn display_predefined_colors_full() {
    // Standard
    assert_eq!(TerminalColor::Default.to_string(), "default");
    assert_eq!(TerminalColor::Black.to_string(), "black");
    assert_eq!(TerminalColor::Red.to_string(), "red");
    assert_eq!(TerminalColor::Green.to_string(), "green");
    assert_eq!(TerminalColor::Yellow.to_string(), "yellow");
    assert_eq!(TerminalColor::Blue.to_string(), "blue");
    assert_eq!(TerminalColor::Magenta.to_string(), "magenta");
    assert_eq!(TerminalColor::Cyan.to_string(), "cyan");
    assert_eq!(TerminalColor::White.to_string(), "white");

    // Bright
    assert_eq!(TerminalColor::BrightYellow.to_string(), "bright yellow");
    assert_eq!(TerminalColor::BrightBlack.to_string(), "bright black");
    assert_eq!(TerminalColor::BrightRed.to_string(), "bright red");
    assert_eq!(TerminalColor::BrightGreen.to_string(), "bright green");
    assert_eq!(TerminalColor::BrightBlue.to_string(), "bright blue");
    assert_eq!(TerminalColor::BrightMagenta.to_string(), "bright magenta");
    assert_eq!(TerminalColor::BrightCyan.to_string(), "bright cyan");
    assert_eq!(TerminalColor::BrightWhite.to_string(), "bright white");

    // Defaults
    assert_eq!(
        TerminalColor::DefaultUnderlineColor.to_string(),
        "default underline color"
    );
    assert_eq!(
        TerminalColor::DefaultBackground.to_string(),
        "default background"
    );
    assert_eq!(
        TerminalColor::DefaultCursorColor.to_string(),
        "default cursor color"
    );

    // Custom
    assert_eq!(
        TerminalColor::Custom(12, 34, 56).to_string(),
        "rgb(12, 34, 56)"
    );
}

#[test]
fn parse_all_valid_and_invalid_colors() {
    // All valid mappings
    let pairs = [
        ("default", TerminalColor::Default),
        ("default_background", TerminalColor::DefaultBackground),
        (
            "default_underline_color",
            TerminalColor::DefaultUnderlineColor,
        ),
        ("default_cursor_color", TerminalColor::DefaultCursorColor),
        ("black", TerminalColor::Black),
        ("red", TerminalColor::Red),
        ("green", TerminalColor::Green),
        ("yellow", TerminalColor::Yellow),
        ("blue", TerminalColor::Blue),
        ("magenta", TerminalColor::Magenta),
        ("cyan", TerminalColor::Cyan),
        ("white", TerminalColor::White),
        ("bright yellow", TerminalColor::BrightYellow),
        ("bright black", TerminalColor::BrightBlack),
        ("bright red", TerminalColor::BrightRed),
        ("bright green", TerminalColor::BrightGreen),
        ("bright blue", TerminalColor::BrightBlue),
        ("bright magenta", TerminalColor::BrightMagenta),
        ("bright cyan", TerminalColor::BrightCyan),
        ("bright white", TerminalColor::BrightWhite),
    ];

    for (name, expected) in pairs {
        assert_eq!(TerminalColor::from_str(name).unwrap(), expected);
    }

    // Invalid input hits Err branch
    let err = TerminalColor::from_str("unknown_color").unwrap_err();
    assert!(err.to_string().contains("Invalid color string"));
}

#[test]
fn manual_display_write_covers_all_paths() {
    let mut buf = String::new();

    // Normal write_str path
    write!(&mut buf, "{}", TerminalColor::Yellow).unwrap();
    assert_eq!(buf, "yellow");
    buf.clear();

    // Return write!() path (Custom)
    write!(&mut buf, "{}", TerminalColor::Custom(200, 150, 100)).unwrap();
    assert_eq!(buf, "rgb(200, 150, 100)");
    buf.clear();

    // Default variant again
    write!(&mut buf, "{}", TerminalColor::Default).unwrap();
    assert_eq!(buf, "default");
}

//
// ---------- Property-Based Tests ----------
//

proptest! {
    #[test]
    fn grayscale_monotonic(index in 232usize..=254usize) {
        let (r1, g1, b1) = lookup_256_color_by_index(index);
        let (r2, _g2, _b2) = lookup_256_color_by_index(index + 1);

        prop_assert_eq!(r1, g1);
        prop_assert_eq!(g1, b1);
        prop_assert!(r2 >= r1);
    }

    #[test]
    fn cube_component_cycles(value in 16usize..=230usize, modifier in prop::sample::select(vec![36usize, 6usize, 1usize])) {
        let c = cube_component(value, modifier);
        prop_assert!(c <= 255);

        let wrap = cube_component(value + modifier * 6, modifier);
        prop_assert_eq!(wrap, c);
    }

    #[test]
    fn lookup_rgb_bounds(index in 0usize..=300usize) {
        let (r, g, b) = lookup_256_color_by_index(index);
        prop_assert!(r <= 255 && g <= 255 && b <= 255);
    }

    #[test]
    fn custom_color_display_roundtrip(r in 0u8..=255, g in 0u8..=255, b in 0u8..=255) {
        let color = TerminalColor::Custom(r, g, b);
        let text = color.to_string();
        prop_assert!(text.starts_with("rgb(") && text.ends_with(")"));

        let parts: Vec<u8> = text.trim_start_matches("rgb(")
            .trim_end_matches(")")
            .split(',')
            .map(|p| p.trim().parse::<u8>().unwrap())
            .collect();

        prop_assert_eq!(parts, vec![r, g, b]);
    }
}
