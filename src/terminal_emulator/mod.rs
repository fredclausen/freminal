// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::{fmt, num::TryFromIntError, path::PathBuf};

use ansi::{FreminalAnsiParser, SelectGraphicRendition, TerminalOutput};
use buffer::TerminalBufferHolder;
use format_tracker::FormatTracker;
use recording::{NotIntOfType, Recorder};

pub use format_tracker::FormatTag;
pub use io::{FreminalPtyInputOutput, FreminalTermInputOutput};
pub use recording::{FreminalRecordingHandle, LoadRecordingError, Recording, SnapshotItem};
pub use replay::{ControlAction, FreminalReplayControl, FreminalReplayIo};

use crate::{error::backtraced_err, terminal_emulator::io::ReadResponse};
use thiserror::Error;

use self::{
    io::CreatePtyIoError,
    recording::{FreminalRecordingItem, StartRecordingResponse},
};

mod ansi;
mod buffer;
mod format_tracker;
mod io;
mod recording;
mod replay;

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

#[derive(Debug, Error)]
enum SnapshotCursorPosErrorPriv {
    #[error("x pos cannot be cast to i64")]
    XNotI64(#[source] TryFromIntError),
    #[error("y pos cannot be cast to i64")]
    YNotI64(#[source] TryFromIntError),
}

#[derive(Debug, Error)]
#[error(transparent)]
pub struct SnapshotCursorPosError(#[from] SnapshotCursorPosErrorPriv);

#[derive(Debug, Error)]
enum LoadCursorPosError {
    #[error("root element is not a map")]
    RootNotMap,
    #[error("x element not present")]
    MissingX,
    #[error("x cannot be case to usize")]
    XNotUsize(#[source] NotIntOfType),
    #[error("y element not present")]
    MissingY,
    #[error("y cannot be case to usize")]
    YNotUsize(#[source] NotIntOfType),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CursorPos {
    pub x: usize,
    pub y: usize,
}

impl CursorPos {
    fn from_snapshot(snapshot: SnapshotItem) -> Result<Self, LoadCursorPosError> {
        use LoadCursorPosError::{MissingX, MissingY, RootNotMap, XNotUsize, YNotUsize};

        let mut map = snapshot.into_map().map_err(|_| RootNotMap)?;

        let x = map.remove("x").ok_or(MissingX)?;
        let x = x.into_num::<usize>().map_err(XNotUsize)?;

        let y = map.remove("y").ok_or(MissingY)?;
        let y = y.into_num::<usize>().map_err(YNotUsize)?;

        Ok(Self { x, y })
    }

    fn snapshot(&self) -> Result<SnapshotItem, SnapshotCursorPosErrorPriv> {
        use SnapshotCursorPosErrorPriv::{XNotI64, YNotI64};
        let x_i64: i64 = self.x.try_into().map_err(XNotI64)?;
        let y_i64: i64 = self.y.try_into().map_err(YNotI64)?;
        let res = SnapshotItem::Map(
            [
                ("x".to_string(), x_i64.into()),
                ("y".to_string(), y_i64.into()),
            ]
            .into(),
        );
        Ok(res)
    }
}

mod cursor_state_keys {
    pub const POS: &str = "pos";
    pub const BOLD: &str = "bold";
    pub const ITALIC: &str = "italic";
    pub const COLOR: &str = "color";
}

#[derive(Debug, Error)]
enum LoadCursorStateErrorPriv {
    #[error("root element is not a map")]
    RootNotMap,
    #[error("bold field is not present")]
    BoldNotPresent,
    #[error("bold field is not a bool")]
    BoldNotBool,
    #[error("italic field is not present")]
    ItalicNotPresent,
    #[error("italic field is not a bool")]
    ItalicNotBool,
    #[error("color field is not present")]
    ColorNotPresent,
    #[error("color field is not a bool")]
    ColorNotString,
    #[error("color failed to parse")]
    ColorInvalid(()),
    #[error("pos field not present")]
    PosNotPresent,
    #[error("failed to parse position")]
    FailParsePos(#[source] LoadCursorPosError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub struct LoadCursorStateError(#[from] LoadCursorStateErrorPriv);

#[derive(Eq, PartialEq, Debug, Clone)]
struct CursorState {
    pos: CursorPos,
    bold: bool,
    italic: bool,
    color: TerminalColor,
}

impl CursorState {
    fn from_snapshot(snapshot: SnapshotItem) -> Result<Self, LoadCursorStateError> {
        use LoadCursorStateErrorPriv::{
            BoldNotBool, BoldNotPresent, ColorInvalid, ColorNotPresent, ColorNotString,
            FailParsePos, PosNotPresent, RootNotMap,
        };
        let mut map = snapshot.into_map().map_err(|_| RootNotMap)?;

        let bold = map.remove(cursor_state_keys::BOLD).ok_or(BoldNotPresent)?;
        let SnapshotItem::Bool(bold) = bold else {
            Err(BoldNotBool)?
        };

        let italic = map.remove(cursor_state_keys::ITALIC).ok_or(BoldNotPresent)?;
        let SnapshotItem::Bool(italic) = italic else {
            Err(BoldNotBool)?
        };

        let color = map
            .remove(cursor_state_keys::COLOR)
            .ok_or(ColorNotPresent)?;
        let SnapshotItem::String(color) = color else {
            Err(ColorNotString)?
        };
        let color = color.parse().map_err(ColorInvalid)?;

        let pos = map.remove(cursor_state_keys::POS).ok_or(PosNotPresent)?;
        let pos = CursorPos::from_snapshot(pos).map_err(FailParsePos)?;

        Ok(Self { pos, bold, italic, color })
    }

    fn snapshot(&self) -> Result<SnapshotItem, SnapshotCursorPosError> {
        let res = SnapshotItem::Map(
            [
                (cursor_state_keys::POS.to_string(), self.pos.snapshot()?),
                (cursor_state_keys::BOLD.to_string(), self.bold.into()),
                (
                    cursor_state_keys::COLOR.to_string(),
                    self.color.to_string().into(),
                ),
            ]
            .into(),
        );
        Ok(res)
    }
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
            _ => return Err(()),
        };
        Ok(ret)
    }
}

impl TerminalColor {
    const fn from_sgr(sgr: SelectGraphicRendition) -> Option<Self> {
        let ret = match sgr {
            SelectGraphicRendition::ForegroundBlack => Self::Black,
            SelectGraphicRendition::ForegroundRed => Self::Red,
            SelectGraphicRendition::ForegroundGreen => Self::Green,
            SelectGraphicRendition::ForegroundYellow => Self::Yellow,
            SelectGraphicRendition::ForegroundBlue => Self::Blue,
            SelectGraphicRendition::ForegroundMagenta => Self::Magenta,
            SelectGraphicRendition::ForegroundCyan => Self::Cyan,
            SelectGraphicRendition::ForegroundWhite => Self::White,
            _ => return None,
        };

        Some(ret)
    }
}

pub struct TerminalData<T> {
    pub scrollback: T,
    pub visible: T,
}

#[derive(Debug, Error)]
enum StartRecordingErrorPriv {
    #[error("failed to start recording")]
    Start(#[from] std::io::Error),
    #[error("failed to snapshot terminal buffer")]
    SnapshotBuffer(#[from] buffer::CreateSnapshotError),
    #[error("failed to snapshot format tracker")]
    SnapshotFormatTracker(#[from] format_tracker::SnapshotFormatTagError),
    #[error("failed to snapshot cursor")]
    SnapshotCursor(#[from] SnapshotCursorPosError),
}

#[derive(Debug, Error)]
#[error(transparent)]
pub struct StartRecordingError(#[from] StartRecordingErrorPriv);

#[derive(Debug, Error)]
enum LoadSnapshotErrorPriv {
    #[error("root element is not a map")]
    RootNotMap,
    #[error("parser field is not present")]
    ParserNotPresent,
    #[error("failed to load parser")]
    LoadParser(#[from] ansi::LoadSnapshotError),
    #[error("terminal_buffer field not present")]
    BufferNotPresent,
    #[error("failed to load buffer")]
    LoadBuffer(#[from] buffer::LoadSnapshotError),
    #[error("format tracker not present")]
    FormatTrackerNotPresent,
    #[error("failed to load format tracker")]
    LoadFormatTracker(#[from] format_tracker::LoadFormatTrackerSnapshotError),
    #[error("decckm field not present")]
    DecckmNotPresent,
    #[error("decckm field not bool")]
    DecckmNotBool,
    #[error("cursor_state not present")]
    CursorStateNotPresent,
    #[error("failed to load cursor state")]
    LoadCursorState(#[from] LoadCursorStateError),
}

#[derive(Debug, Error)]
#[error(transparent)]
pub struct LoadSnapshotError(#[from] LoadSnapshotErrorPriv);

pub struct TerminalEmulator<Io: FreminalTermInputOutput> {
    parser: FreminalAnsiParser,
    terminal_buffer: TerminalBufferHolder,
    format_tracker: FormatTracker,
    cursor_state: CursorState,
    decckm_mode: bool,
    recorder: Recorder,
    io: Io,
}

pub const TERMINAL_WIDTH: usize = 50;
pub const TERMINAL_HEIGHT: usize = 16;

impl TerminalEmulator<FreminalPtyInputOutput> {
    pub fn new(recording_path: PathBuf) -> Result<Self, CreatePtyIoError> {
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
                bold: false,
                italic: false,
                color: TerminalColor::Default,
            },
            recorder: Recorder::new(recording_path),
            io,
        };
        Ok(ret)
    }
}

impl TerminalEmulator<FreminalReplayIo> {
    pub fn from_snapshot(
        snapshot: SnapshotItem,
        io_handle: FreminalReplayIo,
    ) -> Result<Self, LoadSnapshotError> {
        use LoadSnapshotErrorPriv::{
            BufferNotPresent, CursorStateNotPresent, DecckmNotBool, DecckmNotPresent,
            FormatTrackerNotPresent, LoadBuffer, LoadCursorState, LoadFormatTracker, LoadParser,
            ParserNotPresent, RootNotMap,
        };

        let mut root = snapshot.into_map().map_err(|_| RootNotMap)?;
        let parser =
            FreminalAnsiParser::from_snapshot(root.remove("parser").ok_or(ParserNotPresent)?)
                .map_err(LoadParser)?;
        let terminal_buffer = TerminalBufferHolder::from_snapshot(
            root.remove("terminal_buffer").ok_or(BufferNotPresent)?,
        )
        .map_err(LoadBuffer)?;
        let format_tracker = FormatTracker::from_snapshot(
            root.remove("format_tracker")
                .ok_or(FormatTrackerNotPresent)?,
        )
        .map_err(LoadFormatTracker)?;
        let SnapshotItem::Bool(decckm_mode) = root.remove("decckm_mode").ok_or(DecckmNotPresent)?
        else {
            Err(DecckmNotBool)?
        };
        let cursor_state =
            CursorState::from_snapshot(root.remove("cursor_state").ok_or(CursorStateNotPresent)?)
                .map_err(LoadCursorState)?;

        Ok(Self {
            parser,
            terminal_buffer,
            format_tracker,
            decckm_mode,
            cursor_state,
            recorder: Recorder::new("recordings".into()),
            io: io_handle,
        })
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
            self.recorder.set_win_size(width_chars, height_chars);
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
        if let Some(color) = TerminalColor::from_sgr(sgr) {
            self.cursor_state.color = color;
            return
        }

        match sgr {
            SelectGraphicRendition::Reset => {
                self.cursor_state.color = TerminalColor::Default;
                self.cursor_state.bold = false;
                self.cursor_state.italic = false;
            }
            SelectGraphicRendition::Bold => {
                self.cursor_state.bold = true;
            }
            SelectGraphicRendition::Italic => {
                self.cursor_state.italic = true;
            }
            SelectGraphicRendition::DefaultForeground => {
                self.cursor_state.color = TerminalColor::Default;
            }
            SelectGraphicRendition::FastBlink | SelectGraphicRendition::SlowBlink => {
                // Blinking is not supported
                warn!("Blinking is not supported");
                return
            }
            _ => {
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
            Mode::Unknown(_) => {
                warn!("unhandled set mode: {mode:?}");
            }
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
                TerminalOutput::Invalid => {}
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
            self.recorder.write(incoming);
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

    pub fn start_recording(&mut self) -> Result<FreminalRecordingHandle, StartRecordingError> {
        use StartRecordingErrorPriv::{
            SnapshotBuffer, SnapshotCursor, SnapshotFormatTracker, Start,
        };

        let recording_handle = self.recorder.start_recording().map_err(Start)?;
        match recording_handle {
            StartRecordingResponse::New(initializer) => {
                initializer.snapshot_item("parser".to_string(), self.parser.snapshot());
                initializer.snapshot_item(
                    "terminal_buffer".to_string(),
                    self.terminal_buffer.snapshot().map_err(SnapshotBuffer)?,
                );
                initializer.snapshot_item(
                    "format_tracker".to_string(),
                    self.format_tracker
                        .snapshot()
                        .map_err(SnapshotFormatTracker)?,
                );
                initializer.snapshot_item("decckm_mode".to_string(), self.decckm_mode.into());
                initializer.snapshot_item(
                    "cursor_state".to_string(),
                    self.cursor_state.snapshot().map_err(SnapshotCursor)?,
                );
                Ok(initializer.into_handle())
            }
            StartRecordingResponse::Existing(handle) => Ok(handle),
        }
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
                bold: true,
                italic: false,
            },
            FormatTag {
                start: 5,
                end: 7,
                color: TerminalColor::Red,
                bold: false,
                italic: false,
            },
            FormatTag {
                start: 7,
                end: 10,
                color: TerminalColor::Blue,
                bold: true,
                italic: false,
            },
            FormatTag {
                start: 10,
                end: usize::MAX,
                color: TerminalColor::Red,
                bold: true,
                italic: false,
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
                bold: true,
                italic: false,
            },]
        );

        // Case 3: Split a segment
        let res = split_format_data_for_scrollback(tags.clone(), 9);
        assert_eq!(
            res.scrollback,
            &[
                FormatTag {
                    start: 0,
                    end: 5,
                    color: TerminalColor::Blue,
                    bold: true,
                    italic: false,
                },
                FormatTag {
                    start: 5,
                    end: 7,
                    color: TerminalColor::Red,
                    bold: false,
                    italic: false,
                },
                FormatTag {
                    start: 7,
                    end: 9,
                    color: TerminalColor::Blue,
                    bold: true,
                    italic: false,
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
                    bold: true,
                    italic: false,
                },
                FormatTag {
                    start: 1,
                    end: usize::MAX,
                    color: TerminalColor::Red,
                    bold: true,
                    italic: false,
                },
            ]
        );
    }

    #[test]
    fn test_cursor_state_snapshot() {
        let state = CursorState {
            pos: CursorPos { x: 10, y: 50 },
            bold: false,
            italic: false,
            color: TerminalColor::Magenta,
        };

        let snapshot = state.snapshot().expect("failed to create snapshot");
        let loaded = CursorState::from_snapshot(snapshot).expect("failed to load snapshot");
        assert_eq!(loaded, state);
    }
}
