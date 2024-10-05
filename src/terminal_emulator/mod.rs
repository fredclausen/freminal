// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::str;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::{
    fmt,
    fs::File,
    io::Write,
    sync::{Arc, Mutex, MutexGuard},
    thread,
};

mod ansi;
mod buffer;
mod format_tracker;
pub mod io;
pub mod ansi_components {
    pub mod csi;
    pub mod mode;
    pub mod osc;
    pub mod sgr;
}
pub mod error;
pub mod playback;

use crate::{error::backtraced_err, Args};
use ansi::{FreminalAnsiParser, TerminalOutput};
use ansi_components::{
    mode::{BracketedPaste, Decawm, Decckm, Keypad, Mode, TerminalModes},
    osc::{AnsiOscInternalType, AnsiOscType},
    sgr::SelectGraphicRendition,
};
use anyhow::Result;
use buffer::TerminalBufferHolder;
use eframe::{egui::Color32, epaint::text::cursor};
pub use format_tracker::FormatTag;
use format_tracker::FormatTracker;
use io::{pty::{TerminalSize, TerminalWriteCommand}, TerminalRead};
pub use io::{FreminalPtyInputOutput, FreminalTermInputOutput};

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
    pub x_as_characters: usize,
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
    DoubleUnderline,
    Faint,
}

#[derive(Eq, PartialEq, Debug, Clone)]
struct CursorState {
    pos: CursorPos,
    font_weight: FontWeight,
    font_decorations: Vec<FontDecorations>,
    color: TerminalColor,
    background_color: TerminalColor,
}

impl Default for CursorState {
    fn default() -> Self {
        Self {
            pos: CursorPos::default(),
            font_weight: FontWeight::default(),
            font_decorations: Vec::new(),
            color: TerminalColor::Default,
            background_color: TerminalColor::DefaultBackground,
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
            Self::Black | Self::DefaultBackground => "black",
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
            "default_background" => Self::DefaultBackground,
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

struct TermininalEmulatorInternalState {
    parser: FreminalAnsiParser,
    terminal_buffer: TerminalBufferHolder,
    format_tracker: FormatTracker,
    cursor_state: CursorState,
    modes: TerminalModes,
    window_title: Option<String>,
    saved_color_state: Option<(TerminalColor, TerminalColor)>,
}

pub struct TerminalEmulator<Io: FreminalTermInputOutput> {
    io: Arc<Mutex<Io>>,
    rx_reader: Receiver<TerminalRead>,
    tx_sender: Sender<TerminalWriteCommand>,
    recording: Option<File>,
    internal_state: Arc<Mutex<TermininalEmulatorInternalState>>,
}

impl TerminalEmulator<FreminalPtyInputOutput> {
    pub fn new(args: &Args) -> Result<Self> {
        let mut recording = None;

        let (tx_reader, rx_reader) = unbounded::<TerminalRead>();
        let (tx_writer, rx_writer) = unbounded::<TerminalWriteCommand>();
        let io = Arc::new(Mutex::new(FreminalPtyInputOutput::new(args)?));

        // if recording path is some, open a file for writing
        if let Some(path) = &args.recording {
            recording = match std::fs::File::create(path) {
                Ok(file) => Some(file),
                Err(e) => {
                    error!("Failed to create recording file: {}", backtraced_err(&e));
                    None
                }
            }
        }

        // spawn a thread to read from the

        let internal_state = Arc::new(Mutex::new(TermininalEmulatorInternalState {
            parser: FreminalAnsiParser::new(),
            terminal_buffer: TerminalBufferHolder::new(80, 24),
            format_tracker: FormatTracker::new(),
            cursor_state: CursorState::new(),
            modes: TerminalModes::default(),
            window_title: None,
            saved_color_state: None,
        }));

        let internal_rx = rx_reader.clone();
        let internal_internal_state = internal_state.clone();
        let internal_tx = tx_writer.clone();

        thread::spawn(move || {
            read_channel(&internal_rx, internal_internal_state, internal_tx);
        });

        let io_clone = Arc::clone(&io);
        thread::spawn(move || {
            let value =
                io_clone
                    .lock()
                    .unwrap()
                    .set_win_size(TerminalSize::default());
            if let Err(e) = value {
                error!("Failed to set initial window size: {}", backtraced_err(&*e));
            }

            io_clone.lock().unwrap().pty_handler(&rx_writer, &tx_reader);
        });

        let ret = Self {
            io,
            rx_reader,
            tx_sender: tx_writer,
            recording,
            internal_state,
        };
        Ok(ret)
    }
}

impl<Io: FreminalTermInputOutput> TerminalEmulator<Io> {
    pub fn get_win_size(&self) -> (usize, usize) {
        self.internal_state
            .lock()
            .unwrap()
            .terminal_buffer
            .get_win_size()
    }

    pub fn get_window_title(&self) -> Option<String> {
        self.internal_state.lock().unwrap().window_title.clone()
    }

    #[allow(dead_code)]
    pub fn clear_window_title(&self) {
        self.internal_state.lock().unwrap().window_title = None;
    }

    pub fn set_win_size(
        &mut self,
        width_chars: usize,
        height_chars: usize,
        font_width: usize,
        font_height: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sender = self.tx_sender.clone();
        sender.send(TerminalWriteCommand::Resize(TerminalSize {
            cols: width_chars,
            rows: height_chars,
            pixel_width: font_width,
            pixel_height: font_height,
        }));

        Ok(())
    }

    pub fn write(&self, to_write: &TerminalInput) -> Result<(), Box<dyn std::error::Error>> {
        let internal_state = self.internal_state.lock().unwrap();

        write(&internal_state, self.tx_sender.clone(), to_write)
    }

    pub fn data(&self) -> TerminalData<Vec<u8>> {
        let internal_state = match self.internal_state.lock() {
            Ok(state) => TerminalData {
                scrollback: state.terminal_buffer.data().scrollback.to_vec(),
                visible: state.terminal_buffer.data().visible.to_vec(),
            },
            Err(e) => {
                error!("Failed to lock internal state: {}", backtraced_err(&e));
                return TerminalData {
                    scrollback: Vec::new(),
                    visible: Vec::new(),
                };
            }
        };

        internal_state
    }

    pub fn format_data(&self) -> TerminalData<Vec<FormatTag>> {
        let internal_state = self.internal_state.lock().unwrap();

        let offset = internal_state.terminal_buffer.data().scrollback.len();
        split_format_data_for_scrollback(internal_state.format_tracker.tags(), offset)
    }

    pub fn cursor_pos(&self) -> CursorPos {
        let internal_state = self.internal_state.lock().unwrap();
        internal_state.cursor_state.pos.clone()
    }
}

fn reset(internal_state: &mut MutexGuard<'_, TermininalEmulatorInternalState>) {
    internal_state.cursor_state.color = TerminalColor::Default;
    internal_state.cursor_state.background_color = TerminalColor::DefaultBackground;
    internal_state.cursor_state.font_weight = FontWeight::Normal;
    internal_state.cursor_state.font_decorations.clear();
    internal_state.saved_color_state = None;
}

fn set_cursor_pos(
    internal_state: &mut MutexGuard<'_, TermininalEmulatorInternalState>,
    x: Option<usize>,
    y: Option<usize>,
) {
    if let Some(x) = x {
        internal_state.cursor_state.pos.x = x - 1;
    }
    if let Some(y) = y {
        internal_state.cursor_state.pos.y = y - 1;
    }
}

fn set_cursor_pos_rel(
    internal_state: &mut MutexGuard<'_, TermininalEmulatorInternalState>,
    x: Option<i32>,
    y: Option<i32>,
) {
    if let Some(x) = x {
        let x: i64 = x.into();
        let current_x: i64 = internal_state
            .cursor_state
            .pos
            .x
            .try_into()
            .expect("x position larger than i64 can handle");
        internal_state.cursor_state.pos.x = usize::try_from((current_x + x).max(0)).unwrap_or(0);
    }
    if let Some(y) = y {
        let y: i64 = y.into();
        let current_y: i64 = internal_state
            .cursor_state
            .pos
            .y
            .try_into()
            .expect("y position larger than i64 can handle");
        // ensure y is not negative, and throw an error if it is
        internal_state.cursor_state.pos.y = usize::try_from((current_y + y).max(0)).unwrap_or(0);
    }
}

fn clear_forwards(internal_state: &mut MutexGuard<'_, TermininalEmulatorInternalState>) {
    let pos = internal_state.cursor_state.pos.clone();
    if let Some(buf_pos) = internal_state.terminal_buffer.clear_forwards(&pos) {
        let cursor_state = internal_state.cursor_state.clone();
        internal_state
            .format_tracker
            .push_range(&cursor_state, buf_pos..usize::MAX);
    }
}

fn clear_all(internal_state: &mut MutexGuard<'_, TermininalEmulatorInternalState>) {
    let cursor_state = internal_state.cursor_state.clone();
    internal_state
        .format_tracker
        .push_range(&cursor_state, 0..usize::MAX);
    internal_state.terminal_buffer.clear_all();
}

fn clear_line_forwards(internal_state: &mut MutexGuard<'_, TermininalEmulatorInternalState>) {
    let cursor_state = internal_state.cursor_state.clone();
    if let Some(range) = internal_state
        .terminal_buffer
        .clear_line_forwards(&cursor_state.pos)
    {
        internal_state.format_tracker.delete_range(range);
    }
}

fn carriage_return(internal_state: &mut MutexGuard<'_, TermininalEmulatorInternalState>) {
    internal_state.cursor_state.pos.x = 0;
}

fn new_line(internal_state: &mut MutexGuard<'_, TermininalEmulatorInternalState>) {
    internal_state.cursor_state.pos.y = 0;
}

fn backspace(internal_state: &mut MutexGuard<'_, TermininalEmulatorInternalState>) {
    if internal_state.cursor_state.pos.x >= 1 {
        internal_state.cursor_state.pos.x -= 1;
    }
}

fn insert_lines(
    internal_state: &mut MutexGuard<'_, TermininalEmulatorInternalState>,
    num_lines: usize,
) {
    let cursor_state = internal_state.cursor_state.clone();

    let response = internal_state
        .terminal_buffer
        .insert_lines(&cursor_state.pos, num_lines);
    internal_state
        .format_tracker
        .delete_range(response.deleted_range);
    internal_state
        .format_tracker
        .push_range_adjustment(response.inserted_range);
}

fn delete(internal_state: &mut MutexGuard<'_, TermininalEmulatorInternalState>, num_chars: usize) {
    let cursor_state = internal_state.cursor_state.clone();

    let deleted_buf_range = internal_state
        .terminal_buffer
        .delete_forwards(&cursor_state.pos, num_chars);
    if let Some(range) = deleted_buf_range {
        internal_state.format_tracker.delete_range(range);
    }
}

fn font_decordations_add_if_not_contains(
    internal_state: &mut MutexGuard<'_, TermininalEmulatorInternalState>,
    decoration: FontDecorations,
) {
    if !internal_state
        .cursor_state
        .font_decorations
        .contains(&decoration)
    {
        internal_state
            .cursor_state
            .font_decorations
            .push(decoration);
    }
}

fn font_decorations_remove_if_contains(
    internal_state: &mut MutexGuard<'_, TermininalEmulatorInternalState>,
    decoration: &FontDecorations,
) {
    internal_state
        .cursor_state
        .font_decorations
        .retain(|d| *d != *decoration);
}

fn set_foreground(
    internal_state: &mut MutexGuard<'_, TermininalEmulatorInternalState>,
    color: TerminalColor,
) {
    internal_state.cursor_state.color = color;
}

fn set_background(
    internal_state: &mut MutexGuard<'_, TermininalEmulatorInternalState>,
    color: TerminalColor,
) {
    internal_state.cursor_state.background_color = color;
}

fn sgr_process(
    internal_state: &mut MutexGuard<'_, TermininalEmulatorInternalState>,
    sgr: SelectGraphicRendition,
) {
    match sgr {
        SelectGraphicRendition::Reset => reset(internal_state),
        SelectGraphicRendition::Bold => {
            internal_state.cursor_state.font_weight = FontWeight::Bold;
        }
        SelectGraphicRendition::Italic => {
            font_decordations_add_if_not_contains(internal_state, FontDecorations::Italic);
        }
        SelectGraphicRendition::Faint => {
            font_decordations_add_if_not_contains(internal_state, FontDecorations::Faint);
        }
        SelectGraphicRendition::ResetBold => {
            internal_state.cursor_state.font_weight = FontWeight::Normal;
        }
        SelectGraphicRendition::NormalIntensity => {
            font_decorations_remove_if_contains(internal_state, &FontDecorations::Faint);
        }
        SelectGraphicRendition::NotUnderlined => {
            // remove FontDecorations::Underline if it's there
            internal_state.cursor_state.font_decorations.retain(|d| {
                *d != FontDecorations::Underline || *d != FontDecorations::DoubleUnderline
            });
        }
        SelectGraphicRendition::ReverseVideo => {
            let foreground = internal_state.cursor_state.color;
            let background = internal_state.cursor_state.background_color;
            internal_state.saved_color_state = Some((foreground, background));

            internal_state.cursor_state.color = background;
            internal_state.cursor_state.background_color = foreground;
        }
        SelectGraphicRendition::ResetReverseVideo => {
            if let Some((foreground, background)) = internal_state.saved_color_state {
                internal_state.cursor_state.color = foreground;
                internal_state.cursor_state.background_color = background;

                internal_state.saved_color_state = None;
            }
        }
        SelectGraphicRendition::Foreground(color) => set_foreground(internal_state, color),
        SelectGraphicRendition::Background(color) => set_background(internal_state, color),
        SelectGraphicRendition::FastBlink | SelectGraphicRendition::SlowBlink => (),
        SelectGraphicRendition::Unknown(_) => {
            warn!("Unhandled sgr: {:?}", sgr);
        }
    }
}

fn set_mode(internal_state: &mut MutexGuard<'_, TermininalEmulatorInternalState>, mode: &Mode) {
    match mode {
        Mode::Decckm => {
            internal_state.modes.cursor_key = Decckm::Application;
        }
        Mode::Decawm => {
            warn!("Decawm Set is not supported");
            internal_state.modes.autowrap = Decawm::AutoWrap;
        }
        Mode::BracketedPaste => {
            internal_state.modes.bracketed_paste = BracketedPaste::Enabled;
        }
        Mode::Unknown(_) => {
            warn!("unhandled set mode: {mode:?}");
        }
        Mode::Keypad => {
            warn!("Decpam is not supported");
            internal_state.modes.keypad = Keypad::Application;
        }
    }
}

fn insert_spaces(
    internal_state: &mut MutexGuard<'_, TermininalEmulatorInternalState>,
    num_spaces: usize,
) {
    let cursor_state = internal_state.cursor_state.clone();

    let response = internal_state
        .terminal_buffer
        .insert_spaces(&cursor_state.pos, num_spaces);
    internal_state
        .format_tracker
        .push_range_adjustment(response.insertion_range);
}

fn reset_mode(internal_state: &mut MutexGuard<'_, TermininalEmulatorInternalState>, mode: &Mode) {
    match mode {
        Mode::Decckm => {
            internal_state.modes.cursor_key = Decckm::Ansi;
        }
        Mode::Decawm => {
            warn!("Decawm Reset is not supported");
            internal_state.modes.autowrap = Decawm::NoAutoWrap;
        }
        Mode::BracketedPaste => {
            internal_state.modes.bracketed_paste = BracketedPaste::Disabled;
        }
        Mode::Keypad => {
            warn!("Decpam is not supported");
            internal_state.modes.keypad = Keypad::Numeric;
        }
        Mode::Unknown(_) => {}
    }
}

fn osc_response(
    internal_state: &mut MutexGuard<'_, TermininalEmulatorInternalState>,
    io: &Sender<TerminalWriteCommand>,
    osc: AnsiOscType,
) {
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
                        let message = TerminalWriteCommand::Write(vec![*byte]);
                        io.send(message)
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
                        let message = TerminalWriteCommand::Write(vec![*byte]);
                        io.send(message)
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
            internal_state.window_title = Some(title);
        }
        AnsiOscType::Ftcs(value) => {
            warn!("Ftcs is not supported: {value}");
        }
    }
}

fn report_cursor_position(
    internal_state: &MutexGuard<'_, TermininalEmulatorInternalState>,
    io: &Sender<TerminalWriteCommand>,
) {
    let x = internal_state.cursor_state.pos.x + 1;
    let y = internal_state.cursor_state.pos.y + 1;
    let formatted_string = format!("\x1b[{y};{x}R");
    let output = formatted_string.as_bytes();

    for byte in output {
        io.send(TerminalWriteCommand::Write(vec![*byte]))
            .expect("Failed to write cursor position response");
    }
}

fn handle_data(internal_state: &mut MutexGuard<'_, TermininalEmulatorInternalState>, data: &[u8]) {
    let cursor_state = internal_state.cursor_state.clone();

    let response = internal_state
        .terminal_buffer
        .insert_data(&cursor_state.pos, data);

    internal_state
        .format_tracker
        .push_range_adjustment(response.insertion_range);
    internal_state
        .format_tracker
        .push_range(&cursor_state, response.written_range);
    internal_state.cursor_state.pos = response.new_cursor_pos;
}

fn write(
    internal_state: &MutexGuard<'_, TermininalEmulatorInternalState>,
    io: Sender<TerminalWriteCommand>,
    to_write: &TerminalInput,
) -> Result<(), Box<dyn std::error::Error>> {
    match to_write.to_payload(internal_state.modes.cursor_key == Decckm::Application) {
        TerminalInputPayload::Single(c) => {
            let message = TerminalWriteCommand::Write(vec![c]);
            io.send(message);
        }
        TerminalInputPayload::Many(to_write) => {
            while !to_write.is_empty() {
                let message = TerminalWriteCommand::Write(to_write.to_vec());
                io.send(message);
            }
        }
    };

    Ok(())
}

pub fn read_channel(
    rx: &Receiver<TerminalRead>,
    guard: Arc<Mutex<TermininalEmulatorInternalState>>,
    write_channel: Sender<TerminalWriteCommand>,
) {
    while let Ok(TerminalRead {
        buf,
        read: read_size,
    }) = rx.recv()
    {
        info!("in reader");
        // debug!("Read size: {read_size}");

        let incoming = &buf[0..read_size];

        // if let Some(file) = &mut self.recording {
        //     let mut output = String::new();
        //     // loop over the buffer and convert to a string representation of the number, separated by commas
        //     for byte in incoming {
        //         output.push_str(&format!("{byte},"));
        //     }
        //     let _ = file.write_all(output.as_bytes());
        // }
        //debug!("Incoming data: {:?}", std::str::from_utf8(incoming));
        {
            let mut internal_state = guard.lock().unwrap();
            handle_incoming_data(&mut internal_state, write_channel.clone(), incoming);
        }
    }

    info!("out reader");
}

fn handle_incoming_data(
    mut internal_state: &mut MutexGuard<'_, TermininalEmulatorInternalState>,
    write_channel: Sender<TerminalWriteCommand>,
    incoming: &[u8],
) {
    debug!("Incoming data: {:?}", incoming);
    let parsed = internal_state.parser.push(incoming);
    for segment in parsed {
        // if segment is not data, we want to print out the segment
        if let TerminalOutput::Data(data) = &segment {
            debug!("Incoming data: {:?}", str::from_utf8(data).unwrap());
        } else {
            debug!("Incoming segment: {:?}", segment);
        }

        match segment {
            TerminalOutput::Data(data) => handle_data(internal_state, &data),
            TerminalOutput::SetCursorPos { x, y } => set_cursor_pos(internal_state, x, y),
            TerminalOutput::SetCursorPosRel { x, y } => set_cursor_pos_rel(internal_state, x, y),
            TerminalOutput::ClearForwards => clear_forwards(internal_state),
            TerminalOutput::ClearAll => clear_all(internal_state),
            TerminalOutput::ClearLineForwards => clear_line_forwards(internal_state),
            TerminalOutput::CarriageReturn => carriage_return(internal_state),
            TerminalOutput::Newline => new_line(internal_state),
            TerminalOutput::Backspace => backspace(internal_state),
            TerminalOutput::InsertLines(num_lines) => insert_lines(internal_state, num_lines),
            TerminalOutput::Delete(num_chars) => delete(internal_state, num_chars),
            TerminalOutput::Sgr(sgr) => sgr_process(internal_state, sgr),
            TerminalOutput::SetMode(mode) => set_mode(internal_state, &mode),
            TerminalOutput::InsertSpaces(num_spaces) => insert_spaces(internal_state, num_spaces),
            TerminalOutput::ResetMode(mode) => reset_mode(internal_state, &mode),
            TerminalOutput::OscResponse(osc) => {
                osc_response(internal_state, &write_channel.clone(), osc);
            }
            TerminalOutput::CursorReport => {
                report_cursor_position(internal_state, &write_channel.clone());
            }
            TerminalOutput::Bell | TerminalOutput::Invalid => {
                info!("Unhandled terminal output: {segment:?}");
            }
        }
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
                font_weight: FontWeight::Normal,
                font_decorations: Vec::new(),
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

    #[test]
    fn test_format_tracker_scrollback_no_split() {
        let tags = get_tags();
        // Case 1: no split
        let res = split_format_data_for_scrollback(tags.clone(), 0);
        assert_eq!(res.scrollback, &[]);
        assert_eq!(res.visible, &tags[..]);
    }
}
