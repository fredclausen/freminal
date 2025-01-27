use freminal_common::scroll::ScrollDirection;
// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.
use test_log::test;

#[test]
fn test_scroll_default() {
    let scroll = ScrollDirection::default();
    assert_eq!(scroll, ScrollDirection::Up(1));
}
