// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use freminal_common::colors::TerminalColor;
use freminal_terminal_emulator::{
    ansi_components::modes::decawm::Decawm,
    format_tracker::{ranges_overlap, FormatTag, FormatTracker},
    state::{
        cursor::{CursorState, ReverseVideo, StateColors},
        fonts::FontWeight,
    },
};

fn basic_color_test_one(format_tracker: &FormatTracker) {
    let tags = format_tracker.tags();

    assert_eq!(
        tags,
        &[
            FormatTag {
                start: 0,
                end: 3,
                colors: StateColors::default(),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 3,
                end: 10,
                colors: StateColors::default().with_color(TerminalColor::Yellow),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 10,
                end: usize::MAX,
                colors: StateColors::default(),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
        ]
    );
}

fn basic_color_test_two(format_tracker: &FormatTracker) {
    let tags = format_tracker.tags();
    assert_eq!(
        tags,
        &[
            FormatTag {
                start: 0,
                end: 3,
                colors: StateColors::default(),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 3,
                end: 5,
                colors: StateColors::default().with_color(TerminalColor::Yellow),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 5,
                end: 7,
                colors: StateColors::default().with_color(TerminalColor::Blue),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 7,
                end: 10,
                colors: StateColors::default().with_color(TerminalColor::Yellow),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 10,
                end: usize::MAX,
                colors: StateColors::default(),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
        ]
    );
}

fn basic_color_test_three(format_tracker: &FormatTracker) {
    let tags = format_tracker.tags();
    assert_eq!(
        tags,
        &[
            FormatTag {
                start: 0,
                end: 3,
                colors: StateColors::default(),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 3,
                end: 5,
                colors: StateColors::default().with_color(TerminalColor::Yellow),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 5,
                end: 7,
                colors: StateColors::default().with_color(TerminalColor::Blue),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 7,
                end: 9,
                colors: StateColors::default().with_color(TerminalColor::Green),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 9,
                end: 10,
                colors: StateColors::default().with_color(TerminalColor::Yellow),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 10,
                end: usize::MAX,
                colors: StateColors::default(),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
        ]
    );
}

fn basic_color_test_four(format_tracker: &FormatTracker) {
    let tags = format_tracker.tags();
    assert_eq!(
        tags,
        &[
            FormatTag {
                start: 0,
                end: 3,
                colors: StateColors::default(),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 3,
                end: 5,
                colors: StateColors::default().with_color(TerminalColor::Yellow),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 5,
                end: 6,
                colors: StateColors::default().with_color(TerminalColor::Blue),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 6,
                end: 11,
                colors: StateColors::default().with_color(TerminalColor::Red),
                font_weight: FontWeight::Bold,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 11,
                end: usize::MAX,
                colors: StateColors::default(),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
        ]
    );
}

#[test]
fn basic_color_tracker_test() {
    let mut format_tracker = FormatTracker::new();
    let mut cursor_state = CursorState::default().with_color(TerminalColor::Yellow);

    format_tracker.push_range(&cursor_state, 3..10);
    basic_color_test_one(&format_tracker);

    cursor_state.colors.set_color(TerminalColor::Blue);
    format_tracker.push_range(&cursor_state, 5..7);
    basic_color_test_two(&format_tracker);

    cursor_state.colors.set_color(TerminalColor::Green);
    format_tracker.push_range(&cursor_state, 7..9);
    basic_color_test_three(&format_tracker);

    cursor_state.colors.set_color(TerminalColor::Red);
    cursor_state.font_weight = FontWeight::Bold;
    format_tracker.push_range(&cursor_state, 6..11);
    basic_color_test_four(&format_tracker);
}

#[test]
fn test_range_overlap() {
    assert!(ranges_overlap(5..10, 7..9));
    assert!(ranges_overlap(5..10, 8..12));
    assert!(ranges_overlap(5..10, 3..6));
    assert!(ranges_overlap(5..10, 2..12));
    assert!(!ranges_overlap(5..10, 10..12));
    assert!(!ranges_overlap(5..10, 0..5));
}

fn del_range_test_one(format_tracker: &FormatTracker) {
    assert_eq!(
        format_tracker.tags(),
        [
            FormatTag {
                start: 0,
                end: 8,
                colors: StateColors {
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::DefaultBackground,
                    underline_color: TerminalColor::DefaultUnderlineColor,
                    reverse_video: ReverseVideo::Off,
                },
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 8,
                end: 18,
                colors: StateColors {
                    color: TerminalColor::Red,
                    background_color: TerminalColor::DefaultBackground,
                    underline_color: TerminalColor::DefaultUnderlineColor,
                    reverse_video: ReverseVideo::Off,
                },
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 18,
                end: usize::MAX,
                colors: StateColors::default(),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            }
        ]
    );
}

fn del_range_test_two(format_tracker: &FormatTracker) {
    assert_eq!(
        format_tracker.tags(),
        [
            FormatTag {
                start: 0,
                end: 6,
                colors: StateColors {
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::DefaultBackground,
                    underline_color: TerminalColor::DefaultUnderlineColor,
                    reverse_video: ReverseVideo::Off,
                },
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 6,
                end: 16,
                colors: StateColors {
                    color: TerminalColor::Red,
                    background_color: TerminalColor::DefaultBackground,
                    underline_color: TerminalColor::DefaultUnderlineColor,
                    reverse_video: ReverseVideo::Off,
                },
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 16,
                end: usize::MAX,
                colors: StateColors::default(),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            }
        ]
    );
}

fn del_range_test_three(format_tracker: &FormatTracker) {
    assert_eq!(
        format_tracker.tags(),
        [
            FormatTag {
                start: 0,
                end: 4,
                colors: StateColors {
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::DefaultBackground,
                    underline_color: TerminalColor::DefaultUnderlineColor,
                    reverse_video: ReverseVideo::Off,
                },
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 4,
                end: 14,
                colors: StateColors {
                    color: TerminalColor::Red,
                    background_color: TerminalColor::DefaultBackground,
                    underline_color: TerminalColor::DefaultUnderlineColor,
                    reverse_video: ReverseVideo::Off,
                },
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 14,
                end: usize::MAX,
                colors: StateColors::default(),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            }
        ]
    );
}

fn del_range_test_four(format_tracker: &FormatTracker) {
    assert_eq!(
        format_tracker.tags(),
        [
            FormatTag {
                start: 0,
                end: 2,
                colors: StateColors {
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::DefaultBackground,
                    underline_color: TerminalColor::DefaultUnderlineColor,
                    reverse_video: ReverseVideo::Off,
                },
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 2,
                end: 9,
                colors: StateColors {
                    color: TerminalColor::Red,
                    background_color: TerminalColor::DefaultBackground,
                    underline_color: TerminalColor::DefaultUnderlineColor,
                    reverse_video: ReverseVideo::Off,
                },
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 9,
                end: usize::MAX,
                colors: StateColors::default(),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            }
        ]
    );
}

#[test]
fn test_format_tracker_del_range() {
    let mut format_tracker = FormatTracker::new();
    let mut cursor = CursorState::default().with_color(TerminalColor::Blue);
    format_tracker.push_range(&cursor, 0..10);
    cursor.colors.color = TerminalColor::Red;
    format_tracker.push_range(&cursor, 10..20);

    format_tracker.delete_range(0..2).unwrap();
    del_range_test_one(&format_tracker);

    format_tracker.delete_range(2..4).unwrap();
    del_range_test_two(&format_tracker);

    format_tracker.delete_range(4..6).unwrap();
    del_range_test_three(&format_tracker);

    format_tracker.delete_range(2..7).unwrap();
    del_range_test_four(&format_tracker);
}

fn range_adjustment_test_one(format_tracker: &FormatTracker) {
    assert_eq!(
        format_tracker.tags(),
        [
            FormatTag {
                start: 0,
                end: 5,
                colors: StateColors {
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::DefaultBackground,
                    underline_color: TerminalColor::DefaultUnderlineColor,
                    reverse_video: ReverseVideo::Off,
                },
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 5,
                end: 10,
                colors: StateColors {
                    color: TerminalColor::Red,
                    background_color: TerminalColor::DefaultBackground,
                    underline_color: TerminalColor::DefaultUnderlineColor,
                    reverse_video: ReverseVideo::Off,
                },
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 10,
                end: usize::MAX,
                colors: StateColors::default(),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
        ]
    );
}

fn range_adjustment_test_two(format_tracker: &FormatTracker) {
    assert_eq!(
        format_tracker.tags(),
        [
            FormatTag {
                start: 0,
                end: 8,
                colors: StateColors {
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::DefaultBackground,
                    underline_color: TerminalColor::DefaultUnderlineColor,
                    reverse_video: ReverseVideo::Off,
                },
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 8,
                end: 13,
                colors: StateColors {
                    color: TerminalColor::Red,
                    background_color: TerminalColor::DefaultBackground,
                    underline_color: TerminalColor::DefaultUnderlineColor,
                    reverse_video: ReverseVideo::Off,
                },
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 13,
                end: usize::MAX,
                colors: StateColors::default(),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
        ]
    );
}

fn range_adjustment_test_three(format_tracker: &FormatTracker) {
    assert_eq!(
        format_tracker.tags(),
        [
            FormatTag {
                start: 0,
                end: 8,
                colors: StateColors {
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::DefaultBackground,
                    underline_color: TerminalColor::DefaultUnderlineColor,
                    reverse_video: ReverseVideo::Off,
                },
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 8,
                end: 15,
                colors: StateColors {
                    color: TerminalColor::Red,
                    background_color: TerminalColor::DefaultBackground,
                    underline_color: TerminalColor::DefaultUnderlineColor,
                    reverse_video: ReverseVideo::Off,
                },
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 15,
                end: usize::MAX,
                colors: StateColors::default(),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
        ]
    );
}

#[test]
fn test_range_adjustment() {
    let mut format_tracker = FormatTracker::new();
    let mut cursor = CursorState::default().with_color(TerminalColor::Blue);

    format_tracker.push_range(&cursor, 0..5);
    cursor.colors.color = TerminalColor::Red;
    format_tracker.push_range(&cursor, 5..10);
    range_adjustment_test_one(&format_tracker);

    // This should extend the first section, and push all the ones after
    format_tracker.push_range_adjustment(0..3);
    range_adjustment_test_two(&format_tracker);

    // Should have no effect as we're in the last range
    // Repeat the second test
    format_tracker.push_range_adjustment(15..50);
    range_adjustment_test_two(&format_tracker);

    // And for good measure, check something in the middle
    // This should not touch the first segment, extend the second, and move the third forward
    format_tracker.push_range_adjustment(10..12);
    range_adjustment_test_three(&format_tracker);
}
