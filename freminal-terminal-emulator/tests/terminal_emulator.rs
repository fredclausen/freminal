// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use test_log::test;

use freminal_common::colors::TerminalColor;
use freminal_terminal_emulator::{
    ansi_components::modes::decawm::Decawm,
    format_tracker::FormatTag,
    interface::split_format_data_for_scrollback,
    state::{
        cursor::{ReverseVideo, StateColors},
        fonts::FontWeight,
    },
};

fn get_tags() -> Vec<FormatTag> {
    vec![
        FormatTag {
            start: 0,
            end: 5,
            colors: StateColors {
                color: TerminalColor::Blue,
                background_color: TerminalColor::Black,
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
            end: 7,
            colors: StateColors {
                color: TerminalColor::Red,
                background_color: TerminalColor::Black,
                underline_color: TerminalColor::DefaultUnderlineColor,
                reverse_video: ReverseVideo::Off,
            },
            font_weight: FontWeight::Normal,
            font_decorations: Vec::new(),
            line_wrap_mode: Decawm::default(),
            url: None,
        },
        FormatTag {
            start: 7,
            end: 10,
            colors: StateColors {
                color: TerminalColor::Blue,
                background_color: TerminalColor::Black,
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
            colors: StateColors {
                color: TerminalColor::Red,
                background_color: TerminalColor::Black,
                underline_color: TerminalColor::DefaultUnderlineColor,
                reverse_video: ReverseVideo::Off,
            },
            font_weight: FontWeight::Normal,
            font_decorations: Vec::new(),
            line_wrap_mode: Decawm::default(),
            url: None,
        },
    ]
}

#[test]
fn test_format_tracker_scrollback_split_on_boundary() {
    let tags = get_tags();
    // Case 2: Split on a boundary
    let res = split_format_data_for_scrollback(tags.clone(), 10, true);
    assert_eq!(res.scrollback, &tags[0..3]);
    assert_eq!(
        res.visible,
        &[FormatTag {
            start: 0,
            end: usize::MAX,
            colors: StateColors {
                color: TerminalColor::Red,
                background_color: TerminalColor::Black,
                underline_color: TerminalColor::DefaultUnderlineColor,
                reverse_video: ReverseVideo::Off,
            },
            font_weight: FontWeight::Normal,
            font_decorations: Vec::new(),
            line_wrap_mode: Decawm::default(),
            url: None,
        },]
    );
}

#[test]
fn test_format_tracker_scrollback_split_segment() {
    let tags = get_tags();

    // Case 3: Split a segment
    let res = split_format_data_for_scrollback(tags, 9, true);
    assert_eq!(
        res.scrollback,
        &[
            FormatTag {
                start: 0,
                end: 5,
                colors: StateColors {
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::Black,
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
                end: 7,
                colors: StateColors {
                    color: TerminalColor::Red,
                    background_color: TerminalColor::Black,
                    underline_color: TerminalColor::DefaultUnderlineColor,
                    reverse_video: ReverseVideo::Off,
                },
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 7,
                end: 9,
                colors: StateColors {
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::Black,
                    underline_color: TerminalColor::DefaultUnderlineColor,
                    reverse_video: ReverseVideo::Off,
                },
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
        ]
    );

    assert_eq!(
        res.visible,
        &[
            FormatTag {
                start: 0,
                end: 1,
                colors: StateColors {
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::Black,
                    underline_color: TerminalColor::DefaultUnderlineColor,
                    reverse_video: ReverseVideo::Off,
                },
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
            FormatTag {
                start: 1,
                end: usize::MAX,
                colors: StateColors {
                    color: TerminalColor::Red,
                    background_color: TerminalColor::Black,
                    underline_color: TerminalColor::DefaultUnderlineColor,
                    reverse_video: ReverseVideo::Off,
                },
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
                url: None,
            },
        ]
    );
}

#[test]
fn test_format_tracker_scrollback_no_split() {
    let tags = get_tags();
    // Case 1: no split
    let res = split_format_data_for_scrollback(tags.clone(), 0, true);
    assert_eq!(res.scrollback, &[]);
    assert_eq!(res.visible, &tags[..]);
}
