// WIP for replaying terminal data

#![allow(dead_code)]
#![allow(unused_variables)]

use super::ansi_components::{mode::BracketedPaste, sgr::SelectGraphicRendition};
use super::{
    ansi::{FreminalAnsiParser, TerminalOutput},
    buffer::TerminalBufferHolder,
    format_tracker::FormatTracker,
    split_format_data_for_scrollback, CursorPos, CursorState, Decawm, Decckm, FontDecorations,
    FontWeight, FormatTag, Mode, TerminalColor, TerminalData, TerminalModes,
};

pub const TERMINAL_WIDTH: usize = 112;
pub const TERMINAL_HEIGHT: usize = 38;

pub struct ReplayIo {
    parser: FreminalAnsiParser,
    terminal_buffer: TerminalBufferHolder,
    format_tracker: FormatTracker,
    cursor_state: CursorState,
    modes: TerminalModes,
    saved_color_state: Option<(TerminalColor, TerminalColor)>,
}

impl ReplayIo {
    pub fn new() -> Self {
        Self {
            parser: FreminalAnsiParser::new(),
            terminal_buffer: TerminalBufferHolder::new(TERMINAL_WIDTH, TERMINAL_HEIGHT),
            format_tracker: FormatTracker::new(),
            cursor_state: CursorState {
                pos: CursorPos { x: 0, y: 0 },
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                color: TerminalColor::Default,
                background_color: TerminalColor::Black,
            },
            modes: TerminalModes {
                cursor_key: Decckm::default(),
                autowrap: Decawm::default(),
                bracketed_paste: BracketedPaste::default(),
            },
            saved_color_state: None,
        }
    }

    pub const fn get_win_size(&self) -> (usize, usize) {
        self.terminal_buffer.get_win_size()
    }

    pub fn set_win_size(&mut self, width_chars: usize, height_chars: usize) {
        let response =
            self.terminal_buffer
                .set_win_size(width_chars, height_chars, &self.cursor_state.pos);
        self.cursor_state.pos = response.new_cursor_pos;
    }

    fn handle_data(&mut self, data: &[u8]) {
        let response = self
            .terminal_buffer
            .insert_data(&self.cursor_state.pos, data);
        self.format_tracker
            .push_range_adjustment(response.insertion_range);
        self.format_tracker
            .push_range(&self.cursor_state, response.written_range);
        self.cursor_state.pos = response.new_cursor_pos;
    }

    fn set_cursor_pos(&mut self, x: Option<usize>, y: Option<usize>) {
        if let Some(x) = x {
            self.cursor_state.pos.x = x - 1;
        }
        if let Some(y) = y {
            self.cursor_state.pos.y = y - 1;
        }
    }

    fn set_cursor_pos_rel(&mut self, x: Option<i32>, y: Option<i32>) {
        if let Some(x) = x {
            let x: i64 = x.into();
            let current_x: i64 = self
                .cursor_state
                .pos
                .x
                .try_into()
                .expect("x position larger than i64 can handle");
            self.cursor_state.pos.x = usize::try_from((current_x + x).max(0)).unwrap_or(0);
        }
        if let Some(y) = y {
            let y: i64 = y.into();
            let current_y: i64 = self
                .cursor_state
                .pos
                .y
                .try_into()
                .expect("y position larger than i64 can handle");
            // ensure y is not negative, and throw an error if it is
            self.cursor_state.pos.y = usize::try_from((current_y + y).max(0)).unwrap_or(0);
        }
    }

    fn clear_forwards(&mut self) {
        if let Some(buf_pos) = self.terminal_buffer.clear_forwards(&self.cursor_state.pos) {
            self.format_tracker
                .push_range(&self.cursor_state, buf_pos..usize::MAX);
        }
    }

    fn clear_all(&mut self) {
        self.format_tracker
            .push_range(&self.cursor_state, 0..usize::MAX);
        self.terminal_buffer.clear_all();
    }

    fn clear_line_forwards(&mut self) {
        if let Some(range) = self
            .terminal_buffer
            .clear_line_forwards(&self.cursor_state.pos)
        {
            self.format_tracker.delete_range(range);
        }
    }

    fn carriage_return(&mut self) {
        self.cursor_state.pos.x = 0;
    }

    fn new_line(&mut self) {
        self.cursor_state.pos.y += 1;
    }

    fn backspace(&mut self) {
        if self.cursor_state.pos.x >= 1 {
            self.cursor_state.pos.x -= 1;
        }
    }

    fn insert_lines(&mut self, num_lines: usize) {
        let response = self
            .terminal_buffer
            .insert_lines(&self.cursor_state.pos, num_lines);
        self.format_tracker.delete_range(response.deleted_range);
        self.format_tracker
            .push_range_adjustment(response.inserted_range);
    }

    fn delete(&mut self, num_chars: usize) {
        let deleted_buf_range = self
            .terminal_buffer
            .delete_forwards(&self.cursor_state.pos, num_chars);
        if let Some(range) = deleted_buf_range {
            self.format_tracker.delete_range(range);
        }
    }

    fn sgr(&mut self, sgr: SelectGraphicRendition) {
        // if let Some(color) = TerminalColor::from_sgr(sgr) {
        //     self.cursor_state.color = color;
        //     return;
        // }

        match sgr {
            SelectGraphicRendition::Reset => {
                self.cursor_state.color = TerminalColor::Default;
                self.cursor_state.background_color = TerminalColor::Black;
                self.cursor_state.font_weight = FontWeight::Normal;
                self.cursor_state.font_decorations.clear();
                self.saved_color_state = None;
            }
            SelectGraphicRendition::Bold => {
                self.cursor_state.font_weight = FontWeight::Bold;
            }
            SelectGraphicRendition::Italic => {
                // add in FontDecorations::Italic if it's not already there
                if !self
                    .cursor_state
                    .font_decorations
                    .contains(&FontDecorations::Italic)
                {
                    self.cursor_state
                        .font_decorations
                        .push(FontDecorations::Italic);
                }
            }
            SelectGraphicRendition::Faint => {
                if !self
                    .cursor_state
                    .font_decorations
                    .contains(&FontDecorations::Faint)
                {
                    self.cursor_state
                        .font_decorations
                        .push(FontDecorations::Faint);
                }
            }
            SelectGraphicRendition::ResetBold => {
                self.cursor_state.font_weight = FontWeight::Normal;
            }
            SelectGraphicRendition::NormalIntensity => {
                if self
                    .cursor_state
                    .font_decorations
                    .contains(&FontDecorations::Faint)
                {
                    self.cursor_state
                        .font_decorations
                        .retain(|d| *d != FontDecorations::Faint);
                }
            }
            SelectGraphicRendition::NotUnderlined => {
                // remove FontDecorations::Underline if it's there
                self.cursor_state.font_decorations.retain(|d| {
                    *d != FontDecorations::Underline || *d != FontDecorations::DoubleUnderline
                });
            }
            SelectGraphicRendition::ReverseVideo => {
                let foreground = self.cursor_state.color;
                let background = self.cursor_state.background_color;
                self.saved_color_state = Some((foreground, background));

                self.cursor_state.color = background;
                self.cursor_state.background_color = foreground;
            }
            SelectGraphicRendition::ResetReverseVideo => {
                if let Some((foreground, background)) = self.saved_color_state {
                    self.cursor_state.color = foreground;
                    self.cursor_state.background_color = background;

                    self.saved_color_state = None;
                }
            }
            SelectGraphicRendition::ForegroundBlack => {
                self.cursor_state.color = TerminalColor::Black;
            }
            SelectGraphicRendition::ForegroundRed => {
                self.cursor_state.color = TerminalColor::Red;
            }
            SelectGraphicRendition::ForegroundGreen => {
                self.cursor_state.color = TerminalColor::Green;
            }
            SelectGraphicRendition::ForegroundYellow => {
                self.cursor_state.color = TerminalColor::Yellow;
            }
            SelectGraphicRendition::ForegroundBlue => {
                self.cursor_state.color = TerminalColor::Blue;
            }
            SelectGraphicRendition::ForegroundMagenta => {
                self.cursor_state.color = TerminalColor::Magenta;
            }
            SelectGraphicRendition::ForegroundCyan => {
                self.cursor_state.color = TerminalColor::Cyan;
            }
            SelectGraphicRendition::ForegroundWhite => {
                self.cursor_state.color = TerminalColor::White;
            }
            SelectGraphicRendition::DefaultForeground => {
                self.cursor_state.color = TerminalColor::Default;
            }
            SelectGraphicRendition::ForegroundCustom(r, g, b) => {
                let r = u8::try_from(r).unwrap();
                let g = u8::try_from(g).unwrap();
                let b = u8::try_from(b).unwrap();
                self.cursor_state.color = TerminalColor::Custom(r, g, b);
            }
            SelectGraphicRendition::ForegroundBrightYellow => {
                self.cursor_state.color = TerminalColor::BrightYellow;
            }
            SelectGraphicRendition::ForegroundBrightBlack => {
                self.cursor_state.color = TerminalColor::BrightBlack;
            }
            SelectGraphicRendition::ForegroundBrightRed => {
                self.cursor_state.color = TerminalColor::BrightRed;
            }
            SelectGraphicRendition::ForegroundBrightGreen => {
                self.cursor_state.color = TerminalColor::BrightGreen;
            }
            SelectGraphicRendition::ForegroundBrightBlue => {
                self.cursor_state.color = TerminalColor::BrightBlue;
            }
            SelectGraphicRendition::ForegroundBrightMagenta => {
                self.cursor_state.color = TerminalColor::BrightMagenta;
            }
            SelectGraphicRendition::ForegroundBrightCyan => {
                self.cursor_state.color = TerminalColor::BrightCyan;
            }
            SelectGraphicRendition::ForegroundBrightWhite => {
                self.cursor_state.color = TerminalColor::BrightWhite;
            }
            SelectGraphicRendition::DefaultBackground | SelectGraphicRendition::BackgroundBlack => {
                self.cursor_state.background_color = TerminalColor::Black;
            }
            SelectGraphicRendition::BackgroundRed => {
                self.cursor_state.background_color = TerminalColor::Red;
            }
            SelectGraphicRendition::BackgroundGreen => {
                self.cursor_state.background_color = TerminalColor::Green;
            }
            SelectGraphicRendition::BackgroundYellow => {
                self.cursor_state.background_color = TerminalColor::Yellow;
            }
            SelectGraphicRendition::BackgroundBlue => {
                self.cursor_state.background_color = TerminalColor::Blue;
            }
            SelectGraphicRendition::BackgroundMagenta => {
                self.cursor_state.background_color = TerminalColor::Magenta;
            }
            SelectGraphicRendition::BackgroundCyan => {
                self.cursor_state.background_color = TerminalColor::Cyan;
            }
            SelectGraphicRendition::BackgroundWhite => {
                self.cursor_state.background_color = TerminalColor::White;
            }
            SelectGraphicRendition::BackgroundBrightBlack => {
                self.cursor_state.background_color = TerminalColor::BrightBlack;
            }
            SelectGraphicRendition::BackgroundBrightRed => {
                self.cursor_state.background_color = TerminalColor::BrightRed;
            }
            SelectGraphicRendition::BackgroundBrightYellow => {
                self.cursor_state.background_color = TerminalColor::BrightYellow;
            }
            SelectGraphicRendition::BackgroundBrightBlue => {
                self.cursor_state.background_color = TerminalColor::BrightBlue;
            }
            SelectGraphicRendition::BackgroundBrightMagenta => {
                self.cursor_state.background_color = TerminalColor::BrightMagenta;
            }
            SelectGraphicRendition::BackgroundBrightCyan => {
                self.cursor_state.background_color = TerminalColor::BrightCyan;
            }
            SelectGraphicRendition::BackgroundBrightWhite => {
                self.cursor_state.background_color = TerminalColor::BrightWhite;
            }
            SelectGraphicRendition::BackgroundBrightGreen => {
                self.cursor_state.background_color = TerminalColor::BrightGreen;
            }
            SelectGraphicRendition::BackgroundCustom(r, g, b) => {
                let r = u8::try_from(r).unwrap();
                let g = u8::try_from(g).unwrap();
                let b = u8::try_from(b).unwrap();
                self.cursor_state.background_color = TerminalColor::Custom(r, g, b);
            }
            SelectGraphicRendition::FastBlink | SelectGraphicRendition::SlowBlink => (),
            SelectGraphicRendition::Unknown(_) => {
                warn!("Unhandled sgr: {:?}", sgr);
            }
        }
    }

    fn set_mode(&mut self, mode: &Mode) {
        match mode {
            Mode::Decckm => {
                self.modes.cursor_key = Decckm::Application;
            }
            Mode::Decawm => {
                self.modes.autowrap = Decawm::AutoWrap;
            }
            Mode::BracketedPaste => {
                self.modes.bracketed_paste = BracketedPaste::Enabled;
            }
            Mode::Unknown(_) => {
                warn!("unhandled set mode: {mode:?}");
            }
        }
    }

    fn insert_spaces(&mut self, num_spaces: usize) {
        let response = self
            .terminal_buffer
            .insert_spaces(&self.cursor_state.pos, num_spaces);
        self.format_tracker
            .push_range_adjustment(response.insertion_range);
    }

    fn reset_mode(&mut self, mode: &Mode) {
        match mode {
            Mode::Decckm => {
                self.modes.cursor_key = Decckm::Ansi;
            }
            Mode::Decawm => {
                self.modes.autowrap = Decawm::NoAutoWrap;
            }
            Mode::BracketedPaste => {
                self.modes.bracketed_paste = BracketedPaste::Disabled;
            }
            Mode::Unknown(_) => {}
        }
    }

    pub fn handle_incoming_data(&mut self, incoming: &[u8]) {
        let parsed = self.parser.push(incoming);
        for segment in parsed {
            match segment {
                TerminalOutput::Data(data) => self.handle_data(&data),
                TerminalOutput::SetCursorPos { x, y } => self.set_cursor_pos(x, y),
                TerminalOutput::SetCursorPosRel { x, y } => self.set_cursor_pos_rel(x, y),
                TerminalOutput::ClearForwards => self.clear_forwards(),
                TerminalOutput::ClearAll => self.clear_all(),
                TerminalOutput::ClearLineForwards => self.clear_line_forwards(),
                TerminalOutput::CarriageReturn => self.carriage_return(),
                TerminalOutput::Newline => self.new_line(),
                TerminalOutput::Backspace => self.backspace(),
                TerminalOutput::InsertLines(num_lines) => self.insert_lines(num_lines),
                TerminalOutput::Delete(num_chars) => self.delete(num_chars),
                TerminalOutput::Sgr(sgr) => self.sgr(sgr),
                TerminalOutput::SetMode(mode) => self.set_mode(&mode),
                TerminalOutput::InsertSpaces(num_spaces) => self.insert_spaces(num_spaces),
                TerminalOutput::ResetMode(mode) => self.reset_mode(&mode),
                TerminalOutput::Bell
                | TerminalOutput::Invalid
                | TerminalOutput::OscResponse(_)
                | TerminalOutput::CursorReport => (),
            }
        }
    }

    pub fn data(&self) -> TerminalData<&[u8]> {
        self.terminal_buffer.data()
    }

    pub fn format_data(&self) -> TerminalData<Vec<FormatTag>> {
        let offset = self.terminal_buffer.data().scrollback.len();
        split_format_data_for_scrollback(self.format_tracker.tags(), offset)
    }

    pub fn cursor_pos(&self) -> CursorPos {
        self.cursor_state.pos.clone()
    }
}

#[test]

fn format_tracker_with_data() {
    let mut terminal_io = ReplayIo::new();

    let data: [u8; 1023] = [
        27, 91, 49, 109, 27, 91, 51, 50, 109, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
        32, 32, 32, 32, 32, 32, 32, 32, 27, 91, 51, 50, 109, 46, 46, 39, 13, 10, 32, 32, 32, 32,
        32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 44, 120, 78, 77, 77, 46, 13, 10, 32,
        32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 46, 79, 77, 77, 77, 77, 111, 13,
        10, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 108, 77, 77, 34, 13, 10,
        32, 32, 32, 32, 32, 46, 59, 108, 111, 100, 100, 111, 58, 46, 32, 32, 46, 111, 108, 108,
        111, 100, 100, 111, 108, 59, 46, 13, 10, 32, 32, 32, 99, 75, 77, 77, 77, 77, 77, 77, 77,
        77, 77, 77, 78, 87, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 48, 58, 13, 10, 32, 27, 91, 51,
        51, 109, 46, 75, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77,
        77, 77, 77, 77, 77, 87, 100, 46, 13, 10, 32, 88, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77,
        77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 88, 46, 13, 10, 27, 91, 51, 49, 109,
        59, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77,
        77, 77, 58, 13, 10, 58, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77,
        77, 77, 77, 77, 77, 77, 77, 58, 13, 10, 46, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77,
        77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 88, 46, 13, 10, 32, 107, 77, 77, 77, 77,
        77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 87, 100,
        46, 13, 10, 32, 27, 91, 51, 53, 109, 39, 88, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77,
        77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 107, 13, 10, 32, 32, 39, 88,
        77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77,
        77, 75, 46, 13, 10, 32, 32, 32, 32, 27, 91, 51, 52, 109, 107, 77, 77, 77, 77, 77, 77, 77,
        77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 77, 100, 13, 10, 32, 32, 32, 32,
        32, 59, 75, 77, 77, 77, 77, 77, 77, 77, 87, 88, 88, 87, 77, 77, 77, 77, 77, 77, 77, 107,
        46, 13, 10, 32, 32, 32, 32, 32, 32, 32, 34, 99, 111, 111, 99, 42, 34, 32, 32, 32, 32, 34,
        42, 99, 111, 111, 39, 34, 27, 91, 109, 27, 91, 49, 71, 27, 91, 49, 54, 65, 27, 91, 109, 27,
        91, 63, 55, 108, 27, 91, 51, 52, 67, 27, 91, 109, 27, 91, 49, 109, 27, 91, 51, 50, 109,
        102, 114, 101, 100, 27, 91, 109, 64, 27, 91, 49, 109, 27, 91, 51, 50, 109, 70, 114, 101,
        100, 115, 45, 77, 97, 99, 45, 83, 116, 117, 100, 105, 111, 27, 91, 109, 13, 10, 27, 91, 51,
        52, 67, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45, 45,
        13, 10, 27, 91, 51, 52, 67, 27, 91, 109, 27, 91, 49, 109, 27, 91, 51, 51, 109, 79, 83, 27,
        91, 109, 58, 32, 27, 91, 109, 109, 97, 99, 79, 83, 32, 83, 101, 113, 117, 111, 105, 97, 32,
        49, 53, 46, 49, 32, 97, 114, 109, 54, 52, 13, 10, 27, 91, 51, 52, 67, 27, 91, 109, 27, 91,
        49, 109, 27, 91, 51, 51, 109, 72, 111, 115, 116, 27, 91, 109, 58, 32, 27, 91, 109, 77, 97,
        99, 32, 83, 116, 117, 100, 105, 111, 32, 40, 77, 49, 32, 77, 97, 120, 44, 32, 50, 48, 50,
        50, 44, 32, 84, 119, 111, 32, 85, 83, 66, 45, 67, 32, 102, 114, 111, 110, 116, 32, 112,
        111, 114, 116, 115, 41, 27, 91, 109, 13, 10, 27, 91, 51, 52, 67, 27, 91, 109, 27, 91, 49,
        109, 27, 91, 51, 51, 109, 75, 101, 114, 110, 101, 108, 27, 91, 109, 58, 32, 27, 91, 109,
        68, 97, 114, 119, 105, 110, 32, 50, 52, 46, 49, 46, 48, 13, 10, 27, 91, 51, 52, 67, 27, 91,
        109, 27, 91, 49, 109, 27, 91, 51, 51, 109, 85, 112, 116, 105, 109, 101, 27, 91, 109, 58,
        32, 27, 91, 109, 49, 52, 32, 100, 97, 121, 115, 44, 32, 53, 53, 32, 109, 105, 110, 115, 13,
        10, 27, 91, 51, 52, 67, 13, 10, 27, 91, 51, 52, 67, 27, 91, 109, 27, 91, 49, 109, 27, 91,
        51, 51, 109, 80, 97, 99, 107, 97, 103, 101, 115, 27, 91, 109, 58, 32, 27, 91, 109, 49, 51,
        55, 32, 40, 98, 114, 101, 119, 41, 44, 32, 51, 48, 32, 40, 98, 114, 101, 119, 45, 99, 97,
        115, 107, 41, 13, 10, 27, 91, 51, 52, 67, 27, 91, 109, 27, 91, 49, 109, 27, 91, 51, 51,
        109, 83, 104, 101, 108, 108, 27, 91, 109, 58, 32, 27, 91, 109, 122, 115, 104, 32, 53, 46,
        57, 13, 10, 27, 91, 51, 52, 67, 27, 91, 109, 27, 91, 49, 109, 27, 91, 51, 51, 109, 68, 105,
        115, 112, 108, 97, 121, 32, 40, 83, 99, 101, 112, 116, 114, 101, 32, 67, 51, 53, 41, 27,
        91, 109, 58, 32, 27, 91, 109, 51, 52, 52, 48, 120, 49, 52, 52, 48, 32, 64, 32, 54, 48, 32,
        72, 122, 32, 105, 110, 32, 51, 53,
    ];

    //,226128,179,32,91,69,120,116,101,114,110,97,108,93,32,42,13,10,27,91,51,52,67,27,91,109,27,91,49,109,27,91,51,51,109,68,105,115,112,108,97,121,32,40,82,50,52,48,72,89,41,27,91,109,58,32,27,91,109,49,57,50,48,120,49,48,56,48,32,64,32,54,48,32,72,122,32,105,110,32,50,52,226,128,179,32,91,69,120,116,101,114,110,97,108,93,13,10,27,91,51,52,67,27,91,109,27,91,49,109,27,91,51,51,109,68,105,115,112,108,97,121,32,40,82,50,52,48,72,89,41,27,91,109,58,32,27,91,109,49,57,50,48,120,49,48,56,48,32,64,32,54,48,32,72,122,32,105,110,32,50,52,226,128,179,32,91,69,120,116,101,114,110,97,108,93,13,1027,91,51,52,67,27,91,109,27,91,49,109,27,91,51,51,109,84,101,114,109,105,110,97,108,27,91,109,58,32,27,91,109,102,114,101,109,105,110,97,108,13,10,27,91,51,52,67,13,1027,91,51,52,67,27,91,109,27,91,49,109,27,91,51,51,109,67,80,85,27,91,109,58,32,27,91,109,65,112,112,108,101,32,77,49,32,77,97,120,32,40,49,48,41,32,64,32,51,46,50,51,32,71,72,122,13,10];

    terminal_io.handle_incoming_data(&data);
    let data = terminal_io.data();
    let format_tags = terminal_io.format_data();

    println!("Scrollback:");
    for tag in &format_tags.scrollback {
        let start_pos = tag.start;
        let mut end_pos = tag.end;

        assert!(start_pos < end_pos);

        if end_pos > data.scrollback.len() {
            end_pos = data.scrollback.len();
        }

        // create a string from the data scrollback
        let output = String::from_utf8_lossy(&data.scrollback[start_pos..end_pos]);

        print!("{output}");
    }

    println!("Visible:");
    for tag in &format_tags.visible {
        let start_pos = tag.start;
        let mut end_pos = tag.end;

        assert!(start_pos < end_pos);

        if end_pos > data.visible.len() {
            end_pos = data.visible.len();
        }

        // create a string from the data visible
        let output = String::from_utf8_lossy(&data.visible[start_pos..end_pos]);

        print!("{output}");
    }
    // assert!(0 == 1);
}
