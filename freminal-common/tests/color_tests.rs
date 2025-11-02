use freminal_common::colors::{cube_component, lookup_256_color_by_index, TerminalColor};
use proptest::prelude::*;
use std::str::FromStr;

/// ---------- Deterministic Unit Tests ----------

#[test]
fn lookup_standard_colors() {
    assert_eq!(lookup_256_color_by_index(1), (128, 0, 0));
    assert_eq!(lookup_256_color_by_index(4), (0, 0, 128));
    assert_eq!(lookup_256_color_by_index(7), (192, 192, 192));
    assert_eq!(lookup_256_color_by_index(9), (255, 0, 0));
    assert_eq!(lookup_256_color_by_index(15), (255, 255, 255));
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
    assert_eq!(TerminalColor::Red.to_string(), "red");
    assert_eq!(TerminalColor::BrightMagenta.to_string(), "bright magenta");
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
