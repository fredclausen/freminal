// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

mod ansi;
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
pub mod state {
    pub mod buffer;
    pub mod cursor;
    pub mod data;
    pub mod fonts;
    pub mod internal;
    pub mod term_char;
}

use std::{sync::Arc, sync::Mutex};

use ansi_components::mode::{Decawm, Decckm, Mode, TerminalModes};
use anyhow::Result;
use crossbeam_channel::{unbounded, Receiver};
pub use format_tracker::FormatTag;
pub use io::{FreminalPtyInputOutput, FreminalTermInputOutput};
use io::{FreminalTerminalSize, PtyRead, PtyWrite};
use state::{
    cursor::CursorPos,
    data::TerminalSections,
    internal::{TerminalState, TERMINAL_HEIGHT, TERMINAL_WIDTH},
    term_char::TChar,
};

use crate::{gui::colors::TerminalColor, Args};

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
) -> TerminalSections<Vec<FormatTag>> {
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

    TerminalSections {
        scrollback: scrollback_tags,
        visible: canvas_tags,
    }
}

pub struct TerminalEmulator<Io: FreminalTermInputOutput> {
    pub internal: Arc<Mutex<TerminalState>>,
    _io: Io,
    write_tx: crossbeam_channel::Sender<PtyWrite>,
}

impl TerminalEmulator<FreminalPtyInputOutput> {
    pub fn new(args: &Args) -> Result<(Self, Receiver<PtyRead>)> {
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
            internal: Mutex::new(TerminalState::new(write_tx.clone())).into(),
            _io: io,
            write_tx,
        };
        Ok((ret, pty_rx))
    }
}

impl<Io: FreminalTermInputOutput> TerminalEmulator<Io> {
    pub fn get_win_size(&self) -> (usize, usize) {
        self.internal.lock().unwrap().get_win_size()
    }

    pub fn get_window_title(&self) -> Option<String> {
        self.internal.lock().unwrap().get_window_title()
    }

    #[allow(dead_code)]
    pub fn clear_window_title(&self) {
        self.internal.lock().unwrap().clear_window_title();
    }

    pub fn set_win_size(
        &self,
        width_chars: usize,
        height_chars: usize,
        font_pixel_width: usize,
        font_pixel_height: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let response = self
            .internal
            .lock()
            .unwrap()
            .set_win_size(width_chars, height_chars);

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
        self.internal.lock().unwrap().write(to_write)
    }

    pub fn data(&self) -> TerminalSections<Vec<TChar>> {
        let result = self.internal.lock().unwrap().data();

        result
    }

    pub fn format_data(&self) -> TerminalSections<Vec<FormatTag>> {
        self.internal.lock().unwrap().format_data()
    }

    pub fn cursor_pos(&self) -> CursorPos {
        self.internal.lock().unwrap().cursor_pos()
    }
}

#[cfg(test)]
mod test {
    use state::fonts::FontWeight;

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
