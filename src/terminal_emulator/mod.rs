// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::str;
use std::fmt;

mod ansi;
mod buffer;
mod format_tracker;
mod io;
pub mod ansi_components {
    pub mod csi;
    pub mod mode;
    pub mod osc;
    pub mod sgr;
}
pub mod error;
pub mod playback;
pub mod term_char;

use ansi::{FreminalAnsiParser, TerminalOutput};
use ansi_components::{
    mode::{BracketedPaste, Decawm, Decckm, Mode, TerminalModes},
    osc::{AnsiOscInternalType, AnsiOscType},
    sgr::SelectGraphicRendition,
};
use anyhow::Result;
use buffer::TerminalBufferHolder;
use crossbeam_channel::unbounded;
use eframe::egui::Color32;
pub use format_tracker::FormatTag;
use format_tracker::FormatTracker;
pub use io::{FreminalPtyInputOutput, FreminalTermInputOutput};
use io::{FreminalTerminalSize, PtyRead, PtyWrite};
use term_char::TChar;

use crate::Args;

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

#[derive(Clone, Debug)]
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

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CursorPos {
    pub x: usize,
    pub y: usize,
    // pub x_as_characters: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub enum FontWeight {
    #[default]
    Normal,
    Bold,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FontDecorations {
    Italic,
    Underline,
    Faint,
    Strikethrough,
}

#[derive(Eq, PartialEq, Debug, Clone)]
struct CursorState {
    pos: CursorPos,
    font_weight: FontWeight,
    font_decorations: Vec<FontDecorations>,
    color: TerminalColor,
    background_color: TerminalColor,
    underline_color: TerminalColor,
    line_wrap_mode: Decawm,
}

impl Default for CursorState {
    fn default() -> Self {
        Self {
            pos: CursorPos::default(),
            font_weight: FontWeight::default(),
            font_decorations: Vec::new(),
            color: TerminalColor::Default,
            background_color: TerminalColor::DefaultBackground,
            underline_color: TerminalColor::DefaultUnderlineColor,
            line_wrap_mode: Decawm::default(),
        }
    }
}

// FIXME: it would be cool to not lint this out
#[allow(dead_code)]
impl CursorState {
    fn new() -> Self {
        Self::default()
    }

    const fn with_background_color(mut self, background_color: TerminalColor) -> Self {
        self.background_color = background_color;
        self
    }

    const fn with_color(mut self, color: TerminalColor) -> Self {
        self.color = color;
        self
    }

    const fn with_font_weight(mut self, font_weight: FontWeight) -> Self {
        self.font_weight = font_weight;
        self
    }

    fn with_font_decorations(mut self, font_decorations: Vec<FontDecorations>) -> Self {
        self.font_decorations = font_decorations;
        self
    }

    const fn with_pos(mut self, pos: CursorPos) -> Self {
        self.pos = pos;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalColor {
    Default,
    DefaultBackground,
    DefaultUnderlineColor,
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
            Self::DefaultUnderlineColor => "default underline color",
            Self::DefaultBackground => "default background",
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
            "default_background" => Self::DefaultBackground,
            "default_underline_color" => Self::DefaultUnderlineColor,
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

pub struct TerminalData<T> {
    pub scrollback: T,
    pub visible: T,
}

pub struct TerminalEmulator<Io: FreminalTermInputOutput> {
    parser: FreminalAnsiParser,
    terminal_buffer: TerminalBufferHolder,
    format_tracker: FormatTracker,
    cursor_state: CursorState,
    modes: TerminalModes,
    _io: Io,
    write_tx: crossbeam_channel::Sender<PtyWrite>,
    pty_rx: crossbeam_channel::Receiver<PtyRead>,
    window_title: Option<String>,
    saved_color_state: Option<(TerminalColor, TerminalColor, TerminalColor)>,
}

pub const TERMINAL_WIDTH: usize = 50;
pub const TERMINAL_HEIGHT: usize = 16;

impl TerminalEmulator<FreminalPtyInputOutput> {
    pub fn new(args: &Args) -> Result<Self> {
        let (write_tx, read_rx) = unbounded();
        let (pty_tx, pty_rx) = unbounded();

        let io = FreminalPtyInputOutput::new(
            read_rx,
            pty_tx,
            args.recording.clone(),
            args.shell.clone(),
        )?;

        if let Err(e) = write_tx.send(PtyWrite::Resize(FreminalTerminalSize {
            width: TERMINAL_WIDTH,
            height: TERMINAL_HEIGHT,
            pixel_width: 0,
            pixel_height: 0,
        })) {
            error!("Failed to send resize to pty: {e}");
        }

        let ret = Self {
            parser: FreminalAnsiParser::new(),
            terminal_buffer: TerminalBufferHolder::new(TERMINAL_WIDTH, TERMINAL_HEIGHT),
            format_tracker: FormatTracker::new(),
            modes: TerminalModes {
                cursor_key: Decckm::default(),
                bracketed_paste: BracketedPaste::default(),
            },
            cursor_state: CursorState {
                pos: CursorPos::default(),
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                color: TerminalColor::Default,
                background_color: TerminalColor::Black,
                underline_color: TerminalColor::DefaultUnderlineColor,
                line_wrap_mode: Decawm::default(),
            },
            _io: io,
            write_tx,
            pty_rx,
            window_title: None,
            saved_color_state: None,
        };
        Ok(ret)
    }
}

impl<Io: FreminalTermInputOutput> TerminalEmulator<Io> {
    pub const fn get_win_size(&self) -> (usize, usize) {
        self.terminal_buffer.get_win_size()
    }

    pub fn get_window_title(&self) -> Option<String> {
        self.window_title.clone()
    }

    #[allow(dead_code)]
    pub fn clear_window_title(&mut self) {
        self.window_title = None;
    }

    pub fn set_win_size(
        &mut self,
        width_chars: usize,
        height_chars: usize,
        font_pixel_width: usize,
        font_pixel_height: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let response =
            self.terminal_buffer
                .set_win_size(width_chars, height_chars, &self.cursor_state.pos);
        self.cursor_state.pos = response.new_cursor_pos;

        if response.changed {
            self.write_tx.send(PtyWrite::Resize(FreminalTerminalSize {
                width: width_chars,
                height: height_chars,
                pixel_width: font_pixel_width,
                pixel_height: font_pixel_height,
            }))?;
        }

        Ok(())
    }

    pub fn write(&self, to_write: &TerminalInput) -> Result<(), Box<dyn std::error::Error>> {
        match to_write.to_payload(self.modes.cursor_key == Decckm::Application) {
            TerminalInputPayload::Single(c) => {
                self.write_tx.send(PtyWrite::Write(vec![c]))?;
            }
            TerminalInputPayload::Many(to_write) => {
                self.write_tx.send(PtyWrite::Write(to_write.to_vec()))?;
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
        } else {
            // FIXME: this is not correct, we should move to the end of the previous line
            warn!("FIXME: Backspace at the beginning of the line. Not wrapping");
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

    fn reset(&mut self) {
        self.cursor_state.color = TerminalColor::Default;
        self.cursor_state.background_color = TerminalColor::DefaultBackground;
        self.cursor_state.underline_color = TerminalColor::DefaultUnderlineColor;
        self.cursor_state.font_weight = FontWeight::Normal;
        self.cursor_state.font_decorations.clear();
        self.saved_color_state = None;
    }

    fn font_decordations_add_if_not_contains(&mut self, decoration: FontDecorations) {
        if !self.cursor_state.font_decorations.contains(&decoration) {
            self.cursor_state.font_decorations.push(decoration);
        }
    }

    fn font_decorations_remove_if_contains(&mut self, decoration: &FontDecorations) {
        self.cursor_state
            .font_decorations
            .retain(|d| *d != *decoration);
    }

    fn set_foreground(&mut self, color: TerminalColor) {
        self.cursor_state.color = color;
    }

    fn set_background(&mut self, color: TerminalColor) {
        self.cursor_state.background_color = color;
    }

    fn set_underline_color(&mut self, color: TerminalColor) {
        self.cursor_state.underline_color = color;
    }

    fn sgr(&mut self, sgr: SelectGraphicRendition) {
        match sgr {
            SelectGraphicRendition::Reset => self.reset(),
            SelectGraphicRendition::Bold => {
                self.cursor_state.font_weight = FontWeight::Bold;
            }
            SelectGraphicRendition::Underline => {
                self.font_decordations_add_if_not_contains(FontDecorations::Underline);
            }
            SelectGraphicRendition::Italic => {
                self.font_decordations_add_if_not_contains(FontDecorations::Italic);
            }
            SelectGraphicRendition::NotItalic => {
                self.font_decorations_remove_if_contains(&FontDecorations::Italic);
            }
            SelectGraphicRendition::Faint => {
                self.font_decordations_add_if_not_contains(FontDecorations::Faint);
            }
            SelectGraphicRendition::ResetBold => {
                self.cursor_state.font_weight = FontWeight::Normal;
            }
            SelectGraphicRendition::NormalIntensity => {
                self.font_decorations_remove_if_contains(&FontDecorations::Faint);
            }
            SelectGraphicRendition::NotUnderlined => {
                self.font_decorations_remove_if_contains(&FontDecorations::Underline);
            }
            SelectGraphicRendition::Strikethrough => {
                self.font_decordations_add_if_not_contains(FontDecorations::Strikethrough);
            }
            SelectGraphicRendition::NotStrikethrough => {
                self.font_decorations_remove_if_contains(&FontDecorations::Strikethrough);
            }
            SelectGraphicRendition::ReverseVideo => {
                let mut foreground = self.cursor_state.color;
                let mut background = self.cursor_state.background_color;
                let mut underline = self.cursor_state.underline_color;

                self.saved_color_state = Some((foreground, background, underline));

                if foreground == TerminalColor::Default {
                    foreground = TerminalColor::White;
                }

                if underline == TerminalColor::DefaultUnderlineColor {
                    underline = foreground;
                }

                if background == TerminalColor::DefaultBackground {
                    background = TerminalColor::Black;
                }

                self.cursor_state.color = background;
                self.cursor_state.background_color = foreground;
                self.cursor_state.underline_color = underline;
            }
            SelectGraphicRendition::ResetReverseVideo => {
                if let Some((foreground, background, underline)) = self.saved_color_state {
                    self.cursor_state.color = foreground;
                    self.cursor_state.background_color = background;
                    self.cursor_state.underline_color = underline;

                    self.saved_color_state = None;
                }
            }
            SelectGraphicRendition::Foreground(color) => self.set_foreground(color),
            SelectGraphicRendition::Background(color) => self.set_background(color),
            SelectGraphicRendition::UnderlineColor(color) => self.set_underline_color(color),
            SelectGraphicRendition::FastBlink
            | SelectGraphicRendition::SlowBlink
            | SelectGraphicRendition::NotBlinking
            | SelectGraphicRendition::Conceal
            | SelectGraphicRendition::PrimaryFont
            | SelectGraphicRendition::AlternativeFont1
            | SelectGraphicRendition::AlternativeFont2
            | SelectGraphicRendition::AlternativeFont3
            | SelectGraphicRendition::AlternativeFont4
            | SelectGraphicRendition::AlternativeFont5
            | SelectGraphicRendition::AlternativeFont6
            | SelectGraphicRendition::AlternativeFont7
            | SelectGraphicRendition::AlternativeFont8
            | SelectGraphicRendition::AlternativeFont9
            | SelectGraphicRendition::FontFranktur
            | SelectGraphicRendition::ProportionnalSpacing
            | SelectGraphicRendition::DisableProportionnalSpacing
            | SelectGraphicRendition::Framed
            | SelectGraphicRendition::Encircled
            | SelectGraphicRendition::Overlined
            | SelectGraphicRendition::NotOverlined
            | SelectGraphicRendition::NotFramedOrEncircled
            | SelectGraphicRendition::IdeogramUnderline
            | SelectGraphicRendition::IdeogramDoubleUnderline
            | SelectGraphicRendition::IdeogramOverline
            | SelectGraphicRendition::IdeogramDoubleOverline
            | SelectGraphicRendition::IdeogramStress
            | SelectGraphicRendition::IdeogramAttributes
            | SelectGraphicRendition::Superscript
            | SelectGraphicRendition::Subscript
            | SelectGraphicRendition::NeitherSuperscriptNorSubscript
            | SelectGraphicRendition::Revealed => {
                warn!("Unhandled sgr: {:?}", sgr);
            }
            SelectGraphicRendition::Unknown(_) => {
                warn!("Unknown sgr: {:?}", sgr);
            }
        }
    }

    fn set_mode(&mut self, mode: &Mode) {
        match mode {
            Mode::Decckm => {
                self.modes.cursor_key = Decckm::Application;
            }
            Mode::Decawm => {
                self.cursor_state.line_wrap_mode = Decawm::AutoWrap;
            }
            Mode::BracketedPaste => {
                warn!("BracketedPaste Set is not supported");
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
                self.cursor_state.line_wrap_mode = Decawm::NoAutoWrap;
            }
            Mode::BracketedPaste => {
                warn!("BracketedPaste Reset is not supported");
                self.modes.bracketed_paste = BracketedPaste::Disabled;
            }
            Mode::Unknown(_) => {}
        }
    }

    fn osc_response(&mut self, osc: AnsiOscType) {
        match osc {
            AnsiOscType::RequestColorQueryBackground(color) => {
                match color {
                    // OscInternalType::SetColor(_) => {
                    //     warn!("RequestColorQueryBackground: Set is not supported");
                    // }
                    AnsiOscInternalType::Query => {
                        // lets get the color as a hex string

                        let (r, g, b, a) = Color32::BLACK.to_tuple();

                        let formatted_string =
                            format!("\x1b]11;rgb:{r:02x}/{g:02x}/{b:02x}{a:02x}\x1b\\");
                        let output = formatted_string.as_bytes();

                        for byte in output {
                            self.write(&TerminalInput::Ascii(*byte))
                                .expect("Failed to write osc color response");
                        }
                    }
                    AnsiOscInternalType::Unknown(_) => {
                        warn!("OSC Unknown is not supported");
                    }
                    AnsiOscInternalType::String(_) => {
                        warn!("OSC Type {color:?} Skipped");
                    }
                }
            }
            AnsiOscType::RequestColorQueryForeground(color) => {
                match color {
                    // OscInternalType::SetColor(_) => {
                    //     warn!("RequestColorQueryForeground: Set is not supported");
                    // }
                    AnsiOscInternalType::Query => {
                        // lets get the color as a hex string
                        let (r, g, b, a) = Color32::WHITE.to_tuple();

                        let formatted_string =
                            format!("\x1b]10;rgb:{r:02x}/{g:02x}/{b:02x}{a:02x}\x1b\\");

                        let output = formatted_string.as_bytes();

                        for byte in output {
                            self.write(&TerminalInput::Ascii(*byte))
                                .expect("Failed to write osc color response");
                        }
                    }
                    AnsiOscInternalType::Unknown(_) => {
                        warn!("OSC Unknown is not supported");
                    }
                    AnsiOscInternalType::String(_) => {
                        warn!("OSC Type {color:?} Skipped");
                    }
                }
            }
            AnsiOscType::SetTitleBar(title) => {
                self.window_title = Some(title);
            }
            AnsiOscType::Ftcs(value) => {
                warn!("Ftcs is not supported: {value}");
            }
        }
    }

    fn report_cursor_position(&self) {
        let x = self.cursor_state.pos.x + 1;
        let y = self.cursor_state.pos.y + 1;
        let formatted_string = format!("\x1b[{y};{x}R");
        let output = formatted_string.as_bytes();

        for byte in output {
            self.write(&TerminalInput::Ascii(*byte))
                .expect("Failed to write cursor position report");
        }
    }

    fn handle_incoming_data(&mut self, incoming: &[u8]) {
        let parsed = self.parser.push(incoming);
        for segment in parsed {
            // if segment is not data, we want to print out the segment
            if let TerminalOutput::Data(data) = &segment {
                debug!("Incoming data: {:?}", str::from_utf8(data).unwrap());
            } else {
                debug!("Incoming segment: {:?}", segment);
            }

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
                TerminalOutput::OscResponse(osc) => self.osc_response(osc),
                TerminalOutput::CursorReport => self.report_cursor_position(),
                TerminalOutput::Skipped => (),
                TerminalOutput::Bell
                | TerminalOutput::Invalid
                | TerminalOutput::ApplicationKeypadMode
                | TerminalOutput::NormalKeypadMode => {
                    info!("Unhandled terminal output: {segment:?}");
                }
            }
        }
    }

    pub fn read(&mut self) {
        while let Ok(read) = self.pty_rx.try_recv() {
            let incoming = &read.buf[0..read.read_amount];
            self.handle_incoming_data(incoming);
        }
        // loop {
        //     let read = self.pty_rx.try_recv();

        //     let incoming = &buf[0..read_size];

        //     if let Some(file) = &mut self.recording {
        //         let mut output = String::new();
        //         // loop over the buffer and convert to a string representation of the number, separated by commas
        //         for byte in incoming {
        //             output.push_str(&format!("{byte},"));
        //         }
        //         let _ = file.write_all(output.as_bytes());
        //     }
        //     //debug!("Incoming data: {:?}", std::str::from_utf8(incoming));
        //     self.handle_incoming_data(incoming);
        // }
    }

    pub fn data(&self) -> TerminalData<&[TChar]> {
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

    fn get_tags() -> Vec<FormatTag> {
        vec![
            FormatTag {
                start: 0,
                end: 5,
                color: TerminalColor::Blue,
                background_color: TerminalColor::Black,
                underline_color: TerminalColor::DefaultUnderlineColor,
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
            },
            FormatTag {
                start: 5,
                end: 7,
                color: TerminalColor::Red,
                background_color: TerminalColor::Black,
                underline_color: TerminalColor::DefaultUnderlineColor,
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
            },
            FormatTag {
                start: 7,
                end: 10,
                color: TerminalColor::Blue,
                background_color: TerminalColor::Black,
                underline_color: TerminalColor::DefaultUnderlineColor,
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
            },
            FormatTag {
                start: 10,
                end: usize::MAX,
                color: TerminalColor::Red,
                background_color: TerminalColor::Black,
                underline_color: TerminalColor::DefaultUnderlineColor,
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
            },
        ]
    }

    #[test]
    fn test_format_tracker_scrollback_split_on_boundary() {
        let tags = get_tags();
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
                underline_color: TerminalColor::DefaultUnderlineColor,
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
                line_wrap_mode: Decawm::default(),
            },]
        );
    }

    #[test]
    fn test_format_tracker_scrollback_split_segment() {
        let tags = get_tags();

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
                    underline_color: TerminalColor::DefaultUnderlineColor,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 5,
                    end: 7,
                    color: TerminalColor::Red,
                    background_color: TerminalColor::Black,
                    underline_color: TerminalColor::DefaultUnderlineColor,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 7,
                    end: 9,
                    color: TerminalColor::Blue,
                    background_color: TerminalColor::Black,
                    underline_color: TerminalColor::DefaultUnderlineColor,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
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
                    underline_color: TerminalColor::DefaultUnderlineColor,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
                FormatTag {
                    start: 1,
                    end: usize::MAX,
                    color: TerminalColor::Red,
                    background_color: TerminalColor::Black,
                    underline_color: TerminalColor::DefaultUnderlineColor,
                    font_weight: FontWeight::Normal,
                    font_decorations: Vec::new(),
                    line_wrap_mode: Decawm::default(),
                },
            ]
        );
    }

    #[test]
    fn test_format_tracker_scrollback_no_split() {
        let tags = get_tags();
        // Case 1: no split
        let res = split_format_data_for_scrollback(tags.clone(), 0);
        assert_eq!(res.scrollback, &[]);
        assert_eq!(res.visible, &tags[..]);
    }
}
