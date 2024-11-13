// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use terminal_emulator::state::term_char::TChar;

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
    assert_eq!(c, 65);

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
fn test_invalid_utf8() {
    // Ѐ in to bytes
    let s = "\u{0400}".as_bytes();
    assert!(std::str::from_utf8(s).is_ok());

    // make sure the TChar::Utf8 will not panic
    assert!(std::panic::catch_unwind(|| TChar::new_from_many_chars(s.to_vec())).is_ok());

    // drop the last byte to make it invalid utf8
    let s = &s[..s.len() - 1];
    assert!(std::str::from_utf8(s).is_err());

    // now make sure the TChar::Utf8 will panic
    assert!(TChar::new_from_many_chars(s.to_vec()).is_err());
}
