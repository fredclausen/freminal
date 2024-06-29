// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use super::{CursorState, FontWeight, FontDecorations, TerminalColor};
use std::ops::Range;

const fn ranges_overlap(a: Range<usize>, b: Range<usize>) -> bool {
    if a.end <= b.start {
        return false;
    }

    if a.start >= b.end {
        return false;
    }

    true
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
                font_weight: existing_elem.font_weight.clone(),
                font_decorations: existing_elem.font_decorations.clone(),
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
    pub font_weight: FontWeight,
    pub font_decorations: Vec<FontDecorations>,
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
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
            }],
        }
    }

    pub fn push_range(&mut self, cursor: &CursorState, range: Range<usize>) {
        adjust_existing_format_ranges(&mut self.color_info, &range);

        self.color_info.push(FormatTag {
            start: range.start,
            end: range.end,
            color: cursor.color,
            font_weight: cursor.font_weight.clone(),
            font_decorations: cursor.font_decorations.clone(),
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
    use super::super::{CursorPos, CursorState};
    use super::*;

    #[test]
    fn basic_color_tracker_test() {
        let mut format_tracker = FormatTracker::new();
        let mut cursor_state = CursorState {
            pos: CursorPos { x: 0, y: 0 },
            color: TerminalColor::Default,
            font_weight: FontWeight::Normal,
            font_decorations: Vec::new(),
        };

        cursor_state.color = TerminalColor::Yellow;
        format_tracker.push_range(&cursor_state, 3..10);
        let tags = format_tracker.tags();
        assert_eq!(
            tags,
            &[
                FormatTag {
                    start: 0,
                    end: 3,
                    color: TerminalColor::Default,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 3,
                    end: 10,
                    color: TerminalColor::Yellow,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 10,
                    end: usize::MAX,
                    color: TerminalColor::Default,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
            ]
        );

        cursor_state.color = TerminalColor::Blue;
        format_tracker.push_range(&cursor_state, 5..7);
        let tags = format_tracker.tags();
        assert_eq!(
            tags,
            &[
                FormatTag {
                    start: 0,
                    end: 3,
                    color: TerminalColor::Default,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 3,
                    end: 5,
                    color: TerminalColor::Yellow,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 5,
                    end: 7,
                    color: TerminalColor::Blue,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 7,
                    end: 10,
                    color: TerminalColor::Yellow,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 10,
                    end: usize::MAX,
                    color: TerminalColor::Default,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
            ]
        );

        cursor_state.color = TerminalColor::Green;
        format_tracker.push_range(&cursor_state, 7..9);
        let tags = format_tracker.tags();
        assert_eq!(
            tags,
            &[
                FormatTag {
                    start: 0,
                    end: 3,
                    color: TerminalColor::Default,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 3,
                    end: 5,
                    color: TerminalColor::Yellow,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 5,
                    end: 7,
                    color: TerminalColor::Blue,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 7,
                    end: 9,
                    color: TerminalColor::Green,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 9,
                    end: 10,
                    color: TerminalColor::Yellow,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 10,
                    end: usize::MAX,
                    color: TerminalColor::Default,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
            ]
        );

        cursor_state.color = TerminalColor::Red;
        cursor_state.font_weight = FontWeight::Bold;
        format_tracker.push_range(&cursor_state, 6..11);
        let tags = format_tracker.tags();
        assert_eq!(
            tags,
            &[
                FormatTag {
                    start: 0,
                    end: 3,
                    color: TerminalColor::Default,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 3,
                    end: 5,
                    color: TerminalColor::Yellow,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 5,
                    end: 6,
                    color: TerminalColor::Blue,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 6,
                    end: 11,
                    color: TerminalColor::Red,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 11,
                    end: usize::MAX,
                    color: TerminalColor::Default,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
            ]
        );
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

    #[test]
    fn test_format_tracker_del_range() {
        let mut format_tracker = FormatTracker::new();
        let mut cursor = CursorState {
            pos: CursorPos { x: 0, y: 0 },
            color: TerminalColor::Blue,
            font_weight: FontWeight::Normal,
            font_decorations: Vec::new(),
        };
        format_tracker.push_range(&cursor, 0..10);
        cursor.color = TerminalColor::Red;
        format_tracker.push_range(&cursor, 10..20);

        format_tracker.delete_range(0..2);
        assert_eq!(
            format_tracker.tags(),
            [
                FormatTag {
                    start: 0,
                    end: 8,
                    color: TerminalColor::Blue,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 8,
                    end: 18,
                    color: TerminalColor::Red,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 18,
                    end: usize::MAX,
                    color: TerminalColor::Default,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                }
            ]
        );

        format_tracker.delete_range(2..4);
        assert_eq!(
            format_tracker.tags(),
            [
                FormatTag {
                    start: 0,
                    end: 6,
                    color: TerminalColor::Blue,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 6,
                    end: 16,
                    color: TerminalColor::Red,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 16,
                    end: usize::MAX,
                    color: TerminalColor::Default,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                }
            ]
        );

        format_tracker.delete_range(4..6);
        assert_eq!(
            format_tracker.tags(),
            [
                FormatTag {
                    start: 0,
                    end: 4,
                    color: TerminalColor::Blue,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 4,
                    end: 14,
                    color: TerminalColor::Red,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 14,
                    end: usize::MAX,
                    color: TerminalColor::Default,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                }
            ]
        );

        format_tracker.delete_range(2..7);
        assert_eq!(
            format_tracker.tags(),
            [
                FormatTag {
                    start: 0,
                    end: 2,
                    color: TerminalColor::Blue,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 2,
                    end: 9,
                    color: TerminalColor::Red,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 9,
                    end: usize::MAX,
                    color: TerminalColor::Default,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                }
            ]
        );
    }

    #[test]
    fn test_range_adjustment() {
        let mut format_tracker = FormatTracker::new();
        let mut cursor = CursorState {
            pos: CursorPos { x: 0, y: 0 },
            color: TerminalColor::Blue,
            font_weight: FontWeight::Normal,
            font_decorations: Vec::new(),
        };
        format_tracker.push_range(&cursor, 0..5);
        cursor.color = TerminalColor::Red;
        format_tracker.push_range(&cursor, 5..10);

        assert_eq!(
            format_tracker.tags(),
            [
                FormatTag {
                    start: 0,
                    end: 5,
                    color: TerminalColor::Blue,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 5,
                    end: 10,
                    color: TerminalColor::Red,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 10,
                    end: usize::MAX,
                    color: TerminalColor::Default,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
            ]
        );

        // This should extend the first section, and push all the ones after
        format_tracker.push_range_adjustment(0..3);
        assert_eq!(
            format_tracker.tags(),
            [
                FormatTag {
                    start: 0,
                    end: 8,
                    color: TerminalColor::Blue,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 8,
                    end: 13,
                    color: TerminalColor::Red,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 13,
                    end: usize::MAX,
                    color: TerminalColor::Default,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
            ]
        );

        // Should have no effect as we're in the last range
        format_tracker.push_range_adjustment(15..50);
        assert_eq!(
            format_tracker.tags(),
            [
                FormatTag {
                    start: 0,
                    end: 8,
                    color: TerminalColor::Blue,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 8,
                    end: 13,
                    color: TerminalColor::Red,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 13,
                    end: usize::MAX,
                    color: TerminalColor::Default,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
            ]
        );

        // And for good measure, check something in the middle
        // This should not touch the first segment, extend the second, and move the third forward
        format_tracker.push_range_adjustment(10..12);
        assert_eq!(
            format_tracker.tags(),
            [
                FormatTag {
                    start: 0,
                    end: 8,
                    color: TerminalColor::Blue,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 8,
                    end: 15,
                    color: TerminalColor::Red,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 15,
                    end: usize::MAX,
                    color: TerminalColor::Default,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
            ]
        );
    }
}
