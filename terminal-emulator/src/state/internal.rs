// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use anyhow::Result;
use core::str;
use eframe::egui::{Color32, Context};
use freminal_common::colors::TerminalColor;

use crate::{
    ansi::{FreminalAnsiParser, TerminalOutput},
    ansi_components::{
        mode::{BracketedPaste, Decawm, Decckm, Mode, TerminalModes},
        osc::{AnsiOscInternalType, AnsiOscType},
        sgr::SelectGraphicRendition,
    },
    format_tracker::{FormatTag, FormatTracker},
    interface::{split_format_data_for_scrollback, TerminalInput, TerminalInputPayload},
    io::PtyWrite,
};

use super::{
    buffer::{TerminalBufferHolder, TerminalBufferSetWinSizeResponse},
    cursor::{CursorPos, CursorState},
    data::TerminalSections,
    fonts::{FontDecorations, FontWeight},
    term_char::TChar,
};

pub const TERMINAL_WIDTH: usize = 50;
pub const TERMINAL_HEIGHT: usize = 16;

pub struct TerminalState {
    parser: FreminalAnsiParser,
    terminal_buffer: TerminalBufferHolder,
    format_tracker: FormatTracker,
    cursor_state: CursorState,
    modes: TerminalModes,
    saved_color_state: Option<(TerminalColor, TerminalColor, TerminalColor)>,
    window_title: Option<String>,
    write_tx: crossbeam_channel::Sender<PtyWrite>,
    changed: bool,
    ctx: Option<Context>,
    leftover_data: Option<Vec<u8>>,
}

impl TerminalState {
    #[must_use]
    pub fn new(write_tx: crossbeam_channel::Sender<PtyWrite>) -> Self {
        Self {
            parser: FreminalAnsiParser::new(),
            terminal_buffer: TerminalBufferHolder::new(TERMINAL_WIDTH, TERMINAL_HEIGHT),
            format_tracker: FormatTracker::new(),
            modes: TerminalModes {
                cursor_key: Decckm::default(),
                bracketed_paste: BracketedPaste::default(),
            },
            cursor_state: CursorState::default(),
            saved_color_state: None,
            window_title: None,
            write_tx,
            changed: false,
            ctx: None,
            leftover_data: None,
        }
    }

    pub(crate) const fn is_changed(&self) -> bool {
        self.changed
    }

    fn set_state_changed(&mut self) {
        self.changed = true;
    }

    pub(crate) fn clear_changed(&mut self) {
        self.changed = false;
    }

    pub fn set_ctx(&mut self, ctx: Context) {
        if self.ctx.is_some() {
            return;
        }

        self.ctx = Some(ctx);
    }

    fn request_redraw(&mut self) {
        self.changed = true;
        if let Some(ctx) = &self.ctx {
            debug!("Internal State: Requesting repaint");
            ctx.request_repaint();
        }
    }

    pub(crate) const fn get_win_size(&self) -> (usize, usize) {
        self.terminal_buffer.get_win_size()
    }

    pub(crate) fn get_window_title(&self) -> Option<String> {
        self.window_title.clone()
    }

    pub(crate) fn data(&self) -> TerminalSections<Vec<TChar>> {
        self.terminal_buffer.data()
    }

    pub(crate) fn format_data(&self) -> TerminalSections<Vec<FormatTag>> {
        let offset = self.terminal_buffer.data().scrollback.len();
        split_format_data_for_scrollback(self.format_tracker.tags(), offset)
    }

    pub(crate) fn cursor_pos(&self) -> CursorPos {
        self.cursor_state.pos.clone()
    }

    pub(crate) fn clear_window_title(&mut self) {
        self.window_title = None;
    }

    pub(crate) fn set_win_size(
        &mut self,
        width: usize,
        height: usize,
    ) -> TerminalBufferSetWinSizeResponse {
        let response = self
            .terminal_buffer
            .set_win_size(width, height, &self.cursor_state.pos);
        self.cursor_state.pos = response.new_cursor_pos.clone();

        response
    }

    pub(crate) fn get_cursor_key_mode(&self) -> Decckm {
        self.modes.cursor_key.clone()
    }

    pub(crate) fn handle_data(&mut self, data: &[u8]) {
        // if we have leftover data, prepend it to the incoming data
        let data = self.leftover_data.take().map_or_else(
            || data.to_vec(),
            |leftover_data| {
                info!("We have leftover data: {:?}", leftover_data);
                let mut new_data = Vec::with_capacity(leftover_data.len() + data.len());
                new_data.extend_from_slice(&leftover_data);
                new_data.extend_from_slice(data);
                self.leftover_data = None;
                new_data
            },
        );
        let response = match self
            .terminal_buffer
            .insert_data(&self.cursor_state.pos, &data)
        {
            Ok(response) => response,
            Err(e) => {
                error!("Failed to insert data: {e}");
                return;
            }
        };

        if !response.left_over.is_empty() {
            warn!("Leftover data from incoming buffer");
            self.leftover_data = Some(response.left_over);
        }

        self.format_tracker
            .push_range_adjustment(response.insertion_range);
        self.format_tracker
            .push_range(&self.cursor_state, response.written_range);
        self.cursor_state.pos = response.new_cursor_pos;
        self.set_state_changed();
        self.request_redraw();
    }

    pub(crate) fn set_cursor_pos(&mut self, x: Option<usize>, y: Option<usize>) {
        if let Some(x) = x {
            self.cursor_state.pos.x = x - 1;
        }
        if let Some(y) = y {
            self.cursor_state.pos.y = y - 1;
        }
    }

    pub(crate) fn set_cursor_pos_rel(&mut self, x: Option<i32>, y: Option<i32>) {
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

    pub(crate) fn clear_forwards(&mut self) {
        match self.terminal_buffer.clear_forwards(&self.cursor_state.pos) {
            Ok(Some(buf_pos)) => {
                self.format_tracker
                    .push_range(&self.cursor_state, buf_pos..usize::MAX);
            }
            // FIXME: why on god's green earth are we having an option type here?
            Ok(None) => (),
            Err(e) => {
                error!("Failed to clear forwards: {e}");
            }
        }
    }

    pub(crate) fn clear_backwards(&mut self) {
        match self.terminal_buffer.clear_backwards(&self.cursor_state.pos) {
            Ok(Some(buf_pos)) => {
                self.format_tracker.push_range(&self.cursor_state, buf_pos);
            }
            Ok(None) => (),
            Err(e) => {
                error!("Failed to clear backwards: {e}");
            }
        }
    }

    pub(crate) fn clear_all(&mut self) {
        self.format_tracker
            .push_range(&self.cursor_state, 0..usize::MAX);
        self.terminal_buffer.clear_all();
    }

    pub(crate) fn clear_visible(&mut self) {
        let range = self.terminal_buffer.clear_visible();

        if range.end > 0 {
            self.format_tracker.push_range(&self.cursor_state, range);
        }
    }

    pub(crate) fn clear_line_forwards(&mut self) {
        if let Some(range) = self
            .terminal_buffer
            .clear_line_forwards(&self.cursor_state.pos)
        {
            match self.format_tracker.delete_range(range) {
                Ok(()) => (),
                Err(e) => {
                    error!("Failed to delete range: {e}");
                }
            }
        }
    }

    pub(crate) fn clear_line_backwards(&mut self) {
        if let Some(range) = self
            .terminal_buffer
            .clear_line_backwards(&self.cursor_state.pos)
        {
            match self.format_tracker.delete_range(range) {
                Ok(()) => (),
                Err(e) => {
                    error!("Failed to delete range: {e}");
                }
            }
        }
    }

    pub(crate) fn clear_line(&mut self) {
        if let Some(range) = self.terminal_buffer.clear_line(&self.cursor_state.pos) {
            match self.format_tracker.delete_range(range) {
                Ok(()) => (),
                Err(e) => {
                    error!("Failed to delete range: {e}");
                }
            }
        }
    }

    pub(crate) fn carriage_return(&mut self) {
        self.cursor_state.pos.x = 0;
    }

    pub(crate) fn new_line(&mut self) {
        self.cursor_state.pos.y += 1;
    }

    pub(crate) fn backspace(&mut self) {
        if self.cursor_state.pos.x >= 1 {
            self.cursor_state.pos.x -= 1;
        } else {
            // FIXME: this is not correct, we should move to the end of the previous line
            warn!("FIXME: Backspace at the beginning of the line. Not wrapping");
        }
    }

    pub(crate) fn insert_lines(&mut self, num_lines: usize) {
        let response = self
            .terminal_buffer
            .insert_lines(&self.cursor_state.pos, num_lines);
        match self.format_tracker.delete_range(response.deleted_range) {
            Ok(()) => (),
            Err(e) => {
                error!("Failed to delete range: {e}");
                return;
            }
        };
        self.format_tracker
            .push_range_adjustment(response.inserted_range);
    }

    pub(crate) fn delete(&mut self, num_chars: usize) {
        let deleted_buf_range = self
            .terminal_buffer
            .delete_forwards(&self.cursor_state.pos, num_chars);
        if let Some(range) = deleted_buf_range {
            match self.format_tracker.delete_range(range) {
                Ok(()) => (),
                Err(e) => {
                    error!("Failed to delete range: {e}");
                }
            }
        }
    }

    pub(crate) fn reset(&mut self) {
        self.cursor_state.color = TerminalColor::Default;
        self.cursor_state.background_color = TerminalColor::DefaultBackground;
        self.cursor_state.underline_color = TerminalColor::DefaultUnderlineColor;
        self.cursor_state.font_weight = FontWeight::Normal;
        self.cursor_state.font_decorations.clear();
        self.saved_color_state = None;
    }

    pub(crate) fn font_decordations_add_if_not_contains(&mut self, decoration: FontDecorations) {
        if !self.cursor_state.font_decorations.contains(&decoration) {
            self.cursor_state.font_decorations.push(decoration);
        }
    }

    pub(crate) fn font_decorations_remove_if_contains(&mut self, decoration: &FontDecorations) {
        self.cursor_state
            .font_decorations
            .retain(|d| *d != *decoration);
    }

    pub(crate) fn set_foreground(&mut self, color: TerminalColor) {
        self.cursor_state.color = color;
    }

    pub(crate) fn set_background(&mut self, color: TerminalColor) {
        self.cursor_state.background_color = color;
    }

    pub(crate) fn set_underline_color(&mut self, color: TerminalColor) {
        self.cursor_state.underline_color = color;
    }

    pub(crate) fn sgr(&mut self, sgr: SelectGraphicRendition) {
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
            | SelectGraphicRendition::ProportionalSpacing
            | SelectGraphicRendition::DisableProportionalSpacing
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

    pub(crate) fn set_mode(&mut self, mode: &Mode) {
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

    pub(crate) fn insert_spaces(&mut self, num_spaces: usize) {
        let response = self
            .terminal_buffer
            .insert_spaces(&self.cursor_state.pos, num_spaces);
        self.format_tracker
            .push_range_adjustment(response.insertion_range);
    }

    pub(crate) fn reset_mode(&mut self, mode: &Mode) {
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

    pub(crate) fn osc_response(&mut self, osc: AnsiOscType) {
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

    pub(crate) fn report_cursor_position(&self) {
        let x = self.cursor_state.pos.x + 1;
        let y = self.cursor_state.pos.y + 1;
        let formatted_string = format!("\x1b[{y};{x}R");
        let output = formatted_string.as_bytes();

        for byte in output {
            self.write(&TerminalInput::Ascii(*byte))
                .expect("Failed to write cursor position report");
        }
    }

    pub fn handle_incoming_data(&mut self, incoming: &[u8]) {
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
                TerminalOutput::ClearDisplayfromCursortoEndofDisplay => self.clear_forwards(),
                TerminalOutput::ClearDiplayfromStartofDisplaytoCursor => self.clear_backwards(),
                TerminalOutput::ClearScrollbackandDisplay => self.clear_all(),
                TerminalOutput::ClearDisplay => self.clear_visible(),
                TerminalOutput::ClearLineForwards => self.clear_line_forwards(),
                TerminalOutput::ClearLineBackwards => self.clear_line_backwards(),
                TerminalOutput::ClearLine => self.clear_line(),
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

    /// Write data to the terminal
    ///
    /// # Errors
    /// Will return an error if the write fails
    pub fn write(&self, to_write: &TerminalInput) -> Result<()> {
        match to_write.to_payload(self.get_cursor_key_mode() == Decckm::Application) {
            TerminalInputPayload::Single(c) => {
                self.write_tx.send(PtyWrite::Write(vec![c]))?;
            }
            TerminalInputPayload::Many(to_write) => {
                self.write_tx.send(PtyWrite::Write(to_write.to_vec()))?;
            }
        };

        Ok(())
    }
}
