// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use anyhow::Result;
use conv::ConvUtil;
use core::str;
use eframe::egui::{self, Color32, Context};
use freminal_common::{
    colors::TerminalColor, scroll::ScrollDirection, window_manipulation::WindowManipulation,
};
#[cfg(debug_assertions)]
use std::time::Instant;

use crate::{
    ansi::{FreminalAnsiParser, TerminalOutput},
    ansi_components::{
        line_draw::DecSpecialGraphics,
        mode::{Mode, MouseTrack, TerminalModes},
        modes::{decckm::Decckm, dectcem::Dectcem, xtextscrn::XtExtscrn, xtmsewin::XtMseWin},
        osc::{AnsiOscInternalType, AnsiOscType, UrlResponse},
        sgr::SelectGraphicRendition,
    },
    format_tracker::{FormatTag, FormatTracker},
    interface::{
        collect_text, split_format_data_for_scrollback, TerminalInput, TerminalInputPayload,
    },
    io::PtyWrite,
};

use super::{
    buffer::{TerminalBufferHolder, TerminalBufferSetWinSizeResponse},
    cursor::{CursorPos, CursorState, ReverseVideo},
    data::TerminalSections,
    fonts::{FontDecorations, FontWeight},
    term_char::TChar,
};

pub const TERMINAL_WIDTH: usize = 50;
pub const TERMINAL_HEIGHT: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CurrentBuffer {
    #[default]
    Primary,
    Alternate,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Buffer {
    pub terminal_buffer: TerminalBufferHolder,
    pub format_tracker: FormatTracker,
    pub cursor_state: CursorState,
    pub show_cursor: Dectcem,
    pub saved_cursor_position: Option<CursorPos>,
}

impl Buffer {
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            terminal_buffer: TerminalBufferHolder::new(width, height),
            format_tracker: FormatTracker::new(),
            cursor_state: CursorState::default(),
            show_cursor: Dectcem::default(),
            saved_cursor_position: None,
        }
    }

    #[must_use]
    pub fn show_cursor(&self) -> bool {
        self.terminal_buffer.show_cursor(&self.cursor_state.pos)
    }
}

#[derive(Debug)]
pub struct TerminalState {
    pub parser: FreminalAnsiParser,
    pub current_buffer: CurrentBuffer,
    pub primary_buffer: Buffer,
    pub alternate_buffer: Buffer,
    pub modes: TerminalModes,
    pub window_title: Option<String>,
    pub write_tx: crossbeam_channel::Sender<PtyWrite>,
    pub changed: bool,
    pub ctx: Option<Context>,
    pub leftover_data: Option<Vec<u8>>,
    pub character_replace: DecSpecialGraphics,
    pub mouse_position: Option<egui::Pos2>,
    pub window_focused: bool,
    pub window_commands: Vec<WindowManipulation>,
}

impl Default for TerminalState {
    /// This method should never really be used. It was added to allow the test suite to pass
    /// The problem here is that you most likely really really want a rx channel to go with the tx channel
    fn default() -> Self {
        Self::new(crossbeam_channel::unbounded().0)
    }
}

impl PartialEq for TerminalState {
    fn eq(&self, other: &Self) -> bool {
        self.parser == other.parser
            && self.primary_buffer == other.primary_buffer
            && self.alternate_buffer == other.alternate_buffer
            && self.modes == other.modes
            && self.window_title == other.window_title
            && self.changed == other.changed
            && self.ctx == other.ctx
            && self.leftover_data == other.leftover_data
            && self.character_replace == other.character_replace
    }
}

impl TerminalState {
    #[must_use]
    pub fn new(write_tx: crossbeam_channel::Sender<PtyWrite>) -> Self {
        Self {
            parser: FreminalAnsiParser::new(),
            current_buffer: CurrentBuffer::Primary,
            primary_buffer: Buffer::new(TERMINAL_WIDTH, TERMINAL_HEIGHT),
            alternate_buffer: Buffer::new(TERMINAL_WIDTH, TERMINAL_HEIGHT),
            modes: TerminalModes::default(),
            window_title: None,
            write_tx,
            changed: false,
            ctx: None,
            leftover_data: None,
            character_replace: DecSpecialGraphics::DontReplace,
            mouse_position: None,
            window_focused: true,
            window_commands: Vec::new(),
        }
    }

    #[must_use]
    pub fn show_cursor(&mut self) -> bool {
        self.get_current_buffer().show_cursor()
    }

    #[must_use]
    pub const fn is_changed(&self) -> bool {
        self.changed
    }

    pub fn set_state_changed(&mut self) {
        self.changed = true;
    }

    pub fn clear_changed(&mut self) {
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

    pub fn get_current_buffer(&mut self) -> &mut Buffer {
        match self.current_buffer {
            CurrentBuffer::Primary => &mut self.primary_buffer,
            CurrentBuffer::Alternate => &mut self.alternate_buffer,
        }
    }

    #[must_use]
    pub fn get_win_size(&mut self) -> (usize, usize) {
        self.get_current_buffer().terminal_buffer.get_win_size()
    }

    #[must_use]
    pub fn get_window_title(&self) -> Option<String> {
        self.window_title.clone()
    }

    pub(crate) fn data(&mut self, include_scrollback: bool) -> TerminalSections<Vec<TChar>> {
        self.get_current_buffer()
            .terminal_buffer
            .data(include_scrollback)
    }

    pub fn is_mouse_hovered_on_url(&mut self, pos: &CursorPos) -> Option<String> {
        let current_buffer = self.get_current_buffer();
        let buf_pos = current_buffer.terminal_buffer.cursor_pos_to_buf_pos(pos)?;

        let tags = self.get_current_buffer().format_tracker.tags();

        for tag in tags {
            if tag.url.is_none() {
                continue;
            }

            // check if the cursor pos is within the range of the tag
            if tag.start <= buf_pos && buf_pos < tag.end {
                if let Some(url) = &tag.url {
                    return Some(url.url.clone());
                }
            }
        }

        None
    }

    pub(crate) fn data_and_format_data_for_gui(
        &mut self,
    ) -> (
        TerminalSections<Vec<TChar>>,
        TerminalSections<Vec<FormatTag>>,
    ) {
        let (data, offset, end) = self.get_current_buffer().terminal_buffer.data_for_gui();

        let format_data = split_format_data_for_scrollback(
            self.get_current_buffer().format_tracker.tags(),
            offset,
            end,
            false,
        );

        (data, format_data)
    }

    #[must_use]
    pub fn cursor_pos(&mut self) -> CursorPos {
        self.get_current_buffer().cursor_state.pos.clone()
    }

    pub fn clear_window_title(&mut self) {
        self.window_title = None;
    }

    pub fn set_win_size(
        &mut self,
        width: usize,
        height: usize,
    ) -> TerminalBufferSetWinSizeResponse {
        let current_buffer = self.get_current_buffer();
        let response = current_buffer.terminal_buffer.set_win_size(
            width,
            height,
            &current_buffer.cursor_state.pos,
        );
        self.get_current_buffer().cursor_state.pos = response.new_cursor_pos.clone();

        response
    }

    #[must_use]
    pub fn get_cursor_key_mode(&self) -> Decckm {
        self.modes.cursor_key.clone()
    }

    pub fn set_window_focused(&mut self, focused: bool) {
        self.window_focused = focused;

        if self.modes.focus_reporting == XtMseWin::Disabled {
            return;
        }

        let to_write = if focused {
            TerminalInput::InFocus
        } else {
            TerminalInput::LostFocus
        };

        if let Err(e) = self.write(&to_write) {
            error!("Failed to write focus change: {e}");
        }

        debug!("Reported focus change to terminal");
    }

    pub(crate) fn handle_data(&mut self, data: &[u8]) {
        let data = match self.character_replace {
            //  Code page 1090
            // https://en.wikipedia.org/wiki/DEC_Special_Graphics / http://fileformats.archiveteam.org/wiki/DEC_Special_Graphics_Character_Set
            // 0x5f Blank	 	U+00A0 NO-BREAK SPACE
            // 0x60 Diamond	◆	U+25C6 BLACK DIAMOND
            // 0x61 Checkerboard	▒	U+2592 MEDIUM SHADE
            // 0x62 HT	␉	U+2409 SYMBOL FOR HORIZONTAL TABULATION
            // 0x63 FF	␌	U+240C SYMBOL FOR FORM FEED
            // 0x64 CR	␍	U+240D SYMBOL FOR CARRIAGE RETURN
            // 0x65 LF	␊	U+240A SYMBOL FOR LINE FEED
            // 0x66 Degree symbol	°	U+00B0 DEGREE SIGN
            // 0x67 Plus/minus	±	U+00B1 PLUS-MINUS SIGN
            // 0x68 NL	␤	U+2424 SYMBOL FOR NEWLINE
            // 0x69 VT	␋	U+240B SYMBOL FOR VERTICAL TABULATION
            // 0x6a Lower-right corner	┘	U+2518 BOX DRAWINGS LIGHT UP AND LEFT
            // 0x6b Upper-right corner	┐	U+2510 BOX DRAWINGS LIGHT DOWN AND LEFT
            // 0x6c Upper-left corner	┌	U+250C BOX DRAWINGS LIGHT DOWN AND RIGHT
            // 0x6d Lower-left corner	└	U+2514 BOX DRAWINGS LIGHT UP AND RIGHT
            // 0x6e Crossing Lines	┼	U+253C BOX DRAWINGS LIGHT VERTICAL AND HORIZONTAL
            // 0x6f Horizontal line - scan 1	⎺	U+23BA HORIZONTAL SCAN LINE-1
            // 0x70 Horizontal line - scan 3	⎻	U+23BB HORIZONTAL SCAN LINE-3
            // 0x71 Horizontal line - scan 5	─	U+2500 BOX DRAWINGS LIGHT HORIZONTAL
            // 0x72 Horizontal line - scan 7	⎼	U+23BC HORIZONTAL SCAN LINE-7
            // 0x73 Horizontal line - scan 9	⎽	U+23BD HORIZONTAL SCAN LINE-9
            // 0x74 Left "T"	├	U+251C BOX DRAWINGS LIGHT VERTICAL AND RIGHT
            // 0x75 Right "T"	┤	U+2524 BOX DRAWINGS LIGHT VERTICAL AND LEFT
            // 0x76 Bottom "T"	┴	U+2534 BOX DRAWINGS LIGHT UP AND HORIZONTAL
            // 0x77 Top "T"	┬	U+252C BOX DRAWINGS LIGHT DOWN AND HORIZONTAL
            // 0x78 Vertical bar	│	U+2502 BOX DRAWINGS LIGHT VERTICAL
            // 0x79 Less than or equal to	≤	U+2264 LESS-THAN OR EQUAL TO
            // 0x7a Greater than or equal to	≥	U+2265 GREATER-THAN OR EQUAL TO
            // 0x7b Pi	π	U+03C0 GREEK SMALL LETTER PI
            // 0x7c Not equal to	≠	U+2260 NOT EQUAL TO
            // 0x7d UK pound symbol	£	U+00A3 POUND SIGN
            // 0x7e Centered dot	·	U+00B7 MIDDLE DOT
            DecSpecialGraphics::Replace => {
                debug!("Replacing special graphics characters");
                // iterate through the characters and replace them with the appropriate unicode character
                let mut new_data = Vec::new();
                for c in data {
                    match c {
                        0x5f => new_data.extend_from_slice("\u{00A0}".as_bytes()),
                        0x60 => new_data.extend_from_slice("\u{25C6}".as_bytes()),
                        0x61 => new_data.extend_from_slice("\u{2592}".as_bytes()),
                        0x62 => new_data.extend_from_slice("\u{2409}".as_bytes()),
                        0x63 => new_data.extend_from_slice("\u{240C}".as_bytes()),
                        0x64 => new_data.extend_from_slice("\u{240D}".as_bytes()),
                        0x65 => new_data.extend_from_slice("\u{240A}".as_bytes()),
                        0x66 => new_data.extend_from_slice("\u{00B0}".as_bytes()),
                        0x67 => new_data.extend_from_slice("\u{00B1}".as_bytes()),
                        0x68 => new_data.extend_from_slice("\u{2424}".as_bytes()),
                        0x69 => new_data.extend_from_slice("\u{240B}".as_bytes()),
                        0x6a => new_data.extend_from_slice("\u{2518}".as_bytes()),
                        0x6b => new_data.extend_from_slice("\u{2510}".as_bytes()),
                        0x6c => new_data.extend_from_slice("\u{250C}".as_bytes()),
                        0x6d => new_data.extend_from_slice("\u{2514}".as_bytes()),
                        0x6e => new_data.extend_from_slice("\u{253C}".as_bytes()),
                        0x6f => new_data.extend_from_slice("\u{23BA}".as_bytes()),
                        0x70 => new_data.extend_from_slice("\u{23BB}".as_bytes()),
                        0x71 => new_data.extend_from_slice("\u{2500}".as_bytes()),
                        0x72 => new_data.extend_from_slice("\u{23BC}".as_bytes()),
                        0x73 => new_data.extend_from_slice("\u{23BD}".as_bytes()),
                        0x74 => new_data.extend_from_slice("\u{251C}".as_bytes()),
                        0x75 => new_data.extend_from_slice("\u{2524}".as_bytes()),
                        0x76 => new_data.extend_from_slice("\u{2534}".as_bytes()),
                        0x77 => new_data.extend_from_slice("\u{252C}".as_bytes()),
                        0x78 => new_data.extend_from_slice("\u{2502}".as_bytes()),
                        0x79 => new_data.extend_from_slice("\u{2264}".as_bytes()),
                        0x7a => new_data.extend_from_slice("\u{2265}".as_bytes()),
                        0x7b => new_data.extend_from_slice("\u{03C0}".as_bytes()),
                        0x7c => new_data.extend_from_slice("\u{2260}".as_bytes()),
                        0x7d => new_data.extend_from_slice("\u{00A3}".as_bytes()),
                        0x7e => new_data.extend_from_slice("\u{00B7}".as_bytes()),
                        _ => new_data.push(*c),
                    }
                }

                new_data
            }
            DecSpecialGraphics::DontReplace => data.to_vec(),
        };

        let current_buffer = self.get_current_buffer();

        let response = match current_buffer
            .terminal_buffer
            .insert_data(&current_buffer.cursor_state.pos, &data)
        {
            Ok(response) => response,
            Err(e) => {
                error!("Failed to insert data: {e}");
                return;
            }
        };

        current_buffer
            .format_tracker
            .push_range_adjustment(response.insertion_range);
        current_buffer
            .format_tracker
            .push_range(&current_buffer.cursor_state, response.written_range);
        current_buffer.cursor_state.pos = response.new_cursor_pos;
    }

    pub fn set_cursor_pos(&mut self, x: Option<usize>, y: Option<usize>) {
        let current_buffer = self.get_current_buffer();
        if let Some(x) = x {
            current_buffer.cursor_state.pos.x = x - 1;
        }
        if let Some(y) = y {
            current_buffer.cursor_state.pos.y = y - 1;
        }
    }

    pub fn set_cursor_pos_rel(&mut self, x: Option<i32>, y: Option<i32>) {
        let current_buffer = self.get_current_buffer();
        if let Some(x) = x {
            let x: i64 = x.into();
            let current_x: i64 = match current_buffer.cursor_state.pos.x.try_into() {
                Ok(x) => x,
                Err(e) => {
                    error!("Failed to convert x position to i64: {e}");
                    return;
                }
            };

            current_buffer.cursor_state.pos.x =
                usize::try_from((current_x + x).max(0)).unwrap_or(0);
        }
        if let Some(y) = y {
            let y: i64 = y.into();
            let current_y: i64 = match current_buffer.cursor_state.pos.y.try_into() {
                Ok(y) => y,
                Err(e) => {
                    error!("Failed to convert y position to i64: {e}");
                    return;
                }
            };
            // ensure y is not negative, and throw an error if it is
            current_buffer.cursor_state.pos.y =
                usize::try_from((current_y + y).max(0)).unwrap_or(0);
        }
    }

    pub(crate) fn clear_forwards(&mut self) {
        let current_buffer = self.get_current_buffer();
        match current_buffer
            .terminal_buffer
            .clear_forwards(&current_buffer.cursor_state.pos)
        {
            Ok(Some(buf_pos)) => {
                current_buffer
                    .format_tracker
                    .push_range(&current_buffer.cursor_state, buf_pos..usize::MAX);
            }
            // FIXME: why on god's green earth are we having an option type here?
            Ok(None) => (),
            Err(e) => {
                error!("Failed to clear forwards: {e}");
            }
        }
    }

    pub(crate) fn clear_backwards(&mut self) {
        let current_buffer = self.get_current_buffer();
        match current_buffer
            .terminal_buffer
            .clear_backwards(&current_buffer.cursor_state.pos)
        {
            Ok(Some(buf_pos)) => {
                current_buffer
                    .format_tracker
                    .push_range(&current_buffer.cursor_state, buf_pos);
            }
            Ok(None) => (),
            Err(e) => {
                error!("Failed to clear backwards: {e}");
            }
        }
    }

    pub(crate) fn clear_all(&mut self) {
        let current_buffer = self.get_current_buffer();
        current_buffer
            .format_tracker
            .push_range(&current_buffer.cursor_state, 0..usize::MAX);
        current_buffer.terminal_buffer.clear_all();
    }

    pub(crate) fn clear_visible(&mut self) {
        let current_buffer = self.get_current_buffer();

        let Some(range) = current_buffer.terminal_buffer.clear_visible() else {
            return;
        };

        if range.end > 0 {
            current_buffer
                .format_tracker
                .push_range(&current_buffer.cursor_state, range);
        }
    }

    pub(crate) fn clear_line_forwards(&mut self) {
        let current_buffer = self.get_current_buffer();

        if let Some(range) = current_buffer
            .terminal_buffer
            .clear_line_forwards(&current_buffer.cursor_state.pos)
        {
            match current_buffer.format_tracker.delete_range(range) {
                Ok(()) => (),
                Err(e) => {
                    error!("Failed to delete range: {e}");
                }
            }
        }
    }

    pub(crate) fn clear_line_backwards(&mut self) {
        let current_buffer = self.get_current_buffer();

        if let Some(range) = current_buffer
            .terminal_buffer
            .clear_line_backwards(&current_buffer.cursor_state.pos)
        {
            match current_buffer.format_tracker.delete_range(range) {
                Ok(()) => (),
                Err(e) => {
                    error!("Failed to delete range: {e}");
                }
            }
        }
    }

    pub(crate) fn clear_line(&mut self) {
        let current_buffer = self.get_current_buffer();

        if let Some(range) = current_buffer
            .terminal_buffer
            .clear_line(&current_buffer.cursor_state.pos)
        {
            match current_buffer.format_tracker.delete_range(range) {
                Ok(()) => (),
                Err(e) => {
                    error!("Failed to delete range: {e}");
                }
            }
        }
    }

    pub(crate) fn carriage_return(&mut self) {
        self.get_current_buffer().cursor_state.pos.x = 0;
    }

    pub(crate) fn new_line(&mut self) {
        self.get_current_buffer().cursor_state.pos.y += 1;
    }

    pub(crate) fn backspace(&mut self) {
        let current_buffer = self.get_current_buffer();

        if current_buffer.cursor_state.pos.x >= 1 {
            current_buffer.cursor_state.pos.x -= 1;
        } else {
            // FIXME: this is not correct, we should move to the end of the previous line
            warn!("FIXME: Backspace at the beginning of the line. Not wrapping");
        }
    }

    pub(crate) fn insert_lines(&mut self, num_lines: usize) {
        let current_buffer = self.get_current_buffer();

        let response = current_buffer
            .terminal_buffer
            .insert_lines(&current_buffer.cursor_state.pos, num_lines);
        match current_buffer
            .format_tracker
            .delete_range(response.deleted_range)
        {
            Ok(()) => (),
            Err(e) => {
                error!("Failed to delete range: {e}");
                return;
            }
        };
        current_buffer
            .format_tracker
            .push_range_adjustment(response.inserted_range);
    }

    pub(crate) fn delete(&mut self, num_chars: usize) {
        let current_buffer = self.get_current_buffer();

        let deleted_buf_range = current_buffer
            .terminal_buffer
            .delete_forwards(&current_buffer.cursor_state.pos, num_chars);
        if let Some(range) = deleted_buf_range {
            match current_buffer.format_tracker.delete_range(range) {
                Ok(()) => (),
                Err(e) => {
                    error!("Failed to delete range: {e}");
                }
            }
        }
    }

    pub(crate) fn erase(&mut self, num_chars: usize) {
        let current_buffer = self.get_current_buffer();

        let deleted_buf_range = current_buffer
            .terminal_buffer
            .erase_forwards(&current_buffer.cursor_state.pos, num_chars);
        if let Some(range) = deleted_buf_range {
            match current_buffer.format_tracker.delete_range(range) {
                Ok(()) => (),
                Err(e) => {
                    error!("Failed to delete range: {e}");
                }
            }
        }
    }

    pub(crate) fn reset(&mut self) {
        // FIXME: move these to the buffer struct
        let current_buffer = self.get_current_buffer();

        current_buffer.cursor_state.colors.set_default();
        current_buffer.cursor_state.font_weight = FontWeight::Normal;
        current_buffer.cursor_state.font_decorations.clear();
    }

    pub(crate) fn font_decordations_add_if_not_contains(&mut self, decoration: FontDecorations) {
        let current_buffer = self.get_current_buffer();

        if !current_buffer
            .cursor_state
            .font_decorations
            .contains(&decoration)
        {
            current_buffer
                .cursor_state
                .font_decorations
                .push(decoration);
        }
    }

    pub(crate) fn font_decorations_remove_if_contains(&mut self, decoration: &FontDecorations) {
        self.get_current_buffer()
            .cursor_state
            .font_decorations
            .retain(|d| *d != *decoration);
    }

    pub(crate) fn set_foreground(&mut self, color: TerminalColor) {
        self.get_current_buffer()
            .cursor_state
            .colors
            .set_color(color);
    }

    pub(crate) fn set_background(&mut self, color: TerminalColor) {
        self.get_current_buffer()
            .cursor_state
            .colors
            .set_background_color(color);
    }

    pub(crate) fn set_underline_color(&mut self, color: TerminalColor) {
        self.get_current_buffer()
            .cursor_state
            .colors
            .set_underline_color(color);
    }

    pub(crate) fn set_reverse_video(&mut self, reverse_video: ReverseVideo) {
        self.get_current_buffer()
            .cursor_state
            .colors
            .set_reverse_video(reverse_video);
    }

    pub(crate) fn sgr(&mut self, sgr: SelectGraphicRendition) {
        match sgr {
            SelectGraphicRendition::Reset => self.reset(),
            SelectGraphicRendition::Bold => {
                self.get_current_buffer().cursor_state.font_weight = FontWeight::Bold;
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
                self.get_current_buffer().cursor_state.font_weight = FontWeight::Normal;
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
                self.set_reverse_video(ReverseVideo::On);
            }
            SelectGraphicRendition::ResetReverseVideo => {
                self.set_reverse_video(ReverseVideo::Off);
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

    pub(crate) fn insert_spaces(&mut self, num_spaces: usize) {
        let current_buffer = self.get_current_buffer();

        let response = current_buffer
            .terminal_buffer
            .insert_spaces(&current_buffer.cursor_state.pos, num_spaces);
        current_buffer
            .format_tracker
            .push_range_adjustment(response.insertion_range);
    }

    pub(crate) fn set_mode(&mut self, mode: &Mode) {
        match mode {
            Mode::Decckm(decckm) => {
                self.modes.cursor_key = decckm.clone();
            }
            Mode::Decawm(decawm) => {
                self.get_current_buffer().cursor_state.line_wrap_mode = decawm.clone();
            }
            Mode::Dectem(dectem) => {
                self.get_current_buffer().show_cursor = dectem.clone();
            }
            Mode::BracketedPaste(bracketed_paste) => {
                self.modes.bracketed_paste = bracketed_paste.clone();
            }
            Mode::XtCBlink(xtcblink) => {
                self.modes.cursor_blinking = xtcblink.clone();
            }
            Mode::XtExtscrn(XtExtscrn::Alternate) => {
                debug!("Switching to alternate screen buffer");
                // SPEC Steps:
                // 1. Save the cursor position
                // 2. Switch to the alternate screen buffer
                // 3. Clear the screen

                // TODO: We're supposed to save the cursor POS here. Do we assign the current cursor pos to the saved cursor pos?
                // I don't see why we need to explicitly do that, as the cursor pos is already saved in the buffer
                // Do we copy the cursor pos to the new buffer?
                // Also, the "clear screen" bit implies to me that the buffer we switch to is *always* new, but is that correct?
                // This is why we're making a "new" buffer here
                self.current_buffer = CurrentBuffer::Alternate;
            }
            Mode::XtExtscrn(XtExtscrn::Primary) => {
                debug!("Switching to primary screen buffer");
                // SPEC Steps:
                // 1. Restore the cursor position
                // 2. Switch to the primary screen buffer
                // 3. Clear the screen
                // See set mode for notes on the cursor pos

                self.current_buffer = CurrentBuffer::Primary;
                let (width, height) = self.get_current_buffer().terminal_buffer.get_win_size();
                self.alternate_buffer = Buffer::new(width, height);
            }
            Mode::XtMseWin(XtMseWin::Enabled) => {
                debug!("Setting focus reporting");
                self.modes.focus_reporting = XtMseWin::Enabled;

                let to_write = if self.window_focused {
                    TerminalInput::InFocus
                } else {
                    TerminalInput::LostFocus
                };

                if let Err(e) = self.write(&to_write) {
                    error!("Failed to write focus change: {e}");
                }

                debug!("Reported current focus {:?} to terminal", to_write);
            }
            Mode::XtMseWin(XtMseWin::Disabled) => {
                self.modes.focus_reporting = XtMseWin::Disabled;
            }
            Mode::MouseMode(mode) => {
                if let MouseTrack::XtMsex10
                | MouseTrack::XtMseX11
                | MouseTrack::XtMseBtn
                | MouseTrack::NoTracking
                | MouseTrack::XtMseAny
                | MouseTrack::XtMseSgr = mode
                {
                    debug!("Setting mode to: {mode}");
                    self.modes.mouse_tracking = mode.clone();
                } else {
                    warn!("Unhandled mouse mode: {mode}");
                }
            }
            Mode::Unknown(_) => {
                warn!("unhandled set mode: {mode}");
            }
        }
    }

    pub(crate) fn osc_response(&mut self, osc: AnsiOscType) {
        match osc {
            AnsiOscType::Url(url) => match url {
                UrlResponse::End => {
                    self.get_current_buffer().cursor_state.url = None;
                }
                UrlResponse::Url(url_value) => {
                    self.get_current_buffer().cursor_state.url = Some(url_value);
                }
            },
            AnsiOscType::RequestColorQueryBackground(color) => {
                match color {
                    // OscInternalType::SetColor(_) => {
                    //     warn!("RequestColorQueryBackground: Set is not supported");
                    // }
                    AnsiOscInternalType::Query => {
                        // lets get the color as a hex string

                        let (r, g, b, a) = Color32::BLACK.to_tuple();
                        let output = collect_text(&format!(
                            "\x1b]11;rgb:{r:02x}/{g:02x}/{b:02x}{a:02x}\x1b\\"
                        ));

                        for byte in output.iter() {
                            self.write(byte)
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

                        let output = collect_text(&format!(
                            "\x1b]10;rgb:{r:02x}/{g:02x}/{b:02x}{a:02x}\x1b\\"
                        ));

                        for byte in output.iter() {
                            self.write(byte)
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

    pub(crate) fn report_cursor_position(&mut self) {
        let current_buffer = self.get_current_buffer();

        let x = current_buffer.cursor_state.pos.x + 1;
        let y = current_buffer.cursor_state.pos.y + 1;
        let output = collect_text(&format!("\x1b[{y};{x}R"));

        for input in output.iter() {
            self.write(input).expect("Failed to write cursor position");
        }
    }

    pub fn report_window_state(&mut self, minimized: bool) {
        let output = if minimized {
            collect_text(&"\x1b[2t".to_string())
        } else {
            collect_text(&"\x1b[1t".to_string())
        };
        for input in output.iter() {
            match self.write(input) {
                Ok(()) => (),
                Err(e) => {
                    error!("Failed to write window state: {e}");
                }
            }
        }
    }

    pub fn report_window_position(&mut self, x: usize, y: usize) {
        let output = collect_text(&format!("\x1b[3;{x};{y}t"));
        for input in output.iter() {
            match self.write(input) {
                Ok(()) => (),
                Err(e) => {
                    error!("Failed to write window position: {e}");
                }
            }
        }
    }

    pub fn report_window_size(&mut self, width: usize, height: usize) {
        let output = collect_text(&format!("\x1b[4;{height};{width}t"));
        for input in output.iter() {
            match self.write(input) {
                Ok(()) => (),
                Err(e) => {
                    error!("Failed to write window size: {e}");
                }
            }
        }
    }

    pub fn report_root_window_size(&mut self, width: usize, height: usize) {
        let output = collect_text(&format!("\x1b[5;{height};{width}t"));
        for input in output.iter() {
            match self.write(input) {
                Ok(()) => (),
                Err(e) => {
                    error!("Failed to write window size: {e}");
                }
            }
        }
    }

    pub fn report_character_size(&mut self, width: usize, height: usize) {
        let output = collect_text(&format!("\x1b[6;{height};{width}t"));
        for input in output.iter() {
            match self.write(input) {
                Ok(()) => (),
                Err(e) => {
                    error!("Failed to write character size: {e}");
                }
            }
        }
    }

    pub fn report_terminal_size_in_characters(&mut self, width: usize, height: usize) {
        let output = collect_text(&format!("\x1b[8;{height};{width}t"));
        for input in output.iter() {
            match self.write(input) {
                Ok(()) => (),
                Err(e) => {
                    error!("Failed to write terminal size in characters: {e}");
                }
            }
        }
    }

    pub(crate) fn clip_buffer_lines(&mut self) {
        let current_buffer = self.get_current_buffer();

        if let Some(range) = current_buffer.terminal_buffer.clip_lines() {
            match current_buffer.format_tracker.delete_range(range) {
                Ok(()) => (),
                Err(e) => {
                    error!("Failed to delete range: {e}");
                }
            }
        }
    }

    pub fn handle_incoming_data(&mut self, incoming: &[u8]) {
        #[cfg(debug_assertions)]
        let now = Instant::now();
        // if we have leftover data, prepend it to the incoming data
        let mut incoming = self.leftover_data.take().map_or_else(
            || incoming.to_vec(),
            |leftover_data| {
                debug!("We have leftover data: {:?}", leftover_data);
                let mut new_data = Vec::with_capacity(leftover_data.len() + incoming.len());
                new_data.extend_from_slice(&leftover_data);
                new_data.extend_from_slice(incoming);
                trace!("Reassembled buffer: {:?}", new_data);
                self.leftover_data = None;
                new_data
            },
        );

        let mut leftover_bytes = vec![];
        while let Err(_e) = String::from_utf8(incoming.clone()) {
            let Some(p) = incoming.pop() else { break };
            leftover_bytes.insert(0, p);
        }

        if !leftover_bytes.is_empty() {
            match self.leftover_data {
                Some(ref mut self_leftover) => {
                    // this should be at the start of the leftover data
                    self_leftover.splice(0..0, leftover_bytes);
                }
                None => self.leftover_data = Some(leftover_bytes),
            }
        }

        // verify that the incoming data is utf-8
        let parsed = self.parser.push(&incoming);

        for segment in parsed {
            // if segment is not data, we want to print out the segment
            if let TerminalOutput::Data(data) = &segment {
                debug!(
                    "Incoming segment: {:?}",
                    str::from_utf8(data)
                        .unwrap_or("Failed to parse data for display as string: {data:?}")
                );
            } else {
                debug!("Incoming segment: {segment:?}");
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
                TerminalOutput::Erase(num_chars) => self.erase(num_chars),
                TerminalOutput::Sgr(sgr) => self.sgr(sgr),
                TerminalOutput::Mode(mode) => self.set_mode(&mode),
                TerminalOutput::InsertSpaces(num_spaces) => self.insert_spaces(num_spaces),
                TerminalOutput::OscResponse(osc) => self.osc_response(osc),
                TerminalOutput::DecSpecialGraphics(dec_special_graphics) => {
                    self.character_replace = dec_special_graphics;
                }
                TerminalOutput::CursorReport => self.report_cursor_position(),
                TerminalOutput::Skipped | TerminalOutput::Bell => (),
                TerminalOutput::ApplicationKeypadMode => {
                    self.modes.cursor_key = Decckm::Application;
                }
                TerminalOutput::NormalKeypadMode => self.modes.cursor_key = Decckm::Ansi,
                TerminalOutput::CursorVisualStyle(style) => {
                    debug!("Ignoring cursor visual style: {style:?}");
                }
                TerminalOutput::WindowManipulation(manip) => self.window_commands.push(manip),
                TerminalOutput::Invalid => {
                    info!("Unhandled terminal output: {segment:?}");
                }
            }
        }

        // now ensure total lines in buffer are not more then 1000 lines
        self.clip_buffer_lines();

        #[cfg(debug_assertions)]
        // log the frame time
        let elapsed = now.elapsed();
        // show either elapsed as micros or millis, depending on the duration
        #[cfg(debug_assertions)]
        if elapsed.as_millis() > 0 {
            debug!("Data processing time: {}ms", elapsed.as_millis());
        } else {
            debug!("Data processing time: {}μs", elapsed.as_micros());
        }

        self.set_state_changed();
        self.request_redraw();
    }

    /// Write data to the terminal
    ///
    /// # Errors
    /// Will return an error if the write fails
    pub fn write(&self, to_write: &TerminalInput) -> Result<()> {
        match to_write.to_payload(
            self.get_cursor_key_mode() == Decckm::Application,
            self.get_cursor_key_mode() == Decckm::Application,
        ) {
            TerminalInputPayload::Single(c) => {
                self.write_tx.send(PtyWrite::Write(vec![c]))?;
            }
            TerminalInputPayload::Many(to_write) => {
                self.write_tx.send(PtyWrite::Write(to_write.to_vec()))?;
            }
        };

        Ok(())
    }

    pub fn scroll(&mut self, scroll: f32) {
        let current_buffer = &mut self.get_current_buffer().terminal_buffer;
        // convert the scroll to usize, with a minimum of 1
        let mut scroll = scroll.round();

        if scroll < 0.0 {
            scroll *= -1.0;
            let scroll_as_usize = match scroll.max(1.0).approx_as::<usize>() {
                Ok(scroll) => scroll,
                Err(e) => {
                    error!("Failed to convert scroll to usize: {e}\nUsing default of 1");
                    1
                }
            };

            let scoller = ScrollDirection::Down(scroll_as_usize);
            current_buffer.scroll(&scoller);
        } else {
            let scroll_as_usize = match scroll.max(1.0).approx_as::<usize>() {
                Ok(scroll) => scroll,
                Err(e) => {
                    error!("Failed to convert scroll to usize: {e}\nUsing default of 1");
                    1
                }
            };

            let scroller = ScrollDirection::Up(scroll_as_usize);
            current_buffer.scroll(&scroller);
        }
    }
}
