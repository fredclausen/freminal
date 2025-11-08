// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::gui::{
    mouse::{
        handle_pointer_button, handle_pointer_moved, handle_pointer_scroll, FreminalMousePosition,
        PreviousMouseState,
    },
    render_state::TerminalRenderState,
    TerminalEmulator,
};

use freminal_terminal_emulator::{
    ansi_components::modes::rl_bracket::RlBracket,
    format_tracker::FormatTag,
    interface::{collect_text, TerminalInput},
    io::FreminalTermInputOutput,
    state::{cursor::CursorPos, fonts::FontDecorations, term_char::TChar},
};

use eframe::egui::{
    self, scroll_area::ScrollBarVisibility, text::LayoutJob, Color32, Context, CursorIcon,
    DragValue, Event, InputState, Key, Modifiers, OpenUrl, OutputCommand, PointerButton, Pos2,
    Rect, Stroke, TextFormat, TextStyle, Ui,
};

use super::{
    colors::internal_color_to_egui,
    fonts::{get_char_size, setup_font_files, TerminalFont},
};
use anyhow::Result;
use conv2::{ConvUtil, ValueFrom};
use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Default, Clone)]
pub struct CachedRow {
    text_hash: u64,
    galley: Option<Arc<egui::Galley>>,
    // Cached background runs as (col_start, len, color)
    bg_runs: Vec<(usize, usize, egui::Color32)>,
}

fn color32_to_u32(color: egui::Color32) -> u32 {
    let [red, green, blue, alpha] = color.to_array();
    (u32::from(alpha) << 24) | (u32::from(red) << 16) | (u32::from(green) << 8) | u32::from(blue)
}

fn hash_row(text: &str, sections: &[egui::text::LayoutSection], row_len_bytes: usize) -> u64 {
    use ahash::AHasher;
    let mut hasher = AHasher::default();
    text.hash(&mut hasher);
    for sec in sections {
        let s = sec.byte_range.start.min(row_len_bytes);
        let e = sec.byte_range.end.min(row_len_bytes);
        s.hash(&mut hasher);
        e.hash(&mut hasher);
        color32_to_u32(sec.format.color).hash(&mut hasher);
        color32_to_u32(sec.format.background).hash(&mut hasher);
        (sec.format.font_id.size.to_bits()).hash(&mut hasher);
    }
    hasher.finish()
}

fn control_key(key: Key) -> Option<Cow<'static, [TerminalInput]>> {
    if key >= Key::A && key <= Key::Z {
        let name = key.name();
        assert!(name.len() == 1);
        let name_c = name.as_bytes()[0];
        return Some(vec![TerminalInput::Ctrl(name_c)].into());
    } else if key == Key::OpenBracket {
        return Some([TerminalInput::Ctrl(b'[')].as_ref().into());
    } else if key == Key::CloseBracket {
        return Some([TerminalInput::Ctrl(b']')].as_ref().into());
    } else if key == Key::Backslash {
        return Some([TerminalInput::Ctrl(b'\\')].as_ref().into());
    }

    None
}

#[allow(
    clippy::cognitive_complexity,
    clippy::too_many_lines,
    clippy::too_many_arguments
)]
fn write_input_to_terminal<Io: FreminalTermInputOutput>(
    input: &InputState,
    terminal_emulator: &mut TerminalEmulator<Io>,
    character_size_x: f32,
    character_size_y: f32,
    last_reported_mouse_pos: Option<PreviousMouseState>,
    repeat_characters: bool,
    previous_key: Option<Key>,
    scroll_amount: f32,
) -> (bool, Option<PreviousMouseState>, Option<Key>, f32) {
    if input.raw.events.is_empty() {
        return (false, last_reported_mouse_pos, previous_key, scroll_amount);
    }

    let mut previous_key = previous_key;
    let mut state_changed = false;
    let mut last_reported_mouse_pos = last_reported_mouse_pos;
    let mut left_mouse_button_pressed = false;
    let mut scroll_amount = scroll_amount;

    for event in &input.raw.events {
        debug!("event: {:?}", event);
        if let Event::Key { pressed: false, .. } = event {
            previous_key = None;
        }

        let inputs: Cow<'static, [TerminalInput]> = match event {
            // FIXME: We don't support separating out numpad vs regular keys
            // This is an egui issue. See: https://github.com/emilk/egui/issues/3653
            Event::Text(text) => {
                if repeat_characters || previous_key.is_none() {
                    collect_text(text)
                } else {
                    continue;
                }
            }
            Event::Key {
                key: Key::Enter,
                pressed: true,
                modifiers,
                ..
            } => {
                if modifiers.is_none() {
                    [TerminalInput::Enter].as_ref().into()
                } else {
                    continue;
                }
            }
            // https://github.com/emilk/egui/issues/3653
            // FIXME: Technically not correct if we were on a mac, but also we are using linux
            // syscalls so we'd have to solve that before this is a problem
            Event::Copy => [TerminalInput::Ctrl(b'c')].as_ref().into(),
            Event::Key {
                key: Key::J | Key::K,
                pressed: true,
                modifiers: Modifiers { ctrl: true, .. },
                ..
            } => [TerminalInput::LineFeed].as_ref().into(),
            Event::Key {
                key,
                pressed: true,
                modifiers: Modifiers { ctrl: true, .. },
                ..
            } => {
                if let Some(inputs) = control_key(*key) {
                    inputs
                } else {
                    error!("Unexpected ctrl key: {}", key.name());
                    continue;
                }
            }
            Event::Key {
                key: Key::Backspace,
                pressed: true,
                ..
            } => [TerminalInput::Backspace].as_ref().into(),
            Event::Key {
                key: Key::ArrowUp,
                pressed: true,
                ..
            } => [TerminalInput::ArrowUp].as_ref().into(),
            Event::Key {
                key: Key::ArrowDown,
                pressed: true,
                ..
            } => [TerminalInput::ArrowDown].as_ref().into(),
            Event::Key {
                key: Key::ArrowLeft,
                pressed: true,
                ..
            } => [TerminalInput::ArrowLeft].as_ref().into(),
            Event::Key {
                key: Key::ArrowRight,
                pressed: true,
                ..
            } => [TerminalInput::ArrowRight].as_ref().into(),
            Event::Key {
                key: Key::Home,
                pressed: true,
                ..
            } => [TerminalInput::Home].as_ref().into(),
            Event::Key {
                key: Key::End,
                pressed: true,
                ..
            } => [TerminalInput::End].as_ref().into(),
            Event::Key {
                key: Key::Delete,
                pressed: true,
                ..
            } => [TerminalInput::Delete].as_ref().into(),
            Event::Key {
                key: Key::Insert,
                pressed: true,
                ..
            } => [TerminalInput::Insert].as_ref().into(),
            Event::Key {
                key: Key::PageUp,
                pressed: true,
                ..
            } => [TerminalInput::PageUp].as_ref().into(),
            Event::Key {
                key: Key::PageDown,
                pressed: true,
                ..
            } => [TerminalInput::PageDown].as_ref().into(),
            Event::Key {
                key: Key::Tab,
                pressed: true,
                ..
            } => [TerminalInput::Tab].as_ref().into(),

            // log any Event::Key that we don't handle
            // Event::Key { key, pressed: true, .. } => {
            //     warn!("Unhandled key event: {:?}", key);
            //     continue;
            // }
            Event::Key {
                key: Key::Escape,
                pressed: true,
                ..
            } => [TerminalInput::Escape].as_ref().into(),
            Event::Key {
                key,
                pressed: true,
                repeat: true,
                ..
            } => {
                previous_key = Some(*key);
                continue;
            }
            Event::Paste(text) => {
                let bracked_paste_mode = terminal_emulator.internal.modes.bracketed_paste.clone();
                if bracked_paste_mode == RlBracket::Enabled {
                    // ESC [ 200 ~, followed by the pasted text, followed by ESC [ 201 ~.

                    collect_text(&format!("\x1b[200~{}{}", text, "\x1b[201~"))
                } else {
                    collect_text(text)
                }
            }
            Event::PointerGone => {
                terminal_emulator.set_mouse_position(&None);
                last_reported_mouse_pos = None;
                continue;
            }
            Event::WindowFocused(focused) => {
                terminal_emulator.set_window_focused(*focused);

                if !*focused {
                    last_reported_mouse_pos = None;
                }

                continue;
            }
            Event::PointerMoved(pos) => {
                terminal_emulator.set_mouse_position_from_move_event(pos);
                let (x, y) =
                    encode_egui_mouse_pos_as_usize(*pos, (character_size_x, character_size_y));

                let position = FreminalMousePosition::new(x, y, pos.x, pos.y);
                let (previous, current) =
                    if let Some(last_mouse_position) = &mut last_reported_mouse_pos {
                        (
                            last_mouse_position.clone(),
                            last_mouse_position.new_from_previous_mouse_state(position),
                        )
                    } else {
                        (
                            PreviousMouseState::default(),
                            PreviousMouseState::new(
                                PointerButton::Primary,
                                false,
                                position,
                                Modifiers::default(),
                            ),
                        )
                    };

                let res = handle_pointer_moved(
                    &previous,
                    &current,
                    &terminal_emulator.internal.modes.mouse_tracking,
                );

                last_reported_mouse_pos = Some(current);

                if let Some(res) = res {
                    res
                } else {
                    continue;
                }
            }
            Event::PointerButton {
                button,
                pressed,
                modifiers,
                pos,
            } => {
                state_changed = true;

                let (x, y) =
                    encode_egui_mouse_pos_as_usize(*pos, (character_size_x, character_size_y));
                let mouse_pos = FreminalMousePosition::new(x, y, pos.x, pos.y);
                let new_mouse_position =
                    PreviousMouseState::new(*button, *pressed, mouse_pos.clone(), *modifiers);
                // let previous_mouse_button =
                //     if let Some(last_reported_mouse_pos) = &last_reported_mouse_pos {
                //         last_reported_mouse_pos.button
                //     } else {
                //         PointerButton::None
                //     };
                let response = handle_pointer_button(
                    *button,
                    &new_mouse_position,
                    &terminal_emulator.internal.modes.mouse_tracking,
                );

                last_reported_mouse_pos = Some(new_mouse_position.clone());

                if *button == PointerButton::Primary && *pressed {
                    left_mouse_button_pressed = true;
                }

                if let Some(response) = response {
                    response
                } else {
                    continue;
                }
            }
            Event::MouseWheel {
                delta,
                modifiers,
                unit,
            } => {
                match unit {
                    egui::MouseWheelUnit::Point => {
                        scroll_amount += delta.y;
                    }
                    egui::MouseWheelUnit::Line => {
                        scroll_amount += delta.y * character_size_y;
                    }
                    egui::MouseWheelUnit::Page => {
                        error!("Unhandled MouseWheelUnit: {:?}", unit);
                        continue;
                    }
                }
                // TODO: should we care if we scrolled in the x axis?

                if scroll_amount.abs() < character_size_y {
                    continue;
                }

                // the amount scrolled should be in increments of the character size
                // the remaineder should be added to the next scroll event

                let scroll_amount_to_do = scroll_amount.floor();
                scroll_amount -= scroll_amount_to_do;

                state_changed = true;

                if let Some(last_mouse_position) = &mut last_reported_mouse_pos {
                    // update the modifiers if necessary
                    if last_mouse_position.modifiers != *modifiers {
                        last_mouse_position.modifiers = *modifiers;
                        *last_mouse_position = last_mouse_position.clone();
                    }
                    let response = handle_pointer_scroll(
                        egui::Vec2::new(0.0, scroll_amount_to_do / character_size_y),
                        last_mouse_position,
                        &terminal_emulator.internal.modes.mouse_tracking,
                    );

                    if let Some(response) = response {
                        response
                    } else {
                        terminal_emulator
                            .internal
                            .scroll(scroll_amount_to_do / character_size_y);

                        continue;
                    }
                } else {
                    terminal_emulator
                        .internal
                        .scroll(scroll_amount_to_do / character_size_y);

                    continue;
                }
            }
            _ => {
                continue;
            }
        };

        for input in inputs.as_ref() {
            state_changed = true;
            if let Err(e) = terminal_emulator.write(input) {
                error!("Failed to write input to terminal emulator: {}", e);
            }
        }
    }

    if state_changed {
        debug!("Inputs detected, setting previous pass invalid");
        terminal_emulator.set_previous_pass_invalid();
    }

    (
        left_mouse_button_pressed,
        last_reported_mouse_pos,
        previous_key,
        scroll_amount,
    )
}

fn encode_egui_mouse_pos_as_usize(pos: Pos2, character_size: (f32, f32)) -> (usize, usize) {
    let x = ((pos.x / character_size.0).floor())
        .approx_as::<usize>()
        .unwrap_or_else(|_| {
            if pos.x > 0.0 {
                error!("Failed to convert {} to usize. Using default of 255", pos.x);
                255
            } else {
                error!("Failed to convert {} to usize. Using default of 0", pos.x);
                0
            }
        });
    let y = ((pos.y / character_size.1).floor())
        .approx_as::<usize>()
        .unwrap_or_else(|_| {
            if pos.x > 0.0 {
                error!("Failed to convert {} to usize. Using default of 255", pos.y);
                255
            } else {
                error!("Failed to convert {} to usize. Using default of 0", pos.y);
                0
            }
        });

    (x, y)
}

fn paint_cursor(
    label_rect: Rect,
    character_size: (f32, f32),
    cursor_pos: &CursorPos,
    ui: &Ui,
    color: Color32,
) {
    let painter = ui.painter();

    let top = label_rect.top();
    let left = label_rect.left();

    let cursor_y = match f32::value_from(cursor_pos.y) {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to convert cursor y ({0}) to f32: {e}", cursor_pos.y);
            return;
        }
    };

    let cursor_x = match f32::value_from(cursor_pos.x) {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to convert cursor x ({0}) to f32: {e}", cursor_pos.x);
            return;
        }
    };

    let y_offset: f32 = cursor_y * character_size.1;
    let x_offset: f32 = cursor_x * character_size.0;
    painter.rect_filled(
        Rect::from_min_size(
            egui::pos2(left + x_offset, top + y_offset),
            egui::vec2(character_size.0, character_size.1),
        ),
        0.0,
        color,
    );
}

fn setup_bg_fill(ctx: &egui::Context) {
    ctx.style_mut(|style| {
        style.visuals.window_fill = internal_color_to_egui(
            freminal_common::colors::TerminalColor::DefaultBackground,
            false,
        );
        style.visuals.panel_fill = internal_color_to_egui(
            freminal_common::colors::TerminalColor::DefaultBackground,
            false,
        );
    });
}

fn create_terminal_output_layout_job(
    data: &[TChar],
    format_data: &[FormatTag],
) -> Result<(String, Vec<FormatTag>)> {
    if data.is_empty() {
        return Ok((String::new(), Vec::new()));
    }
    let mut offset = Vec::with_capacity(data.len());

    // Convert data into an array of bytes
    let mut data_converted = Vec::with_capacity(data.len());
    for c in data {
        let offset_amount = match c {
            TChar::NewLine => {
                data_converted.push(b'\n');
                1
            }
            TChar::Space => {
                data_converted.push(b' ');
                1
            }
            TChar::Ascii(c) => {
                data_converted.push(*c);
                1
            }
            TChar::Utf8(all) => {
                data_converted.extend_from_slice(all);
                all.len()
            }
        };

        offset.push(data_converted.len() - offset_amount);
    }

    let data_utf8 = match std::str::from_utf8(&data_converted) {
        Ok(v) => v,
        Err(e) => {
            error!(
                "Create output job: Failed to convert terminal data to utf8: {}",
                e
            );
            return Err(e.into());
        }
    };

    // Map the format data to the utf8 data
    // Shift the format data for the number of added bytes (utf8) for any TChar found in the input data

    let mut format_data_shifted = Vec::with_capacity(format_data.len());
    for tag in format_data {
        // Adjust byte_offset based on the length of utf8 characters
        let start = if tag.start < offset.len() {
            offset[tag.start]
        } else {
            offset[offset.len() - 1]
        };

        let end = if tag.start == tag.end {
            start
        } else if tag.end == usize::MAX || tag.end >= offset.len() {
            data_converted.len()
        } else {
            offset[tag.end]
        };

        assert!(
            start <= end,
            "Start is greater than end. Start: {start}, End: {end}, tag: {tag:?}"
        );

        format_data_shifted.push(FormatTag {
            start,
            end,
            colors: tag.colors.clone(),
            font_weight: tag.font_weight.clone(),
            font_decorations: tag.font_decorations.clone(),
            url: tag.url.clone(),
        });
    }

    #[cfg(feature = "validation")]
    match validate_tags_to_buffer(data_utf8.as_bytes(), &format_data_shifted) {
        Ok(()) => Ok((data_utf8.to_string(), format_data_shifted)),
        Err(e) => {
            error!("Failed to validate tags to buffer: {}", e);
            Err(e)
        }
    }

    #[cfg(not(any(feature = "validation")))]
    Ok((data_utf8.to_string(), format_data_shifted))
}

// Small function to help validate the tags to the buffer
// We don't want this normally, as it's a performance hit and once the kinks are worked out
// This is likely not needed
#[cfg(feature = "validation")]
fn validate_tags_to_buffer(buffer: &[u8], tags: &[FormatTag]) -> Result<()> {
    // loop over the tags and validate that the start and end are within the bounds of the buffer
    for tag in tags {
        if tag.start >= buffer.len() {
            warn!(
                "Tag start is greater than buffer length: start: {start}, buffer length: {buffer_len}",
                start = tag.start,
                buffer_len = buffer.len()
            );

            continue;
        }

        // now verify that the slice represented by the range tag.start..end is valid utf8

        if let Err(e) = std::str::from_utf8(&buffer[tag.start..tag.end]) {
            error!(
                "Tag range is not valid utf8: start: {start}, end: {end}, buffer length: {buffer_len}, error: {error}",
                start = tag.start,
                end = tag.end,
                buffer_len = buffer.len(),
                error = e
            );

            Err(e)?;
        }
    }

    Ok(())
}

#[derive(Default, Clone, Debug)]
pub struct UiJobAction {
    text: String,
    adjusted_format_data: Vec<FormatTag>,
}

#[derive(Debug)]
pub struct NewJobAction<'a> {
    text: &'a [TChar],
    format_data: Vec<FormatTag>,
}

#[derive(Debug)]
pub enum UiData<'a> {
    NewPass(&'a NewJobAction<'a>),
    PreviousPass(UiJobAction),
}

fn setup_job(ui: &Ui, data_utf8: &str) -> (egui::text::LayoutJob, egui::TextFormat) {
    let width = ui.available_width();
    let style = ui.style();
    let text_style = &style.text_styles[&TextStyle::Monospace];

    let mut job = egui::text::LayoutJob::simple(
        data_utf8.to_string(),
        text_style.clone(),
        style.visuals.text_color(),
        width,
    );
    job.wrap.break_anywhere = true;
    let textformat = job.sections[0].format.clone();
    job.sections.clear();

    (job, textformat)
}

fn process_tags(
    adjusted_format_data: &Vec<FormatTag>,
    data_len: usize,
    textformat: &mut TextFormat,
    font_size: f32,
    job: &mut LayoutJob,
    #[cfg(feature = "validation")] buffer: &[u8],
) {
    let terminal_fonts = TerminalFont::new();

    let mut range;
    let mut color;
    let mut background_color;
    let mut underline_color;

    for tag in adjusted_format_data {
        range = tag.start..tag.end;
        color = tag.colors.get_color();
        background_color = tag.colors.get_background_color();
        underline_color = tag.colors.get_underline_color();

        if range.end == usize::MAX {
            range.end = data_len;
        }

        match range.start.cmp(&data_len) {
            std::cmp::Ordering::Greater => {
                #[cfg(feature = "validation")]
                warn!("Skipping unusable format data");
                continue;
            }
            std::cmp::Ordering::Equal => {
                continue;
            }
            std::cmp::Ordering::Less => (),
        }

        if range.end > data_len {
            #[cfg(feature = "validation")]
            warn!("Truncating format data end");
            range.end = data_len;
        }

        textformat.font_id.family =
            terminal_fonts.get_family(&tag.font_decorations, &tag.font_weight);
        textformat.font_id.size = font_size;
        let make_faint = tag.font_decorations.contains(&FontDecorations::Faint);
        textformat.color = internal_color_to_egui(color, make_faint);
        // FIXME: ????? should background be faint? I feel like no, but....
        textformat.background = internal_color_to_egui(background_color, make_faint);
        if tag.font_decorations.contains(&FontDecorations::Underline) {
            let underline_color_converted = internal_color_to_egui(underline_color, make_faint);

            textformat.underline = Stroke::new(1.0, underline_color_converted);
        } else {
            textformat.underline = Stroke::new(0.0, textformat.color);
        }

        if tag
            .font_decorations
            .contains(&FontDecorations::Strikethrough)
        {
            textformat.strikethrough = Stroke::new(1.0, textformat.color);
        } else {
            textformat.strikethrough = Stroke::new(0.0, textformat.color);
        }

        // Validate the range is valid utf8
        #[cfg(feature = "validation")]
        if std::str::from_utf8(&buffer[range.clone()]).is_err() {
            warn!("Range is not valid utf8");
            continue;
        }

        job.sections.push(egui::text::LayoutSection {
            leading_space: 0.0f32,
            byte_range: range,
            format: textformat.clone(),
        });
    }
}

#[allow(clippy::too_many_lines)]
pub fn render_terminal_text(
    ui: &mut egui::Ui,
    full_text: &str,
    job: &egui::text::LayoutJob,
    font_size: f32,
    row_cache: &mut Vec<CachedRow>,
    dirty_rows: Option<&[usize]>,
) -> egui::Response {
    // --- Font metrics ---------------------------------------------------------
    let (glyph_width, row_height) = ui.ctx().fonts_mut(|fonts| {
        let font_id = egui::FontId::monospace(font_size);
        let glyph_width = fonts.glyph_width(&font_id, 'W');
        let row_height = fonts.row_height(&font_id);
        (glyph_width, row_height)
    });

    // Optional dirty-row filter: when provided, we only recompute expensive layout
    // for rows listed here. Clean rows reuse cached galley+bg runs.
    let dirty_set: Option<std::collections::HashSet<usize>> =
        dirty_rows.map(|rows| rows.iter().copied().collect());
    // --- Layout geometry ------------------------------------------------------
    let longest = full_text
        .split('\n')
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0usize);

    let total_lines = full_text.lines().count();
    let total_height = f32::value_from(total_lines).unwrap_or_default() * row_height;
    let width_f32 = f32::value_from(longest).unwrap_or_default();

    // --- Ensure row cache -----------------------------------------------------
    if row_cache.len() < total_lines {
        row_cache.resize_with(total_lines, Default::default);
    }

    // --- Allocate paint region ------------------------------------------------
    let (response, painter) = ui.allocate_painter(
        egui::vec2(glyph_width * width_f32, total_height),
        egui::Sense::hover(),
    );

    // --- Compute line byte ranges --------------------------------------------
    let mut line_ranges = Vec::with_capacity(total_lines);
    let mut offset = 0usize;
    for line in full_text.split_inclusive('\n') {
        let len = line.len();
        line_ranges.push((offset, offset + len));
        offset += len;
    }

    // --- Render rows ----------------------------------------------------------
    for (row_idx, (row_start, row_end)) in line_ranges.iter().enumerate() {
        let row_text_raw = &full_text[*row_start..*row_end];
        let row_text = row_text_raw.strip_suffix('\n').unwrap_or(row_text_raw);
        let row_origin = response.rect.left_top()
            + egui::vec2(
                0.0,
                f32::value_from(row_idx).unwrap_or_default() * row_height,
            );

        // Fast path for clean rows: reuse cached background runs and galley
        if let Some(ds) = &dirty_set {
            if !ds.contains(&row_idx) {
                let cache = &row_cache[row_idx];
                if let Some(gal) = &cache.galley {
                    // Paint cached backgrounds
                    for (col_start, len, color) in &cache.bg_runs {
                        let x0 = row_origin.x
                            + f32::value_from(*col_start)
                                .unwrap_or_default()
                                .mul_add(glyph_width, 0.0);
                        let len_f = f32::value_from(*len).unwrap_or_default();
                        let rect = egui::Rect::from_min_size(
                            egui::pos2(x0, row_origin.y),
                            egui::vec2(len_f * glyph_width, row_height),
                        );
                        painter.rect_filled(rect, 0.0, *color);
                    }
                    // Paint cached text galley
                    painter.galley(row_origin, gal.clone(), egui::Color32::WHITE);
                    continue;
                }
            }
        }

        // --- Byte → column map for multibyte-safe geometry --------------------
        let mut byte_to_col = vec![0usize; row_text.len() + 1];
        {
            let mut col = 0usize;
            for (b, _) in row_text.char_indices() {
                byte_to_col[b] = col;
                col += 1;
            }
            byte_to_col[row_text.len()] = col;
        }

        // --- Rebase layout sections (fixes missing colors) --------------------
        let row_sections: Vec<egui::text::LayoutSection> = job
            .sections
            .iter()
            .filter_map(|sec| {
                if sec.byte_range.end <= *row_start || sec.byte_range.start >= *row_end {
                    return None;
                }
                let start = sec.byte_range.start.saturating_sub(*row_start);
                let end = sec.byte_range.end.saturating_sub(*row_start);
                Some(egui::text::LayoutSection {
                    leading_space: 0.0,
                    byte_range: start..end.min(row_text.len()),
                    format: sec.format.clone(),
                })
            })
            .collect();

        // --- Build background rectangles --------------------------------------
        let mut bg_runs: Vec<(usize, usize, egui::Color32)> = Vec::new();
        for sec in &row_sections {
            let color = sec.format.background;
            if color == egui::Color32::TRANSPARENT {
                continue;
            }
            let s = sec.byte_range.start.min(row_text.len());
            let e = sec.byte_range.end.min(row_text.len());
            if s >= e {
                continue;
            }
            let col_start = byte_to_col[s];
            let col_end = byte_to_col[e];
            let len = col_end.saturating_sub(col_start);

            if let Some(last) = bg_runs.last_mut() {
                if last.0 + last.1 == col_start && last.2 == color {
                    last.1 += len;
                    continue;
                }
            }
            bg_runs.push((col_start, len, color));
        }

        // --- Paint backgrounds ------------------------------------------------
        for (col_start, len, color) in bg_runs {
            let x0 = row_origin.x
                + f32::value_from(col_start)
                    .unwrap_or_default()
                    .mul_add(glyph_width, 0.0);
            let len_f = f32::value_from(len).unwrap_or_default();
            let rect = egui::Rect::from_min_size(
                egui::pos2(x0, row_origin.y),
                egui::vec2(len_f * glyph_width, row_height),
            );
            painter.rect_filled(rect, 0.0, color);
        }

        // --- Cache check ------------------------------------------------------
        let row_hash = hash_row(row_text, &row_sections, row_text.len());
        let cache = &mut row_cache[row_idx];
        if cache.text_hash == row_hash {
            if let Some(gal) = &cache.galley {
                painter.galley(row_origin, gal.clone(), egui::Color32::WHITE);
                continue;
            }
        }

        // --- Draw glyphs manually (no egui spacing) ---------------------------
        // --- Draw text per-foreground run (fast, no kerning bleed) -------------------
        let font_id = egui::FontId::monospace(font_size);

        // Render each section's substring at the exact cell-aligned x
        for sec in &row_sections {
            let fg = sec.format.color;
            let s = sec.byte_range.start.min(row_text.len());
            let e = sec.byte_range.end.min(row_text.len());
            if s >= e {
                continue;
            }

            // Slice the row's text for this section
            let piece = &row_text[s..e];

            // Column-aligned x based on byte→column map (multi-byte safe)
            let col_start = byte_to_col[s];
            let x0 = row_origin.x
                + f32::value_from(col_start)
                    .unwrap_or_default()
                    .mul_add(glyph_width, 0.0);

            painter.text(
                egui::pos2(x0, row_origin.y),
                egui::Align2::LEFT_TOP,
                piece.to_owned(),
                font_id.clone(),
                fg,
            );
        }

        // --- Build and cache galley for fast path next frames -----------------
        // Construct a row-local LayoutJob mirroring the row_sections
        let mut row_job = eframe::egui::text::LayoutJob {
            wrap: eframe::egui::text::TextWrapping {
                max_width: f32::INFINITY,
                max_rows: 1,
                break_anywhere: false,
                overflow_character: None,
            },
            ..Default::default()
        };
        row_job.append(row_text, 0.0, TextFormat::default());
        row_job.sections.clone_from(&row_sections);

        let galley = ui.ctx().fonts_mut(|f| f.layout_job(row_job));
        // --- Cache new galley + bg runs + hash --------------------------------
        cache.text_hash = row_hash;
        cache.bg_runs = row_sections
            .iter()
            .filter_map(|sec| {
                let color = sec.format.background;
                if color == egui::Color32::TRANSPARENT {
                    return None;
                }
                let s = sec.byte_range.start.min(row_text.len());
                let e = sec.byte_range.end.min(row_text.len());
                if s >= e {
                    return None;
                }
                // compute byte->col map again for start..end
                let mut col = 0usize;
                let mut map = vec![0usize; row_text.len() + 1];
                for (b, _) in row_text.char_indices() {
                    map[b] = col;
                    col += 1;
                }
                map[row_text.len()] = col;
                let col_start = map[s];
                let len = map[e].saturating_sub(col_start);
                Some((col_start, len, color))
            })
            .collect();
        cache.galley = Some(galley);
    }

    response
}

fn add_terminal_data_to_ui(
    ui: &mut Ui,
    data: &UiData,
    font_size: f32,
    render_state: &mut TerminalRenderState,
) -> Result<(egui::Response, Option<UiJobAction>)> {
    let data_utf8: String;
    let adjusted_format_data: Vec<FormatTag>;
    let data_len: usize;

    match data {
        UiData::NewPass(data) => {
            let (data_utf8_new, adjusted_format_data_new) =
                create_terminal_output_layout_job(data.text, &data.format_data)?;
            data_len = data_utf8_new.len();
            data_utf8 = data_utf8_new;
            adjusted_format_data = adjusted_format_data_new;
        }
        UiData::PreviousPass(data) => {
            data_utf8 = data.text.clone();
            adjusted_format_data = data.adjusted_format_data.clone();
            data_len = data_utf8.len();
        }
    }

    let (mut job, mut textformat) = setup_job(ui, &data_utf8);
    process_tags(
        &adjusted_format_data,
        data_len,
        &mut textformat,
        font_size,
        &mut job,
        #[cfg(feature = "validation")]
        data_utf8.as_bytes(),
    );

    match data {
        UiData::NewPass(_) => {
            let response_data = UiJobAction {
                text: data_utf8.clone(),
                adjusted_format_data: adjusted_format_data.clone(),
            };
            // Update render_state text lines
            render_state.lines = data_utf8.lines().map(str::to_string).collect();

            // Mark all dirty for now (until fine-grained tracking is implemented)
            render_state.mark_all_dirty();

            // Use the row-level invalidation renderer
            let response = render_state.render(ui, &job, font_size);

            Ok((response, Some(response_data)))
        }
        UiData::PreviousPass(_) => {
            // Update render_state text lines
            render_state.lines = data_utf8.lines().map(str::to_string).collect();

            // Mark all dirty for now (until fine-grained tracking is implemented)
            render_state.mark_all_dirty();

            // Use the row-level invalidation renderer
            let response = render_state.render(ui, &job, font_size);

            Ok((response, None))
        }
    }
}

#[derive(Clone)]
struct TerminalOutputRenderResponse {
    canvas_area: Rect,
    canvas: UiJobAction,
}

fn render_terminal_output<Io: FreminalTermInputOutput>(
    ui: &mut egui::Ui,
    terminal_emulator: &mut TerminalEmulator<Io>,
    font_size: f32,
    previous_pass: Option<&TerminalOutputRenderResponse>,
    render_state: &mut TerminalRenderState,
) -> TerminalOutputRenderResponse {
    let response = egui::ScrollArea::new([false, true])
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .animated(false)
        .scroll([false, false])
        .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            ui.style_mut().interaction.selectable_labels = false;

            let error_logged_rect =
                |response: Result<(egui::Response, Option<UiJobAction>)>| match response {
                    Ok((v, action)) => (v.rect, action),
                    Err(e) => {
                        error!("failed to add terminal data to ui: {}", e);
                        (Rect::NOTHING, None)
                    }
                };

            let canvas_response: (Rect, Option<UiJobAction>);

            if let Some(previous_pass) = previous_pass {
                _ = error_logged_rect(add_terminal_data_to_ui(
                    ui,
                    &UiData::PreviousPass(previous_pass.canvas.clone()),
                    font_size,
                    render_state,
                ));

                (*previous_pass).clone()
            } else {
                let (terminal_data, format_data) = terminal_emulator.data_and_format_data_for_gui();
                if !terminal_data.scrollback.is_empty() {
                    error!(
                        "Scrollback is not empty: {}",
                        terminal_data.scrollback.len()
                    );
                }

                let mut canvas_data = terminal_data.visible;

                if canvas_data.ends_with(&[TChar::NewLine]) {
                    canvas_data = canvas_data[0..canvas_data.len() - 1].to_vec();
                }
                canvas_response = error_logged_rect(add_terminal_data_to_ui(
                    ui,
                    &UiData::NewPass(&NewJobAction {
                        text: &canvas_data,
                        format_data: format_data.visible,
                    }),
                    font_size,
                    render_state,
                ));

                // We want the program to crash here if we're testing
                #[cfg(feature = "validation")]
                return TerminalOutputRenderResponse {
                    canvas_area: canvas_response.0,
                    #[allow(clippy::unwrap_used)]
                    canvas: canvas_response.1.unwrap(),
                };

                #[cfg(not(any(feature = "validation")))]
                return TerminalOutputRenderResponse {
                    canvas_area: canvas_response.0,
                    canvas: canvas_response.1.unwrap_or_default(),
                };
            }
        });

    response.inner
}

struct DebugRenderer {
    enable: bool,
}

impl DebugRenderer {
    const fn new() -> Self {
        Self { enable: false }
    }

    fn render(&self, ui: &Ui, rect: Rect, color: Color32) {
        if !self.enable {
            return;
        }

        let color = color.gamma_multiply(0.25);
        ui.painter().rect_filled(rect, 0.0, color);
    }
}

pub struct FreminalTerminalWidget {
    font_size: f32,
    character_size: (f32, f32),
    previous_font_size: Option<f32>,
    debug_renderer: DebugRenderer,
    previous_pass: TerminalOutputRenderResponse,
    previous_mouse_state: Option<PreviousMouseState>,
    previous_key: Option<Key>,
    previous_scroll_amount: f32,
    ctx: Context,
    render_state: TerminalRenderState,
}

impl FreminalTerminalWidget {
    #[must_use]
    pub fn new(ctx: &Context) -> Self {
        setup_font_files(ctx);
        setup_bg_fill(ctx);

        Self {
            font_size: 12.0,
            character_size: (0.0, 0.0),
            previous_font_size: None,
            debug_renderer: DebugRenderer::new(),
            previous_pass: TerminalOutputRenderResponse {
                canvas_area: Rect::NOTHING,
                canvas: UiJobAction::default(),
            },
            previous_mouse_state: None,
            previous_key: None,
            previous_scroll_amount: 0.0,
            ctx: ctx.clone(),
            render_state: TerminalRenderState::default(),
        }
    }

    #[must_use]
    pub const fn get_font_size(&self) -> f32 {
        self.font_size
    }

    #[must_use]
    pub fn calculate_available_size(&self, ui: &Ui) -> (usize, usize) {
        let character_size = get_char_size(ui.ctx(), self.font_size);
        let width_chars =
            match ((ui.available_width() / character_size.0).floor()).approx_as::<usize>() {
                Ok(v) => v,
                Err(e) => {
                    error!("Failed to calculate width chars: {}", e);
                    10
                }
            };

        let height_chars =
            match ((ui.available_height() / character_size.1).floor()).approx_as::<usize>() {
                Ok(v) => {
                    if v > 1 {
                        v - 1
                    } else {
                        1
                    }
                }
                Err(e) => {
                    error!("Failed to calculate height chars: {}", e);
                    10
                }
            };

        (width_chars, height_chars)
    }

    #[allow(clippy::too_many_lines)]
    pub fn show<Io: FreminalTermInputOutput>(
        &mut self,
        ui: &mut Ui,
        terminal_emulator: &mut TerminalEmulator<Io>,
    ) {
        let frame_response = egui::Frame::new().show(ui, |ui| {
            // if the previous font size is None, or the font size has changed, we need to update the font size
            if self.previous_font_size.is_none()
                || (self.previous_font_size.unwrap_or_default() - self.font_size).abs()
                    > f32::EPSILON
            {
                debug!("Font size changed, updating character size");
                self.character_size = get_char_size(ui.ctx(), self.font_size);
                terminal_emulator.set_egui_ctx_if_missing(self.ctx.clone());

                let (width_chars, height_chars) = terminal_emulator.get_win_size();
                let width_chars = match f32::value_from(width_chars) {
                    Ok(v) => v,
                    Err(e) => {
                        error!("Failed to convert width chars to f32: {}", e);
                        10.0
                    }
                };

                let height_chars = match f32::value_from(height_chars) {
                    Ok(v) => v,
                    Err(e) => {
                        error!("Failed to convert height chars to f32: {}", e);
                        10.0
                    }
                };

                ui.set_width((width_chars + 0.5) * self.character_size.0);
                ui.set_height((height_chars + 0.5) * self.character_size.1);
                self.previous_font_size = Some(self.font_size);
            }

            let repeat_characters = terminal_emulator.internal.should_repeat_keys();
            let (left_mouse_button_pressed, new_mouse_pos, previous_key, scroll_amount) =
                ui.input(|input_state| {
                    write_input_to_terminal(
                        input_state,
                        terminal_emulator,
                        self.character_size.0,
                        self.character_size.1,
                        self.previous_mouse_state.clone(),
                        repeat_characters,
                        self.previous_key,
                        self.previous_scroll_amount,
                    )
                });
            self.previous_mouse_state = new_mouse_pos;
            self.previous_key = previous_key;
            self.previous_scroll_amount = scroll_amount;

            if terminal_emulator.needs_redraw() {
                self.previous_pass = render_terminal_output(
                    ui,
                    terminal_emulator,
                    self.font_size,
                    None,
                    &mut self.render_state,
                );
            } else {
                debug!("Reusing previous terminal output");
                let _response = render_terminal_output(
                    ui,
                    terminal_emulator,
                    self.font_size,
                    Some(&self.previous_pass),
                    &mut self.render_state,
                );
            }

            #[cfg(debug_assertions)]
            self.debug_renderer
                .render(ui, self.previous_pass.canvas_area, Color32::BLUE);

            if terminal_emulator.show_cursor() {
                let color = internal_color_to_egui(
                    terminal_emulator.internal.get_current_buffer().cursor_color,
                    false,
                );
                paint_cursor(
                    self.previous_pass.canvas_area,
                    self.character_size,
                    &terminal_emulator.cursor_pos(),
                    ui,
                    color,
                );
            }

            // lets see if we're hovering over a URL
            if let Some(mouse_position) = terminal_emulator.get_mouse_position() {
                // convert the mouse position x and y to character positions
                let mut x = ((mouse_position.x / self.character_size.0).floor())
                    .approx_as::<usize>()
                    .unwrap_or_default();
                let mut y = ((mouse_position.y / self.character_size.1).floor())
                    .approx_as::<usize>()
                    .unwrap_or_default();

                x = x.saturating_sub(1);
                y = y.saturating_sub(1);

                let cursor_pos = CursorPos { x, y };

                if let Some(url) = terminal_emulator.is_mouse_hovered_on_url(&cursor_pos) {
                    debug!("Mouse is hovering over a URL");
                    if left_mouse_button_pressed {
                        ui.ctx().output_mut(|output| {
                            output.cursor_icon = CursorIcon::Wait;
                            output.commands.push(OutputCommand::OpenUrl(OpenUrl {
                                url: url.clone(),
                                new_tab: true,
                            }));
                        });
                    } else {
                        ui.ctx().output_mut(|output| {
                            output.cursor_icon = CursorIcon::PointingHand;
                        });
                    }
                }
            } else {
                debug!("No mouse position");

                ui.ctx().output_mut(|output| {
                    output.cursor_icon = CursorIcon::Default;
                });
            }
        });

        terminal_emulator.set_previous_pass_valid();

        self.debug_renderer
            .render(ui, frame_response.response.rect, Color32::RED);
    }

    pub fn show_options(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Font size:");
            ui.add(DragValue::new(&mut self.font_size).range(1.0..=100.0));
        });
        #[cfg(debug_assertions)]
        ui.checkbox(&mut self.debug_renderer.enable, "Debug render");
    }
}
