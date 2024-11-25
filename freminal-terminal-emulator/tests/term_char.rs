// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use freminal_terminal_emulator::state::term_char::{display_vec_tchar_as_string, TChar};
use unicode_segmentation::UnicodeSegmentation;

#[test]
fn test_new_from_single_char() {
    let c = TChar::new_from_single_char(32);
    assert_eq!(c, TChar::Space);

    let c = TChar::new_from_single_char(10);
    assert_eq!(c, TChar::NewLine);

    let c = TChar::new_from_single_char(65);
    assert_eq!(c, TChar::Ascii(65));
}

#[test]
fn test_new_from_many_chars() {
    let c = TChar::new_from_many_chars(vec![65, 66, 67]).unwrap();
    assert_eq!(c, TChar::Utf8(vec![65, 66, 67]));
}

#[test]
fn test_to_u8() {
    let c = TChar::Ascii(65);
    assert_eq!(c.to_u8(), 65);

    let c = TChar::Space;
    assert_eq!(c.to_u8(), 0);

    let c = TChar::NewLine;
    assert_eq!(c.to_u8(), 0);

    let c = TChar::Utf8(vec![65, 66, 67]);
    assert_eq!(c.to_u8(), 0);
}

#[test]
fn test_from_u8() {
    let c: TChar = 65.into();
    assert_eq!(c, TChar::Ascii(65));
}

#[test]
fn test_from_vec() {
    let c: TChar = vec![65, 66, 67].into();
    assert_eq!(c, TChar::Utf8(vec![65, 66, 67]));
}

#[test]
fn test_eq_u8() {
    let c = TChar::Ascii(65);
    assert_eq!(c, TChar::Ascii(65));

    let c = TChar::Space;
    assert_eq!(c, 32);

    let c = TChar::NewLine;
    assert_eq!(c, 10);

    let c = TChar::Utf8(vec![65, 66, 67]);
    assert_ne!(c, 65);
}

#[test]
fn test_eq_vec() {
    let c = TChar::Utf8(vec![65, 66, 67]);
    assert_eq!(c, vec![65, 66, 67]);

    let c = TChar::Ascii(65);
    assert_ne!(c, vec![65, 66, 67]);
}

#[test]
fn test_self_from_vec() {
    let s = "test";
    let c = TChar::from_vec(s.as_bytes()).unwrap();
    assert_eq!(
        c,
        vec![
            TChar::Ascii(116),
            TChar::Ascii(101),
            TChar::Ascii(115),
            TChar::Ascii(116),
        ]
    );

    // test invalid utf8 input
    let s = vec![0, 128, 255];
    let c = TChar::from_vec(&s);
    assert!(c.is_err());
}

#[test]
fn test_self_from_string() {
    let s = "test";
    let c = TChar::from_string(s).unwrap();
    assert_eq!(
        c,
        vec![
            TChar::Ascii(116),
            TChar::Ascii(101),
            TChar::Ascii(115),
            TChar::Ascii(116),
        ]
    );

    // TODO: We need to test invalid string input....which may be impossible?
}

#[test]
fn test_from_vec_of_graphemes() {
    let s = "test";
    let graphemes = s.graphemes(true).collect::<Vec<&str>>();
    let result = TChar::from_vec_of_graphemes(&graphemes).unwrap();
    assert_eq!(
        result,
        vec![
            TChar::Ascii(116),
            TChar::Ascii(101),
            TChar::Ascii(115),
            TChar::Ascii(116),
        ]
    );
}

#[test]
fn test_equals() {
    let c = TChar::Ascii(65);
    assert_eq!(c, TChar::Ascii(65));

    let c = TChar::Space;
    assert_eq!(c, TChar::Space);

    let c = TChar::NewLine;
    assert_eq!(c, TChar::NewLine);

    let c = TChar::Utf8(vec![65, 66, 67]);
    assert_eq!(c, TChar::Utf8(vec![65, 66, 67]));

    // array of tchars
    let c = vec![
        TChar::Ascii(65),
        TChar::Space,
        TChar::NewLine,
        TChar::Utf8(vec![65, 66, 67]),
    ];
    let d = vec![
        TChar::Ascii(65),
        TChar::Space,
        TChar::NewLine,
        TChar::Utf8(vec![65, 66, 67]),
    ];
    assert_eq!(c, d);

    // different array of tchars
    let c = vec![
        TChar::Ascii(65),
        TChar::Space,
        TChar::NewLine,
        TChar::Utf8(vec![65, 66, 67]),
    ];
    let d = vec![
        TChar::Ascii(65),
        TChar::Space,
        TChar::NewLine,
        TChar::Utf8(vec![65, 66, 68]),
    ];
    assert_ne!(c, d);
}

#[test]
fn test_display() {
    let c = TChar::Ascii(65);
    assert_eq!(format!("{}", c), "A");

    let c = TChar::Space;
    assert_eq!(format!("{}", c), " ");

    let c = TChar::NewLine;
    assert_eq!(format!("{}", c), "\n");

    let c = TChar::Utf8(vec![65, 66, 67]);
    assert_eq!(format!("{}", c), "ABC");

    let c = vec![
        TChar::Ascii(65),
        TChar::Space,
        TChar::NewLine,
        TChar::Utf8(vec![65, 66, 67]),
    ];
    assert_eq!(format!("{}", display_vec_tchar_as_string(&c)), "A \nABC");
}
