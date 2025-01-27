use freminal_common::cursor::CursorVisualStyle;
// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.
use test_log::test;

#[test]
fn test_cursor_visual_style_from_usize() {
    let cursor = CursorVisualStyle::from(2);
    assert_eq!(cursor, CursorVisualStyle::BlockCursorSteady);

    let cursor = CursorVisualStyle::from(3);
    assert_eq!(cursor, CursorVisualStyle::UnderlineCursorBlink);

    let cursor = CursorVisualStyle::from(4);
    assert_eq!(cursor, CursorVisualStyle::UnderlineCursorSteady);

    let cursor = CursorVisualStyle::from(5);
    assert_eq!(cursor, CursorVisualStyle::VerticalLineCursorBlink);

    let cursor = CursorVisualStyle::from(6);
    assert_eq!(cursor, CursorVisualStyle::VerticalLineCursorSteady);

    let cursor = CursorVisualStyle::from(7);
    assert_eq!(cursor, CursorVisualStyle::BlockCursorBlink);
}
