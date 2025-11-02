use freminal_common::colors::{cube_component, lookup_256_color_by_index, TerminalColor};
use proptest::prelude::*;
use std::str::FromStr;

/// ---------- Deterministic Unit Tests ----------

#[test]
fn lookup_standard_colors_complete() {
    // Standard colors 0–15
    assert_eq!(lookup_256_color_by_index(0), (0, 0, 0));
    assert_eq!(lookup_256_color_by_index(1), (128, 0, 0)); // dark red
    assert_eq!(lookup_256_color_by_index(2), (0, 128, 0)); // dark green
    assert_eq!(lookup_256_color_by_index(3), (128, 128, 0)); // dark yellow
    assert_eq!(lookup_256_color_by_index(4), (0, 0, 128)); // dark blue
    assert_eq!(lookup_256_color_by_index(5), (128, 0, 128)); // dark magenta
    assert_eq!(lookup_256_color_by_index(6), (0, 128, 128)); // dark cyan
    assert_eq!(lookup_256_color_by_index(7), (192, 192, 192)); // light gray
    assert_eq!(lookup_256_color_by_index(8), (128, 128, 128)); // gray
    assert_eq!(lookup_256_color_by_index(9), (255, 0, 0)); // bright red
    assert_eq!(lookup_256_color_by_index(10), (0, 255, 0)); // bright green
    assert_eq!(lookup_256_color_by_index(11), (255, 255, 0)); // bright yellow
    assert_eq!(lookup_256_color_by_index(12), (0, 0, 255)); // bright blue
    assert_eq!(lookup_256_color_by_index(13), (255, 0, 255)); // bright magenta
    assert_eq!(lookup_256_color_by_index(14), (0, 255, 255)); // bright cyan
    assert_eq!(lookup_256_color_by_index(15), (255, 255, 255)); // bright white

    // Verify alias values (same return path)
    assert_eq!(lookup_256_color_by_index(196), (255, 0, 0));
    assert_eq!(lookup_256_color_by_index(46), (0, 255, 0));
    assert_eq!(lookup_256_color_by_index(226), (255, 255, 0)); // alias for 11
    assert_eq!(lookup_256_color_by_index(21), (0, 0, 255));
    assert_eq!(lookup_256_color_by_index(201), (255, 0, 255));
    assert_eq!(lookup_256_color_by_index(51), (0, 255, 255));
    assert_eq!(lookup_256_color_by_index(231), (255, 255, 255));
    assert_eq!(lookup_256_color_by_index(244), (128, 128, 128)); // alias for 8
}

#[test]
fn lookup_alias_colors() {
    assert_eq!(lookup_256_color_by_index(196), (255, 0, 0));
    assert_eq!(lookup_256_color_by_index(46), (0, 255, 0));
    assert_eq!(lookup_256_color_by_index(21), (0, 0, 255));
    assert_eq!(lookup_256_color_by_index(244), (128, 128, 128));
}

#[test]
fn lookup_grayscale_range() {
    assert_eq!(lookup_256_color_by_index(232), (8, 8, 8));
    assert_eq!(lookup_256_color_by_index(255), (238, 238, 238));
}

#[test]
fn lookup_black_variants_and_out_of_range() {
    assert_eq!(lookup_256_color_by_index(0), (0, 0, 0));
    assert_eq!(lookup_256_color_by_index(16), (0, 0, 0));
    assert_eq!(lookup_256_color_by_index(300), (0, 0, 0));
}

#[test]
fn lookup_programmatic_color() {
    let idx = 40;
    let expected = (
        cube_component(idx, 36),
        cube_component(idx, 6),
        cube_component(idx, 1),
    );
    assert_eq!(lookup_256_color_by_index(idx), expected);
}

#[test]
fn cube_component_values() {
    assert_eq!(cube_component(16, 36), 0);
    assert_eq!(cube_component(52, 36), ((14135 + 10280) / 256));
    assert_eq!(cube_component(88, 36), ((14135 + 10280 * 2) / 256));
}

#[test]
fn default_colors_map_to_regular() {
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
fn display_predefined_colors() {
    // Standard colors
    assert_eq!(TerminalColor::Default.to_string(), "default");
    assert_eq!(TerminalColor::Black.to_string(), "black");
    assert_eq!(TerminalColor::Red.to_string(), "red");
    assert_eq!(TerminalColor::Green.to_string(), "green");
    assert_eq!(TerminalColor::Yellow.to_string(), "yellow");
    assert_eq!(TerminalColor::Blue.to_string(), "blue");
    assert_eq!(TerminalColor::Magenta.to_string(), "magenta");
    assert_eq!(TerminalColor::Cyan.to_string(), "cyan");
    assert_eq!(TerminalColor::White.to_string(), "white");

    // Bright variants
    assert_eq!(TerminalColor::BrightYellow.to_string(), "bright yellow");
    assert_eq!(TerminalColor::BrightBlack.to_string(), "bright black");
    assert_eq!(TerminalColor::BrightRed.to_string(), "bright red");
    assert_eq!(TerminalColor::BrightGreen.to_string(), "bright green");
    assert_eq!(TerminalColor::BrightBlue.to_string(), "bright blue");
    assert_eq!(TerminalColor::BrightMagenta.to_string(), "bright magenta");
    assert_eq!(TerminalColor::BrightCyan.to_string(), "bright cyan");
    assert_eq!(TerminalColor::BrightWhite.to_string(), "bright white");

    // Defaults and special
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

    // Custom RGB branch
    assert_eq!(
        TerminalColor::Custom(12, 34, 56).to_string(),
        "rgb(12, 34, 56)"
    );
}

#[test]
fn parse_all_valid_and_invalid_colors() {
    use std::str::FromStr;

    // Basic and bright colors
    assert_eq!(
        TerminalColor::from_str("default").unwrap(),
        TerminalColor::Default
    );
    assert_eq!(
        TerminalColor::from_str("black").unwrap(),
        TerminalColor::Black
    );
    assert_eq!(TerminalColor::from_str("red").unwrap(), TerminalColor::Red);
    assert_eq!(
        TerminalColor::from_str("green").unwrap(),
        TerminalColor::Green
    );
    assert_eq!(
        TerminalColor::from_str("yellow").unwrap(),
        TerminalColor::Yellow
    );
    assert_eq!(
        TerminalColor::from_str("blue").unwrap(),
        TerminalColor::Blue
    );
    assert_eq!(
        TerminalColor::from_str("magenta").unwrap(),
        TerminalColor::Magenta
    );
    assert_eq!(
        TerminalColor::from_str("cyan").unwrap(),
        TerminalColor::Cyan
    );
    assert_eq!(
        TerminalColor::from_str("white").unwrap(),
        TerminalColor::White
    );

    // Bright variants
    assert_eq!(
        TerminalColor::from_str("bright yellow").unwrap(),
        TerminalColor::BrightYellow
    );
    assert_eq!(
        TerminalColor::from_str("bright black").unwrap(),
        TerminalColor::BrightBlack
    );
    assert_eq!(
        TerminalColor::from_str("bright red").unwrap(),
        TerminalColor::BrightRed
    );
    assert_eq!(
        TerminalColor::from_str("bright green").unwrap(),
        TerminalColor::BrightGreen
    );
    assert_eq!(
        TerminalColor::from_str("bright blue").unwrap(),
        TerminalColor::BrightBlue
    );
    assert_eq!(
        TerminalColor::from_str("bright magenta").unwrap(),
        TerminalColor::BrightMagenta
    );
    assert_eq!(
        TerminalColor::from_str("bright cyan").unwrap(),
        TerminalColor::BrightCyan
    );
    assert_eq!(
        TerminalColor::from_str("bright white").unwrap(),
        TerminalColor::BrightWhite
    );

    // Defaults
    assert_eq!(
        TerminalColor::from_str("default_background").unwrap(),
        TerminalColor::DefaultBackground
    );
    assert_eq!(
        TerminalColor::from_str("default_underline_color").unwrap(),
        TerminalColor::DefaultUnderlineColor
    );
    assert_eq!(
        TerminalColor::from_str("default_cursor_color").unwrap(),
        TerminalColor::DefaultCursorColor
    );

    // ❌ Explicitly test error branch and check message
    let err = TerminalColor::from_str("unknown_value").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("Invalid color string"),
        "unexpected error message: {msg}"
    );
}

#[test]
fn manual_display_write_covers_all_paths() {
    use std::fmt::Write;

    // Explicitly test `f.write_str` branch (normal color)
    let color = TerminalColor::Yellow;
    let mut buf = String::new();
    write!(&mut buf, "{color}").unwrap();
    assert_eq!(buf, "yellow");

    // Explicitly test `return write!(...)` branch (Custom color)
    let color = TerminalColor::Custom(200, 150, 100);
    let mut buf = String::new();
    write!(&mut buf, "{color}").unwrap();
    assert_eq!(buf, "rgb(200, 150, 100)");

    // Cover Default variant again via direct fmt call (not .to_string)
    let color = TerminalColor::Default;
    let mut buf = String::new();
    write!(&mut buf, "{color}").unwrap();
    assert_eq!(buf, "default");
}

#[test]
fn display_custom_color() {
    let c = TerminalColor::Custom(123, 45, 67);
    assert_eq!(c.to_string(), "rgb(123, 45, 67)");
}

#[test]
fn parse_predefined_colors() {
    assert_eq!(TerminalColor::from_str("red").unwrap(), TerminalColor::Red);
    assert_eq!(
        TerminalColor::from_str("bright blue").unwrap(),
        TerminalColor::BrightBlue
    );
    assert_eq!(
        TerminalColor::from_str("default_background").unwrap(),
        TerminalColor::DefaultBackground
    );
}

#[test]
fn parse_invalid_color_fails() {
    assert!(TerminalColor::from_str("not_a_color").is_err());
}

proptest! {
    /// Grayscale 232–255: equal channels, monotonic increasing brightness.
    #[test]
    fn grayscale_continuity(index in 232usize..=254usize) {
        let (r1, g1, b1) = lookup_256_color_by_index(index);
        let (r2, _g2, _b2) = lookup_256_color_by_index(index + 1);

        prop_assert_eq!(r1, g1);
        prop_assert_eq!(g1, b1);
        prop_assert!(r2 >= r1);
        prop_assert!(r1 <= 255);
    }

    /// Cube components are always in range 0–255 and repeat every 6 steps.
    #[test]
    fn cube_component_range_and_cycle(
        value in 16usize..=230usize,
        modifier in prop::sample::select(vec![36usize, 6usize, 1usize])
    ) {
        let c = cube_component(value, modifier);
        prop_assert!(c <= 255);

        let next = cube_component(value + modifier * 6, modifier);
        // After one full 6-step cycle, the value repeats
        prop_assert_eq!(c, next);
    }

     #[test]
    fn cube_component_increases_within_block(
        base in 16usize..=196usize,
        modifier in prop::sample::select(vec![36usize, 6usize, 1usize])
    ) {
        let mut prev = cube_component(base, modifier);

        for step in 1..6 {
            let val_i = ((base + modifier * step - 16) / modifier) % 6;
            let val = cube_component(base + modifier * step, modifier);

            // Only enforce monotonicity when we haven’t wrapped around to i == 0
            if val_i != 0 && val >= prev {
                // OK, increasing as expected
            } else if val_i != 0 {
                prop_assert!(
                    val >= prev,
                    "Expected non-decreasing value before wrap: base={}, step={}, modifier={}",
                    base, step, modifier
                );
            }

            prev = val;
        }

        // The 6th increment should wrap to i == 0
        let wrap_i = ((base + modifier * 6 - 16) / modifier) % 6;
        let wrap = cube_component(base + modifier * 6, modifier);
        if wrap_i == 0 {
            prop_assert_eq!(
                wrap,
                0,
                "Expected wrap-to-zero at cube boundary: base={}, modifier={}",
                base,
                modifier
            );
        }
    }

    /// lookup_256_color_by_index produces valid RGB values in 0–255.
    #[test]
    fn lookup_always_within_rgb_bounds(index in 0usize..=300usize) {
        let (r, g, b) = lookup_256_color_by_index(index);
        prop_assert!(r <= 255 && g <= 255 && b <= 255);
    }

    /// Predefined color strings parse to valid enums, Display always returns non-empty.
    #[test]
    fn color_roundtrip(s in prop::sample::select(vec![
        "red", "blue", "bright yellow", "default_background", "white"
    ])) {
        let color = TerminalColor::from_str(s).unwrap();
        let out = color.to_string();
        prop_assert!(!out.trim().is_empty());
    }

    /// Custom RGB always formats correctly as "rgb(r, g, b)" and matches its components.
    #[test]
    fn custom_color_display_roundtrip(r in 0u8..=255, g in 0u8..=255, b in 0u8..=255) {
        let color = TerminalColor::Custom(r, g, b);
        let s = color.to_string();

        // Ensure it follows exact rgb(...) format
        let prefix = "rgb(";
        prop_assert!(s.starts_with(prefix));
        prop_assert!(s.ends_with(")"));

        // Parse numeric parts back
        let nums: Vec<u8> = s.trim_start_matches(prefix)
            .trim_end_matches(")")
            .split(',')
            .map(|x| x.trim().parse::<u8>().unwrap())
            .collect();

        prop_assert_eq!(nums, vec![r, g, b]);
    }
}
