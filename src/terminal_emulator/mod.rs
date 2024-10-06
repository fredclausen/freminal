// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use core::str;
use path_clean::PathClean;
use std::{
    env, fmt,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::{mpsc::{self, Receiver, Sender}, Mutex, MutexGuard};

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
use eframe::egui::Color32;
pub use format_tracker::FormatTag;
use format_tracker::FormatTracker;
use io::{
    pty::{TerminalSize, TerminalWriteCommand},
    TerminalRead,
};
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

pub struct TerminalEmulator<Io: FreminalTermInputOutput + Send> {
    _io: Arc<Mutex<Io>>,
    tx_sender: Sender<TerminalWriteCommand>,
    internal_state: Arc<Mutex<TermininalEmulatorInternalState>>,
}

fn absolute_path(path: impl AsRef<Path>) -> std::io::Result<PathBuf> {
    let path = path.as_ref();

    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()?.join(path)
    }
    .clean();

    info!("Recording path: {}", absolute_path.display());

    Ok(absolute_path)
}

impl TerminalEmulator<FreminalPtyInputOutput> {
    pub fn new(args: &Args) -> Result<Self> {
        let mut recording = None;

        let (tx_reader, mut rx_reader) = mpsc::channel::<TerminalRead>(100);
        let (tx_writer, mut rx_writer) = mpsc::channel::<TerminalWriteCommand>(100);
        let io = Arc::new(Mutex::new(FreminalPtyInputOutput::new(args)?));

        // if recording path is some, open a file for writing
        if let Some(path) = &args.recording {
            // convert the path to a fully qualified path
            let cleaned_path = absolute_path(path)?;

            recording = match std::fs::File::create(cleaned_path) {
                Ok(file) => Some(file),
                Err(e) => {
                    error!("Failed to create recording file: {}", backtraced_err(&e));
                    None
                }
            }
        }

        // spawn a thread to read from the

        let default_size = TerminalSize::default();
        let internal_state = Arc::new(Mutex::new(TermininalEmulatorInternalState {
            parser: FreminalAnsiParser::new(),
            terminal_buffer: TerminalBufferHolder::new(
                default_size.get_rows(),
                default_size.get_cols(),
            ),
            format_tracker: FormatTracker::new(),
            cursor_state: CursorState::new(),
            modes: TerminalModes::default(),
            window_title: None,
            saved_color_state: None,
        }));

        let io_clone = Arc::clone(&io);
        tokio::spawn(async move {
            let value = io_clone.lock().await.set_win_size(default_size);
            if let Err(e) = value {
                error!("Failed to set initial window size: {}", backtraced_err(&*e));
            }

            io_clone.lock().await.pty_handler(rx_writer, tx_reader);
        });

        let internal_internal_state = internal_state.clone();
        let internal_tx = tx_writer.clone();

        tokio::spawn(async move {
            read_channel(
                &mut rx_reader,
                &internal_internal_state,
                &internal_tx,
                &mut recording,
            ).await;

            info!("Finished reading from channel");
        });

        let ret = Self {
            _io: io,
            tx_sender: tx_writer,
            internal_state,
        };
        Ok(ret)
    }
}

impl<Io: FreminalTermInputOutput + Send> TerminalEmulator<Io> {
    pub async fn get_win_size(&self) -> (usize, usize) {
        self.internal_state
            .lock()
            .await
            .terminal_buffer
            .get_win_size()
    }

    pub async fn get_window_title(&self) -> Option<String> {
        self.internal_state.lock().await.window_title.clone()
    }

    #[allow(dead_code)]
    pub async fn clear_window_title(&self) {
        self.internal_state.lock().await.window_title = None;
    }

    pub async fn set_win_size(
        &self,
        width_chars: usize,
        height_chars: usize,
        font_width: usize,
        font_height: usize,
    ) -> Result<()> {
        let sender = self.tx_sender.clone();
        //let mut state = self.internal_state.lock().await;
        let cursor_state = self.internal_state.lock().await.cursor_state.clone();

        let response =
        self.internal_state.lock().await
                .terminal_buffer
                .set_win_size(width_chars, height_chars, &cursor_state.pos);
            self.internal_state.lock().await.cursor_state.pos = response.new_cursor_pos;

        if response.changed {
            debug!("Resizing terminal to: {width_chars}x{height_chars}");
            match sender
                .send(TerminalWriteCommand::Resize(TerminalSize {
                    cols: width_chars,
                    rows: height_chars,
                    pixel_width: font_width,
                    pixel_height: font_height,
                }))
                .await
            {
                Ok(()) => Ok(()),
                Err(e) => {
                    error!("Failed to send resize command: {}", backtraced_err(&e));
                    Err(e.into())
                }
            }
        } else {
            Ok(())
        }
    }

    pub async fn write(&self, to_write: &TerminalInput) -> Result<()> {
        let internal_state = self.internal_state.lock().await;

        match write(&internal_state, &self.tx_sender, to_write).await {
            Ok(()) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub async fn data(&self) -> TerminalData<Vec<u8>> {
        let internal_state = self.internal_state.lock().await;

        TerminalData {
            scrollback: internal_state.terminal_buffer.data().scrollback.to_vec(),
            visible: internal_state.terminal_buffer.data().visible.to_vec(),
        }
    }

    pub async fn format_data(&self) -> TerminalData<Vec<FormatTag>> {
        let internal_state = self.internal_state.lock().await;

        let offset = internal_state.terminal_buffer.data().scrollback.len();
        split_format_data_for_scrollback(internal_state.format_tracker.tags(), offset)
    }

    pub async fn cursor_pos(&self) -> CursorPos {
        let internal_state = self.internal_state.lock().await;
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

async fn osc_response(
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
                        let _ = io.send(message).await;
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
                        let _ = io.send(message).await;
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

async fn report_cursor_position(
    internal_state: &MutexGuard<'_, TermininalEmulatorInternalState>,
    io: &Sender<TerminalWriteCommand>,
) {
    let x = internal_state.cursor_state.pos.x + 1;
    let y = internal_state.cursor_state.pos.y + 1;
    let formatted_string = format!("\x1b[{y};{x}R");
    let output = formatted_string.as_bytes();

    for byte in output {
        let _ = io.send(TerminalWriteCommand::Write(vec![*byte]))
            .await;
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

async fn write(
    internal_state: &MutexGuard<'_, TermininalEmulatorInternalState>,
    io: &Sender<TerminalWriteCommand>,
    to_write: &TerminalInput,
) -> Result<()> {
    info!("Writing: {:?}", to_write);
    match to_write.to_payload(internal_state.modes.cursor_key == Decckm::Application) {
        TerminalInputPayload::Single(c) => {
            info!("Writing single: {:?}", c);
            let message = TerminalWriteCommand::Write(vec![c]);
            match io.send(message).await {
                Ok(()) => Ok(()),
                Err(e) => Err(e.into()),
            }
        }
        TerminalInputPayload::Many(to_write) => {
            info!("Writing many: {:?}", to_write);
            while !to_write.is_empty() {
                let message = TerminalWriteCommand::Write(to_write.to_vec());
                match io.send(message).await {
                    Ok(()) => (),
                    Err(e) => return Err(e.into()),
                }
            }

            Ok(())
        }
    }
}

async fn read_channel(
    rx: &mut Receiver<TerminalRead>,
    guard: &Arc<Mutex<TermininalEmulatorInternalState>>,
    write_channel: &Sender<TerminalWriteCommand>,
    recording: &mut Option<File>,
) {
    info!("Reading from channel");
    while let Some(read) = rx.recv().await
    {
        info!("Handling incoming data");
        let incoming = read.get_buffer();

        if let Some(file) = recording {
            let mut output = String::new();
            // loop over the buffer and convert to a string representation of the number, separated by commas
            for byte in incoming {
                output.push_str(&format!("{byte},"));
            }
            let _ = file.write_all(output.as_bytes());
        }
        //debug!("Incoming data: {:?}", std::str::from_utf8(incoming));

        let mut internal_state = guard.lock().await;
        handle_incoming_data(&mut internal_state, write_channel, incoming).await;
        info!("done handling incoming data");
    }

    error!("Failed to read from channel");
}

async fn handle_incoming_data(
    internal_state: &mut MutexGuard<'_, TermininalEmulatorInternalState>,
    write_channel: &Sender<TerminalWriteCommand>,
    incoming: &[u8],
) {
    debug!("Incoming data: {:?}", incoming);
    let parsed = internal_state.parser.push(incoming);
    info!("Parsed data: {:?}", parsed);
    for segment in parsed {
        // if segment is not data, we want to print out the segment
        if let TerminalOutput::Data(data) = &segment {
            debug!("Incoming data: {:?}", str::from_utf8(data).unwrap());
        }
        // else {
        //     debug!("Incoming segment: {:?}", segment);
        // }

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
                osc_response(internal_state, &write_channel.clone(), osc).await;
            }
            TerminalOutput::CursorReport => {
                report_cursor_position(internal_state, &write_channel.clone()).await;
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
