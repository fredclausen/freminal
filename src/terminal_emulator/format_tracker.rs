// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use super::{
    ansi_components::mode::Decawm, CursorState, FontDecorations, FontWeight, TerminalColor,
};
use std::ops::Range;

const fn ranges_overlap(a: Range<usize>, b: Range<usize>) -> bool {
    !(a.end <= b.start || a.start >= b.end)
}
/// if a and b overlap like
/// a:  [         ]
/// b:      [  ]
const fn range_fully_conatins(a: &Range<usize>, b: &Range<usize>) -> bool {
    a.start <= b.start && a.end >= b.end
}

/// if a and b overlap like
/// a:     [      ]
/// b:  [     ]
const fn range_starts_overlapping(a: &Range<usize>, b: &Range<usize>) -> bool {
    a.start > b.start && a.end > b.end
}

/// if a and b overlap like
/// a: [      ]
/// b:    [      ]
const fn range_ends_overlapping(a: &Range<usize>, b: &Range<usize>) -> bool {
    range_starts_overlapping(b, a)
}

fn adjust_existing_format_range(
    existing_elem: &mut FormatTag,
    range: &Range<usize>,
) -> ColorRangeAdjustment {
    let mut ret = ColorRangeAdjustment {
        should_delete: false,
        to_insert: None,
    };

    let existing_range = existing_elem.start..existing_elem.end;
    if range_fully_conatins(range, &existing_range) {
        ret.should_delete = true;
    } else if range_fully_conatins(&existing_range, range) {
        if existing_elem.start == range.start {
            ret.should_delete = true;
        }

        if range.end != existing_elem.end {
            ret.to_insert = Some(FormatTag {
                start: range.end,
                end: existing_elem.end,
                color: existing_elem.color,
                background_color: existing_elem.background_color,
                font_weight: existing_elem.font_weight.clone(),
                font_decorations: existing_elem.font_decorations.clone(),
                line_wrap_mode: existing_elem.line_wrap_mode.clone(),
            });
        }

        existing_elem.end = range.start;
    } else if range_starts_overlapping(range, &existing_range) {
        existing_elem.end = range.start;
        if existing_elem.start == existing_elem.end {
            ret.should_delete = true;
        }
    } else if range_ends_overlapping(range, &existing_range) {
        existing_elem.start = range.end;
        if existing_elem.start == existing_elem.end {
            ret.should_delete = true;
        }
    } else {
        panic!(
            "Unhandled case {}-{}, {}-{}",
            existing_elem.start, existing_elem.end, range.start, range.end
        );
    }

    ret
}

fn delete_items_from_vec<T>(mut to_delete: Vec<usize>, vec: &mut Vec<T>) {
    to_delete.sort_unstable();
    for idx in to_delete.iter().rev() {
        vec.remove(*idx);
    }
}

fn adjust_existing_format_ranges(existing: &mut Vec<FormatTag>, range: &Range<usize>) {
    let mut effected_infos = existing
        .iter_mut()
        .enumerate()
        .filter(|(_i, item)| ranges_overlap(item.start..item.end, range.clone()))
        .collect::<Vec<_>>();

    let mut to_delete = Vec::new();
    let mut to_push = Vec::new();
    for info in &mut effected_infos {
        let adjustment = adjust_existing_format_range(info.1, range);
        if adjustment.should_delete {
            to_delete.push(info.0);
        }
        if let Some(item) = adjustment.to_insert {
            to_push.push(item);
        }
    }

    delete_items_from_vec(to_delete, existing);
    existing.extend(to_push);
}

struct ColorRangeAdjustment {
    // If a range adjustment results in a 0 width element we need to delete it
    should_delete: bool,
    // If a range was split we need to insert a new one
    to_insert: Option<FormatTag>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FormatTag {
    pub start: usize,
    pub end: usize,
    pub color: TerminalColor,
    pub background_color: TerminalColor,
    pub font_weight: FontWeight,
    pub font_decorations: Vec<FontDecorations>,
    pub line_wrap_mode: Decawm,
}

pub struct FormatTracker {
    color_info: Vec<FormatTag>,
}

impl FormatTracker {
    pub fn new() -> Self {
        Self {
            color_info: vec![FormatTag {
                start: 0,
                end: usize::MAX,
                color: TerminalColor::Default,
                background_color: TerminalColor::Black,
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
            }],
        }
    }

    pub fn push_range(&mut self, cursor: &CursorState, range: Range<usize>) {
        adjust_existing_format_ranges(&mut self.color_info, &range);

        self.color_info.push(FormatTag {
            start: range.start,
            end: range.end,
            color: cursor.color,
            background_color: cursor.background_color,
            font_weight: cursor.font_weight.clone(),
            font_decorations: cursor.font_decorations.clone(),
            line_wrap_mode: cursor.line_wrap_mode.clone(),
        });

        // FIXME: Insertion sort
        // FIXME: Merge adjacent
        self.color_info.sort_by(|a, b| a.start.cmp(&b.start));
    }

    /// Move all tags > range.start to range.start + range.len
    /// No gaps in coloring data, so one range must expand instead of just be adjusted
    pub fn push_range_adjustment(&mut self, range: Range<usize>) {
        let range_len = range.end - range.start;
        for info in &mut self.color_info {
            if info.end <= range.start {
                continue;
            }

            if info.start > range.start {
                info.start += range_len;
                if info.end != usize::MAX {
                    info.end += range_len;
                }
            } else if info.end != usize::MAX {
                info.end += range_len;
            }
        }
    }

    pub fn tags(&self) -> Vec<FormatTag> {
        self.color_info.clone()
    }

    pub fn delete_range(&mut self, range: Range<usize>) {
        let mut to_delete = Vec::new();
        let del_size = range.end - range.start;

        for (i, info) in &mut self.color_info.iter_mut().enumerate() {
            let info_range = info.start..info.end;
            if info.end <= range.start {
                continue;
            }

            if ranges_overlap(range.clone(), info_range.clone()) {
                if range_fully_conatins(&range, &info_range) {
                    to_delete.push(i);
                } else if range_starts_overlapping(&range, &info_range) {
                    if info.end != usize::MAX {
                        info.end = range.start;
                    }
                } else if range_ends_overlapping(&range, &info_range) {
                    info.start = range.start;
                    if info.end != usize::MAX {
                        info.end -= del_size;
                    }
                } else if range_fully_conatins(&info_range, &range) {
                    if info.end != usize::MAX {
                        info.end -= del_size;
                    }
                } else {
                    panic!("Unhandled overlap");
                }
            } else {
                assert!(!ranges_overlap(range.clone(), info_range.clone()));
                info.start -= del_size;
                if info.end != usize::MAX {
                    info.end -= del_size;
                }
            }
        }

        for i in to_delete.into_iter().rev() {
            self.color_info.remove(i);
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::CursorState;
    use super::*;

    fn basic_color_test_one(format_tracker: &FormatTracker) {
        let tags = format_tracker.tags();

        assert_eq!(
            tags,
            &[
                FormatTag {
                    start: 0,
                    end: 3,
                    color: TerminalColor::Default,
                    background_color: TerminalColor::Black,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 3,
                    end: 10,
                    color: TerminalColor::Yellow,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 10,
                    end: usize::MAX,
                    color: TerminalColor::Default,
                    background_color: TerminalColor::Black,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
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
                    color: TerminalColor::Default,
                    background_color: TerminalColor::Black,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 3,
                    end: 5,
                    color: TerminalColor::Yellow,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 5,
                    end: 7,
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 7,
                    end: 10,
                    color: TerminalColor::Yellow,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 10,
                    end: usize::MAX,
                    color: TerminalColor::Default,
                    background_color: TerminalColor::Black,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
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
                    color: TerminalColor::Default,
                    background_color: TerminalColor::Black,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 3,
                    end: 5,
                    color: TerminalColor::Yellow,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 5,
                    end: 7,
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 7,
                    end: 9,
                    color: TerminalColor::Green,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 9,
                    end: 10,
                    color: TerminalColor::Yellow,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 10,
                    end: usize::MAX,
                    color: TerminalColor::Default,
                    background_color: TerminalColor::Black,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
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
                    color: TerminalColor::Default,
                    background_color: TerminalColor::Black,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 3,
                    end: 5,
                    color: TerminalColor::Yellow,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 5,
                    end: 6,
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 6,
                    end: 11,
                    color: TerminalColor::Red,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Bold,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 11,
                    end: usize::MAX,
                    color: TerminalColor::Default,
                    background_color: TerminalColor::Black,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
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

        cursor_state.color = TerminalColor::Blue;
        format_tracker.push_range(&cursor_state, 5..7);
        basic_color_test_two(&format_tracker);

        cursor_state.color = TerminalColor::Green;
        format_tracker.push_range(&cursor_state, 7..9);
        basic_color_test_three(&format_tracker);

        cursor_state.color = TerminalColor::Red;
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
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 8,
                    end: 18,
                    color: TerminalColor::Red,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 18,
                    end: usize::MAX,
                    color: TerminalColor::Default,
                    background_color: TerminalColor::Black,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
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
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 6,
                    end: 16,
                    color: TerminalColor::Red,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 16,
                    end: usize::MAX,
                    color: TerminalColor::Default,
                    background_color: TerminalColor::Black,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
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
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 4,
                    end: 14,
                    color: TerminalColor::Red,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 14,
                    end: usize::MAX,
                    color: TerminalColor::Default,
                    background_color: TerminalColor::Black,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
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
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 2,
                    end: 9,
                    color: TerminalColor::Red,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 9,
                    end: usize::MAX,
                    color: TerminalColor::Default,
                    background_color: TerminalColor::Black,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                }
            ]
        );
    }

    #[test]
    fn test_format_tracker_del_range() {
        let mut format_tracker = FormatTracker::new();
        let mut cursor = CursorState::default().with_color(TerminalColor::Blue);
        format_tracker.push_range(&cursor, 0..10);
        cursor.color = TerminalColor::Red;
        format_tracker.push_range(&cursor, 10..20);

        format_tracker.delete_range(0..2);
        del_range_test_one(&format_tracker);

        format_tracker.delete_range(2..4);
        del_range_test_two(&format_tracker);

        format_tracker.delete_range(4..6);
        del_range_test_three(&format_tracker);

        format_tracker.delete_range(2..7);
        del_range_test_four(&format_tracker);
    }

    fn range_adjustment_test_one(format_tracker: &FormatTracker) {
        assert_eq!(
            format_tracker.tags(),
            [
                FormatTag {
                    start: 0,
                    end: 5,
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 5,
                    end: 10,
                    color: TerminalColor::Red,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 10,
                    end: usize::MAX,
                    color: TerminalColor::Default,
                    background_color: TerminalColor::Black,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
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
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 8,
                    end: 13,
                    color: TerminalColor::Red,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 13,
                    end: usize::MAX,
                    color: TerminalColor::Default,
                    background_color: TerminalColor::Black,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
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
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 8,
                    end: 15,
                    color: TerminalColor::Red,
                    background_color: TerminalColor::DefaultBackground,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 15,
                    end: usize::MAX,
                    color: TerminalColor::Default,
                    background_color: TerminalColor::Black,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
            ]
        );
    }

    #[test]
    fn test_range_adjustment() {
        let mut format_tracker = FormatTracker::new();
        let mut cursor = CursorState::default().with_color(TerminalColor::Blue);

        format_tracker.push_range(&cursor, 0..5);
        cursor.color = TerminalColor::Red;
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
}
