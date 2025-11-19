#[cfg(test)]
mod tests {
    use freminal_buffer::buffer::Buffer;
    use freminal_common::buffer_states::tchar::TChar;

    fn chars(s: &str) -> Vec<TChar> {
        s.chars()
            .map(|c| TChar::Utf8(c.to_string().into_bytes()))
            .collect()
    }

    #[test]
    fn insert_simple_text_in_buffer() {
        let mut buf = Buffer::new(80, 10);

        buf.insert_text(&chars("Hello"));

        assert_eq!(buf.get_rows().len(), 1);
        let row = &buf.get_rows()[0];
        assert_eq!(row.get_row_width(), 5);
        assert_eq!(buf.get_cursor().pos.x, 5);
        assert_eq!(buf.get_cursor().pos.y, 0);
    }

    #[test]
    fn insert_wraps_into_next_row() {
        let mut buf = Buffer::new(3, 10);

        buf.insert_text(&chars("Hello"));

        assert_eq!(buf.get_rows().len(), 2);

        let row0 = &buf.get_rows()[0];
        let row1 = &buf.get_rows()[1];

        assert_eq!(
            row0.get_characters()
                .iter()
                .map(|c| c.into_utf8())
                .collect::<String>(),
            "Hel" // codespell:ignore
        );
        assert_eq!(
            row1.get_characters()
                .iter()
                .map(|c| c.into_utf8())
                .collect::<String>(),
            "lo"
        );

        assert_eq!(buf.get_cursor().pos.y, 1);
        assert_eq!(buf.get_cursor().pos.x, 2);
    }

    #[test]
    fn insert_multiple_wraps() {
        let mut buf = Buffer::new(2, 10);

        buf.insert_text(&chars("abcdef"));

        assert_eq!(buf.get_rows().len(), 3); // "ab", "cd", "ef"
        assert_eq!(buf.get_cursor().pos.y, 2);
        assert_eq!(buf.get_cursor().pos.x, 2);
    }

    #[test]
    fn insert_wide_char_wrap() {
        let mut buf = Buffer::new(3, 10);

        let text = chars("a🙂b"); // widths: 1, 2, 1

        buf.insert_text(&text);

        assert_eq!(buf.get_rows().len(), 2);

        assert_eq!(buf.get_cursor().pos.y, 1);
        assert_eq!(buf.get_cursor().pos.x, 1); // 'b' inserted on row 1
    }
}
