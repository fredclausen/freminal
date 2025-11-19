#[cfg(test)]
mod tests {
    use freminal_buffer::{response::InsertResponse, row::Row};
    use freminal_common::buffer_states::{format_tag::FormatTag, tchar::TChar};

    fn tag() -> FormatTag {
        FormatTag::default()
    }

    #[test]
    fn insert_fits_entirely_in_row() {
        let mut row = Row::new(10);
        let text = vec![TChar::Ascii(b'A'), TChar::Ascii(b'B'), TChar::Ascii(b'C')];

        let result = row.insert_text(0, &text, &tag(), 10);

        match result {
            InsertResponse::Consumed(final_col) => {
                assert_eq!(final_col, 3);
                assert_eq!(row.get_row_width(), 3);
                assert_eq!(row.get_char_at(0).unwrap().get_character(), &text[0]);
                assert_eq!(row.get_char_at(1).unwrap().get_character(), &text[1]);
                assert_eq!(row.get_char_at(2).unwrap().get_character(), &text[2]);
            }
            _ => panic!("Expected Consumed"),
        }
    }

    #[test]
    fn insert_overflows_and_returns_leftover() {
        let mut row = Row::new(5);
        let text = vec![
            TChar::Ascii(b'H'),
            TChar::Ascii(b'e'),
            TChar::Ascii(b'l'),
            TChar::Ascii(b'l'),
            TChar::Ascii(b'o'),
        ];

        let result = row.insert_text(3, &text, &tag(), 5);

        match result {
            InsertResponse::Leftover { data, final_col } => {
                assert_eq!(final_col, 5);
                assert_eq!(data.len(), 3);
                // and check that data == ['l','l','o']
                assert_eq!(data[0], TChar::Ascii(b'l'));
                assert_eq!(data[1], TChar::Ascii(b'l'));
                assert_eq!(data[2], TChar::Ascii(b'o'));
                assert!(row.get_row_width() <= 5);
            }
            _ => panic!("Expected Leftover"),
        }
    }

    #[test]
    fn insert_wide_character_that_fits() {
        let mut row = Row::new(5);
        let emoji = TChar::Utf8("🙂".as_bytes().to_vec()); // width 2

        let result = row.insert_text(0, std::slice::from_ref(&emoji), &tag(), 5);

        match result {
            InsertResponse::Consumed(final_col) => {
                assert_eq!(final_col, emoji.display_width());
                assert_eq!(row.get_row_width(), emoji.display_width());
            }
            _ => panic!("Expected Consumed"),
        }
    }

    #[test]
    fn insert_wide_character_that_overflows() {
        let mut row = Row::new(3); // only 3 cols wide
        let emoji = TChar::Utf8("🙂".as_bytes().to_vec()); // width 2

        // insert at col 2 → needs col 2 + width 2 = col 4 (overflow)
        let result = row.insert_text(2, std::slice::from_ref(&emoji), &tag(), 3);

        match result {
            InsertResponse::Leftover { data, final_col } => {
                assert_eq!(final_col, 2);
                assert_eq!(data.len(), 1);
                assert_eq!(data[0], emoji);
            }
            _ => panic!("Expected Leftover"),
        }
    }
}
