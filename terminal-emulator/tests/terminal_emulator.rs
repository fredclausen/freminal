// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use freminal_common::colors::TerminalColor;
use terminal_emulator::{
    ansi_components::mode::Decawm, format_tracker::FormatTag,
    interface::split_format_data_for_scrollback, state::fonts::FontWeight,
};

fn get_tags() -> Vec<FormatTag> {
    vec![
        FormatTag {
            start: 0,
            end: 5,
            color: TerminalColor::Blue,
            background_color: TerminalColor::Black,
            underline_color: TerminalColor::DefaultUnderlineColor,
            font_weight: FontWeight::Normal,
            font_decorations: Vec::new(),
            line_wrap_mode: Decawm::default(),
        },
        FormatTag {
            start: 5,
            end: 7,
            color: TerminalColor::Red,
            background_color: TerminalColor::Black,
            underline_color: TerminalColor::DefaultUnderlineColor,
            font_weight: FontWeight::Normal,
            font_decorations: Vec::new(),
            line_wrap_mode: Decawm::default(),
        },
        FormatTag {
            start: 7,
            end: 10,
            color: TerminalColor::Blue,
            background_color: TerminalColor::Black,
            underline_color: TerminalColor::DefaultUnderlineColor,
            font_weight: FontWeight::Normal,
            font_decorations: Vec::new(),
            line_wrap_mode: Decawm::default(),
        },
        FormatTag {
            start: 10,
            end: usize::MAX,
            color: TerminalColor::Red,
            background_color: TerminalColor::Black,
            underline_color: TerminalColor::DefaultUnderlineColor,
            font_weight: FontWeight::Normal,
            font_decorations: Vec::new(),
            line_wrap_mode: Decawm::default(),
        },
    ]
}

#[test]
fn test_format_tracker_scrollback_split_on_boundary() {
    let tags = get_tags();
    // Case 2: Split on a boundary
    let res = split_format_data_for_scrollback(tags.clone(), 10);
    assert_eq!(res.scrollback, &tags[0..3]);
    assert_eq!(
        res.visible,
        &[FormatTag {
            start: 0,
            end: usize::MAX,
            color: TerminalColor::Red,
            background_color: TerminalColor::Black,
            underline_color: TerminalColor::DefaultUnderlineColor,
            font_weight: FontWeight::Normal,
            font_decorations: Vec::new(),
            line_wrap_mode: Decawm::default(),
        },]
    );
}

#[test]
fn test_format_tracker_scrollback_split_segment() {
    let tags = get_tags();

    // Case 3: Split a segment
    let res = split_format_data_for_scrollback(tags, 9);
    assert_eq!(
        res.scrollback,
        &[
            FormatTag {
                start: 0,
                end: 5,
                color: TerminalColor::Blue,
                background_color: TerminalColor::Black,
                underline_color: TerminalColor::DefaultUnderlineColor,
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
            },
            FormatTag {
                start: 5,
                end: 7,
                color: TerminalColor::Red,
                background_color: TerminalColor::Black,
                underline_color: TerminalColor::DefaultUnderlineColor,
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
            },
            FormatTag {
                start: 7,
                end: 9,
                color: TerminalColor::Blue,
                background_color: TerminalColor::Black,
                underline_color: TerminalColor::DefaultUnderlineColor,
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
            },
        ]
    );

    assert_eq!(
        res.visible,
        &[
            FormatTag {
                start: 0,
                end: 1,
                color: TerminalColor::Blue,
                background_color: TerminalColor::Black,
                underline_color: TerminalColor::DefaultUnderlineColor,
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
            },
            FormatTag {
                start: 1,
                end: usize::MAX,
                color: TerminalColor::Red,
                background_color: TerminalColor::Black,
                underline_color: TerminalColor::DefaultUnderlineColor,
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
            },
        ]
    );
}

#[test]
fn test_format_tracker_scrollback_no_split() {
    let tags = get_tags();
    // Case 1: no split
    let res = split_format_data_for_scrollback(tags.clone(), 0);
    assert_eq!(res.scrollback, &[]);
    assert_eq!(res.visible, &tags[..]);
}
