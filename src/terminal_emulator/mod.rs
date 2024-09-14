// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::fmt;

use ansi::{FreminalAnsiParser, SelectGraphicRendition, TerminalOutput};
use buffer::TerminalBufferHolder;
use format_tracker::FormatTracker;

pub use format_tracker::FormatTag;
pub use io::{FreminalPtyInputOutput, FreminalTermInputOutput};

use crate::{error::backtraced_err, terminal_emulator::io::ReadResponse};

use self::io::CreatePtyIoError;

mod ansi;
mod buffer;
mod format_tracker;
mod io;

#[derive(Eq, PartialEq)]
enum Mode {
    // Cursor keys mode
    // https://vt100.net/docs/vt100-ug/chapter3.html
    Decckm,
    Unknown(Vec<u8>),
}

impl fmt::Debug for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decckm => f.write_str("Decckm"),
            Self::Unknown(params) => {
                let params_s = std::str::from_utf8(params)
                    .expect("parameter parsing should not allow non-utf8 characters here");
                f.write_fmt(format_args!("Unknown({params_s})"))
            }
        }
    }
}

const fn char_to_ctrl_code(c: u8) -> u8 {
    // https://catern.com/posts/terminal_quirks.html
    // man ascii
    c & 0b0001_1111
}

#[derive(Eq, PartialEq, Debug)]
enum TerminalInputPayload {
    Single(u8),
    Many(&'static [u8]),
}

#[derive(Clone)]
pub enum TerminalInput {
    // Normal keypress
    Ascii(u8),
    // Normal keypress with ctrl
    Ctrl(u8),
    Enter,
    Backspace,
    ArrowRight,
    ArrowLeft,
    ArrowUp,
    ArrowDown,
    Home,
    End,
    Delete,
    Insert,
    PageUp,
    PageDown,
}

impl TerminalInput {
    const fn to_payload(&self, decckm_mode: bool) -> TerminalInputPayload {
        match self {
            Self::Ascii(c) => TerminalInputPayload::Single(*c),
            Self::Ctrl(c) => TerminalInputPayload::Single(char_to_ctrl_code(*c)),
            Self::Enter => TerminalInputPayload::Single(b'\n'),
            // Hard to tie back, but check default VERASE in terminfo definition
            Self::Backspace => TerminalInputPayload::Single(0x7f),
            // https://vt100.net/docs/vt100-ug/chapter3.html
            // Table 3-6
            Self::ArrowRight => {
                if decckm_mode {
                    TerminalInputPayload::Many(b"\x1bOC")
                } else {
                    TerminalInputPayload::Many(b"\x1b[C")
                }
            }
            Self::ArrowLeft => {
                if decckm_mode {
                    TerminalInputPayload::Many(b"\x1bOD")
                } else {
                    TerminalInputPayload::Many(b"\x1b[D")
                }
            }
            Self::ArrowUp => {
                if decckm_mode {
                    TerminalInputPayload::Many(b"\x1bOA")
                } else {
                    TerminalInputPayload::Many(b"\x1b[A")
                }
            }
            Self::ArrowDown => {
                if decckm_mode {
                    TerminalInputPayload::Many(b"\x1bOB")
                } else {
                    TerminalInputPayload::Many(b"\x1b[B")
                }
            }
            Self::Home => {
                if decckm_mode {
                    TerminalInputPayload::Many(b"\x1bOH")
                } else {
                    TerminalInputPayload::Many(b"\x1b[H")
                }
            }
            Self::End => {
                if decckm_mode {
                    TerminalInputPayload::Many(b"\x1bOF")
                } else {
                    TerminalInputPayload::Many(b"\x1b[F")
                }
            }
            // Why \e[3~? It seems like we are emulating the vt510. Other terminals do it, so we
            // can too
            // https://web.archive.org/web/20160304024035/http://www.vt100.net/docs/vt510-rm/chapter8
            // https://en.wikipedia.org/wiki/Delete_character
            Self::Delete => TerminalInputPayload::Many(b"\x1b[3~"),
            Self::Insert => TerminalInputPayload::Many(b"\x1b[2~"),
            Self::PageUp => TerminalInputPayload::Many(b"\x1b[5~"),
            Self::PageDown => TerminalInputPayload::Many(b"\x1b[6~"),
        }
    }
}

fn split_format_data_for_scrollback(
    tags: Vec<FormatTag>,
    scrollback_split: usize,
) -> TerminalData<Vec<FormatTag>> {
    let scrollback_tags = tags
        .iter()
        .filter(|tag| tag.start < scrollback_split)
        .cloned()
        .map(|mut tag| {
            tag.end = tag.end.min(scrollback_split);
            tag
        })
        .collect();

    let canvas_tags = tags
        .into_iter()
        .filter(|tag| tag.end > scrollback_split)
        .map(|mut tag| {
            tag.start = tag.start.saturating_sub(scrollback_split);
            if tag.end != usize::MAX {
                tag.end -= scrollback_split;
            }
            tag
        })
        .collect();

    TerminalData {
        scrollback: scrollback_tags,
        visible: canvas_tags,
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CursorPos {
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FontWeight {
    Normal,
    Bold,
    Faint,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FontDecorations {
    Italic,
    Underline,
    DoubleUnderline,
}

#[derive(Eq, PartialEq, Debug, Clone)]
struct CursorState {
    pos: CursorPos,
    font_weight: FontWeight,
    font_decorations: Vec<FontDecorations>,
    color: TerminalColor,
    background_color: TerminalColor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalColor {
    Default,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightYellow,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    Custom(u8, u8, u8),
}

impl fmt::Display for TerminalColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Default => "default",
            Self::Black => "black",
            Self::Red => "red",
            Self::Green => "green",
            Self::Yellow => "yellow",
            Self::Blue => "blue",
            Self::Magenta => "magenta",
            Self::Cyan => "cyan",
            Self::White => "white",
            Self::BrightYellow => "bright yellow",
            Self::BrightBlack => "bright black",
            Self::BrightRed => "bright red",
            Self::BrightGreen => "bright green",
            Self::BrightBlue => "bright blue",
            Self::BrightMagenta => "bright magenta",
            Self::BrightCyan => "bright cyan",
            Self::BrightWhite => "bright white",
            Self::Custom(r, g, b) => {
                return write!(f, "rgb({r}, {g}, {b})");
            }
        };

        f.write_str(s)
    }
}

impl std::str::FromStr for TerminalColor {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ret = match s {
            "default" => Self::Default,
            "black" => Self::Black,
            "red" => Self::Red,
            "green" => Self::Green,
            "yellow" => Self::Yellow,
            "blue" => Self::Blue,
            "magenta" => Self::Magenta,
            "cyan" => Self::Cyan,
            "white" => Self::White,
            "bright yellow" => Self::BrightYellow,
            "bright black" => Self::BrightBlack,
            "bright red" => Self::BrightRed,
            "bright green" => Self::BrightGreen,
            "bright blue" => Self::BrightBlue,
            "bright magenta" => Self::BrightMagenta,
            "bright cyan" => Self::BrightCyan,
            "bright white" => Self::BrightWhite,
            _ => return Err(()),
        };
        Ok(ret)
    }
}

impl TerminalColor {
    fn from_sgr(sgr: SelectGraphicRendition) -> Option<Self> {
        let ret = match sgr {
            SelectGraphicRendition::ForegroundBlack => Self::Black,
            SelectGraphicRendition::ForegroundRed => Self::Red,
            SelectGraphicRendition::ForegroundGreen => Self::Green,
            SelectGraphicRendition::ForegroundYellow => Self::Yellow,
            SelectGraphicRendition::ForegroundBlue => Self::Blue,
            SelectGraphicRendition::ForegroundMagenta => Self::Magenta,
            SelectGraphicRendition::ForegroundCyan => Self::Cyan,
            SelectGraphicRendition::ForegroundWhite => Self::White,
            SelectGraphicRendition::ForegroundBrightBlack => Self::BrightBlack,
            SelectGraphicRendition::ForegroundBrightRed => Self::BrightRed,
            SelectGraphicRendition::ForegroundBrightGreen => Self::BrightGreen,
            SelectGraphicRendition::ForegroundBrightYellow => Self::BrightYellow,
            SelectGraphicRendition::ForegroundBrightBlue => Self::BrightBlue,
            SelectGraphicRendition::ForegroundBrightMagenta => Self::BrightMagenta,
            SelectGraphicRendition::ForegroundBrightCyan => Self::BrightCyan,
            SelectGraphicRendition::ForegroundBrightWhite => Self::BrightWhite,
            SelectGraphicRendition::ForegroundCustom(r, g, b) => {
                let r = u8::try_from(r).ok()?;
                let g = u8::try_from(g).ok()?;
                let b = u8::try_from(b).ok()?;

                Self::Custom(r, g, b)
            }
            _ => return None,
        };

        Some(ret)
    }
}

pub struct TerminalData<T> {
    pub scrollback: T,
    pub visible: T,
}

pub struct TerminalEmulator<Io: FreminalTermInputOutput> {
    parser: FreminalAnsiParser,
    terminal_buffer: TerminalBufferHolder,
    format_tracker: FormatTracker,
    cursor_state: CursorState,
    decckm_mode: bool,
    io: Io,
}

pub const TERMINAL_WIDTH: usize = 50;
pub const TERMINAL_HEIGHT: usize = 16;

impl TerminalEmulator<FreminalPtyInputOutput> {
    pub fn new() -> Result<Self, CreatePtyIoError> {
        let mut io = FreminalPtyInputOutput::new()?;

        if let Err(e) = io.set_win_size(TERMINAL_WIDTH, TERMINAL_HEIGHT) {
            error!("Failed to set initial window size: {}", backtraced_err(&*e));
        }

        let ret = Self {
            parser: FreminalAnsiParser::new(),
            terminal_buffer: TerminalBufferHolder::new(TERMINAL_WIDTH, TERMINAL_HEIGHT),
            format_tracker: FormatTracker::new(),
            decckm_mode: false,
            cursor_state: CursorState {
                pos: CursorPos { x: 0, y: 0 },
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                color: TerminalColor::Default,
                background_color: TerminalColor::Black,
            },
            io,
        };
        Ok(ret)
    }
}

impl<Io: FreminalTermInputOutput> TerminalEmulator<Io> {
    pub const fn get_win_size(&self) -> (usize, usize) {
        self.terminal_buffer.get_win_size()
    }

    pub fn set_win_size(
        &mut self,
        width_chars: usize,
        height_chars: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let response =
            self.terminal_buffer
                .set_win_size(width_chars, height_chars, &self.cursor_state.pos);
        self.cursor_state.pos = response.new_cursor_pos;

        if response.changed {
            self.io.set_win_size(width_chars, height_chars)?;
        }

        Ok(())
    }

    pub fn write(&mut self, to_write: &TerminalInput) -> Result<(), Box<dyn std::error::Error>> {
        match to_write.to_payload(self.decckm_mode) {
            TerminalInputPayload::Single(c) => {
                let mut written = 0;
                while written == 0 {
                    written = self.io.write(&[c])?;
                }
            }
            TerminalInputPayload::Many(mut to_write) => {
                while !to_write.is_empty() {
                    let written = self.io.write(to_write)?;
                    to_write = &to_write[written..];
                }
            }
        };
        Ok(())
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
            SelectGraphicRendition::ResetBold | SelectGraphicRendition::NormalIntensity => {
                self.cursor_state.font_weight = FontWeight::Normal;
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
                self.cursor_state.color = background;
                self.cursor_state.background_color = foreground;
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
            SelectGraphicRendition::DefaultBackground => {
                self.cursor_state.background_color = TerminalColor::Black;
            }
            SelectGraphicRendition::BackgroundBlack => {
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
                self.decckm_mode = true;
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
                self.decckm_mode = false;
            }
            Mode::Unknown(_) => {}
        }
    }

    fn handle_incoming_data(&mut self, incoming: &[u8]) {
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
                TerminalOutput::Bell | TerminalOutput::Invalid => (),
            }
        }
    }

    pub fn read(&mut self) {
        let mut buf = vec![0u8; 4096];
        loop {
            let read_size = match self.io.read(&mut buf) {
                Ok(ReadResponse::Empty) => break,
                Ok(ReadResponse::Success(v)) => v,
                Err(e) => {
                    error!("Failed to read from child process: {e}");
                    break;
                }
            };

            let incoming = &buf[0..read_size];
            debug!("Incoming data: {:?}", std::str::from_utf8(incoming));
            self.handle_incoming_data(incoming);
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_format_tracker_scrollback_split() {
        let tags = vec![
            FormatTag {
                start: 0,
                end: 5,
                color: TerminalColor::Blue,
                background_color: TerminalColor::Black,
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
            },
            FormatTag {
                start: 5,
                end: 7,
                color: TerminalColor::Red,
                background_color: TerminalColor::Black,
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
            },
            FormatTag {
                start: 7,
                end: 10,
                color: TerminalColor::Blue,
                background_color: TerminalColor::Black,
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
            },
            FormatTag {
                start: 10,
                end: usize::MAX,
                color: TerminalColor::Red,
                background_color: TerminalColor::Black,
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
            },
        ];

        // Case 1: no split
        let res = split_format_data_for_scrollback(tags.clone(), 0);
        assert_eq!(res.scrollback, &[]);
        assert_eq!(res.visible, &tags[..]);

        // Case 2: Split on a boundary
        let res = split_format_data_for_scrollback(tags.clone(), 10);
        assert_eq!(res.scrollback, &tags[0..3]);
        assert_eq!(
            res.visible,
            &[FormatTag {
                start: 0,
                end: usize::MAX,
                color: TerminalColor::Red,
                background_color: TerminalColor::Black,
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
            },]
        );

        // Case 3: Split a segment
        let res = split_format_data_for_scrollback(tags, 9);
        assert_eq!(
            res.scrollback,
            &[
                FormatTag {
                    start: 0,
                    end: 5,
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::Black,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 5,
                    end: 7,
                    color: TerminalColor::Red,
                    background_color: TerminalColor::Black,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 7,
                    end: 9,
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::Black,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
            ]
        );
        assert_eq!(
            res.visible,
            &[
                FormatTag {
                    start: 0,
                    end: 1,
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::Black,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
                FormatTag {
                    start: 1,
                    end: usize::MAX,
                    color: TerminalColor::Red,
                    background_color: TerminalColor::Black,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                },
            ]
        );
    }
}
