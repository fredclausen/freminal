// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::gui::TerminalEmulator;

use freminal_terminal_emulator::{
    ansi_components::{mode::MouseTrack, modes::rl_bracket::RlBracket},
    format_tracker::FormatTag,
    interface::TerminalInput,
    io::FreminalTermInputOutput,
    state::{cursor::CursorPos, fonts::FontDecorations, term_char::TChar},
};

use eframe::egui::{
    self, scroll_area::ScrollBarVisibility, text::LayoutJob, Color32, Context, CursorIcon,
    DragValue, Event, InputState, Key, Modifiers, OpenUrl, PointerButton, Rect, Stroke, TextFormat,
    TextStyle, Ui, Vec2,
};

use super::{
    colors::internal_color_to_egui,
    fonts::{get_char_size, setup_font_files, TerminalFont},
};
use anyhow::Result;
use conv::{ConvUtil, ValueFrom};
use std::borrow::Cow;

fn collect_text(text: &String) -> Cow<'static, [TerminalInput]> {
    text.as_bytes()
        .iter()
        .map(|c| TerminalInput::Ascii(*c))
        .collect::<Vec<_>>()
        .into()
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

#[derive(Debug, PartialEq, Clone)]
struct PreviousMouseState {
    button: PointerButton,
    button_pressed: bool,
    mouse_position: Option<FreminalMousePosition>,
    modifiers: Modifiers,
}

impl Default for PreviousMouseState {
    fn default() -> Self {
        Self {
            button: PointerButton::Primary,
            button_pressed: false,
            mouse_position: None,
            modifiers: Modifiers::default(),
        }
    }
}

impl PreviousMouseState {
    pub fn should_report(&self, new: Option<&Self>) -> bool {
        if let Some(new) = new {
            return self.mouse_position != new.mouse_position;
        }
        false
    }
}

enum MouseEvent {
    Button(PointerButton),
    Scroll(Vec2),
}

#[derive(Debug, PartialEq, Clone)]
enum MouseEncoding {
    X11,
    Sgr,
}

#[derive(Debug, PartialEq, Clone)]
struct FreminalMousePosition {
    x_as_character_column: usize,
    y_as_character_row: usize,
}

#[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
fn write_input_to_terminal<Io: FreminalTermInputOutput>(
    input: &InputState,
    terminal_emulator: &mut TerminalEmulator<Io>,
    character_size_x: f32,
    character_size_y: f32,
    last_reported_mouse_pos: Option<PreviousMouseState>,
) -> (bool, Option<PreviousMouseState>) {
    if input.raw.events.is_empty() {
        return (false, last_reported_mouse_pos);
    }

    let mut state_changed = false;
    let mut last_reported_mouse_pos = last_reported_mouse_pos;
    let mut left_mouse_button_pressed = false;

    for event in &input.raw.events {
        debug!("event: {:?}", event);
        let inputs: Cow<'static, [TerminalInput]> = match event {
            // FIXME: We don't support separating out numpad vs regular keys
            // This is an egui issue. See: https://github.com/emilk/egui/issues/3653
            Event::Text(text) => collect_text(text),
            Event::Key {
                key: Key::Enter,
                pressed: true,
                ..
            } => [TerminalInput::Enter].as_ref().into(),
            // https://github.com/emilk/egui/issues/3653
            // FIXME: Technically not correct if we were on a mac, but also we are using linux
            // syscalls so we'd have to solve that before this is a problem
            Event::Copy => [TerminalInput::Ctrl(b'c')].as_ref().into(),
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
                continue;
            }
            Event::MouseMoved(pos) => {
                info!("Mouse moved: {:?}", pos);
                continue;
            }
            Event::PointerMoved(pos) => {
                info!("Pointer moved: {:?}", pos);
                terminal_emulator.set_mouse_position_from_move_event(pos);
                if terminal_emulator
                    .internal
                    .modes
                    .mouse_tracking
                    .should_report_motion()
                {
                    let previous_mouse_state = last_reported_mouse_pos.clone().unwrap_or_default();

                    let x = ((pos.x / character_size_x).floor())
                        .approx_as::<usize>()
                        .unwrap_or_else(|_| {
                            error!("Failed to convert {} to u8. Using default of 0", pos.x);
                            0
                        });
                    let y = ((pos.y / character_size_y).floor())
                        .approx_as::<usize>()
                        .unwrap_or_else(|_| {
                            error!("Failed to convert {} to u8. Using default of 0", pos.y);
                            0
                        });
                    let mouse_pos = FreminalMousePosition {
                        x_as_character_column: x,
                        y_as_character_row: y,
                    };
                    let new_mouse_position = PreviousMouseState {
                        button: previous_mouse_state.button,
                        button_pressed: previous_mouse_state.button_pressed,
                        mouse_position: Some(mouse_pos.clone()),
                        modifiers: previous_mouse_state.modifiers,
                    };

                    let report_motion =
                        new_mouse_position.should_report(last_reported_mouse_pos.as_ref());

                    if last_reported_mouse_pos.is_none() {
                        last_reported_mouse_pos = Some(new_mouse_position.clone());
                    }

                    if report_motion {
                        info!("Reporting mouse motion");
                        let encoding = if terminal_emulator.internal.modes.mouse_tracking
                            == MouseTrack::XtMseSgr
                        {
                            MouseEncoding::Sgr
                        } else {
                            MouseEncoding::X11
                        };

                        encode_x11_mouse_button(
                            new_mouse_position.button,
                            true,
                            new_mouse_position.modifiers,
                            &mouse_pos,
                            false,
                            &encoding,
                        )
                    } else {
                        continue;
                    }
                } else {
                    continue;
                };

                continue;
            }
            Event::PointerButton {
                button,
                pressed,
                modifiers,
                pos,
            } => {
                state_changed = true;
                let x = ((pos.x / character_size_x).floor())
                    .approx_as::<usize>()
                    .unwrap_or_else(|_| {
                        error!("Failed to convert {} to u8. Using default of 0", pos.x);
                        0
                    });
                let y = ((pos.y / character_size_y).floor())
                    .approx_as::<usize>()
                    .unwrap_or_else(|_| {
                        error!("Failed to convert {} to u8. Using default of 0", pos.y);
                        0
                    });
                let mouse_pos = FreminalMousePosition {
                    x_as_character_column: x,
                    y_as_character_row: y,
                };
                let new_mouse_position = PreviousMouseState {
                    button: *button,
                    button_pressed: *pressed,
                    mouse_position: Some(mouse_pos.clone()),
                    modifiers: *modifiers,
                };

                if *button == PointerButton::Primary && *pressed {
                    left_mouse_button_pressed = true;
                }

                if terminal_emulator.internal.modes.mouse_tracking == MouseTrack::NoTracking {
                    continue;
                }

                // TODO: We should probably also set the mouse position here
                //terminal_emulator.set_mouse_position(&Some(pos));
                if *pressed
                    || terminal_emulator
                        .internal
                        .modes
                        .mouse_tracking
                        .should_scroll()
                {
                    last_reported_mouse_pos = Some(new_mouse_position);
                } else {
                    last_reported_mouse_pos = None;
                }

                let encoding =
                    if terminal_emulator.internal.modes.mouse_tracking == MouseTrack::XtMseSgr {
                        MouseEncoding::Sgr
                    } else {
                        MouseEncoding::X11
                    };

                encode_x11_mouse_button(*button, *pressed, *modifiers, &mouse_pos, false, &encoding)
            }
            Event::MouseWheel {
                delta, modifiers, ..
            } => {
                // TODO: should we care if we scrolled in the x axis?
                if delta.y != 0.0 {
                    terminal_emulator.internal.scroll(delta.y);
                }

                state_changed = true;

                if terminal_emulator.internal.modes.mouse_tracking == MouseTrack::NoTracking
                    || last_reported_mouse_pos.is_none()
                {
                    continue;
                }

                let new_mouse_position = last_reported_mouse_pos.clone().unwrap();
                let encoding =
                    if terminal_emulator.internal.modes.mouse_tracking == MouseTrack::XtMseSgr {
                        MouseEncoding::Sgr
                    } else {
                        MouseEncoding::X11
                    };

                if let Some(response) = encode_x11_mouse_wheel(
                    *delta,
                    *modifiers,
                    &new_mouse_position.mouse_position.unwrap(),
                    &encoding,
                ) {
                    response
                } else {
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

    (left_mouse_button_pressed, last_reported_mouse_pos)
}

fn encode_mouse_for_x11(button: &MouseEvent, pressed: bool) -> usize {
    if pressed {
        match button {
            MouseEvent::Button(PointerButton::Primary) => 0,
            MouseEvent::Button(PointerButton::Middle) => 1,
            MouseEvent::Button(PointerButton::Secondary) => 2,
            MouseEvent::Button(_) => {
                error!("Unsupported mouse button. Treating as left mouse button");
                0
            }
            MouseEvent::Scroll(amount) => {
                // FIXME: This is not correct. eframe encodes a x and y event together I think.
                // For now we'll prefer the y event as the driver for the scroll
                // If that is the case should we be sending a two different events for scroll?

                // if amount.y != 0.0 {
                //     info!("scrolling y: {}", amount.y);
                //     if amount.y > 0.0 {
                //         return 66;
                //     }
                //     return 67;
                // };

                if amount.y != 0.0 {
                    if amount.y > 0.0 {
                        return 64;
                    }
                    return 65;
                }

                0
            }
        }
    } else {
        3
    }
}

const fn encode_modifiers_for_x11(modifiers: Modifiers) -> usize {
    let mut cb = 0;

    if modifiers.ctrl || modifiers.command {
        cb += 16;
    }

    if modifiers.shift {
        cb += 4;
    }

    // This is for meta, but wezterm seems to use alt as the meta?
    if modifiers.alt {
        cb += 8;
    }

    cb
}

fn encode_x11_mouse_wheel(
    delta: Vec2,
    modifiers: Modifiers,
    pos: &FreminalMousePosition,
    encoding: &MouseEncoding,
) -> Option<Cow<'static, [TerminalInput]>> {
    info!("Scrolling with position: {:?}, delta: {:?}", pos, delta);
    let padding = if encoding == &MouseEncoding::X11 {
        32
    } else {
        0
    };

    let mut cb = padding;

    cb += encode_mouse_for_x11(&MouseEvent::Scroll(delta), true);
    if cb == 32 {
        return None;
    }
    cb += encode_modifiers_for_x11(modifiers);

    let x = pos.x_as_character_column + padding;
    let y = pos.y_as_character_row + padding;

    if encoding == &MouseEncoding::X11 {
        let cb = cb.approx_as::<u8>().unwrap_or_else(|_| {
            error!("Failed to convert {} to char. Using default of 256", cb);
            255
        });
        let x = x.approx_as::<u8>().unwrap_or_else(|_| {
            error!("Failed to convert {} to char. Using default of 256", x);
            255
        });
        let y = y.approx_as::<u8>().unwrap_or_else(|_| {
            error!("Failed to convert {} to char. Using default of 256", y);
            255
        });
        Some(collect_text(&format!(
            "\x1b[M{}{}{}",
            cb as char, x as char, y as char
        )))
    } else {
        Some(collect_text(&format!("\x1b[<{cb};{x};{y}M")))
    }
}

fn encode_x11_mouse_button(
    button: PointerButton,
    pressed: bool,
    modifiers: Modifiers,
    pos: &FreminalMousePosition,
    report_motion: bool,
    encoding: &MouseEncoding,
) -> Cow<'static, [TerminalInput]> {
    //Normal tracking mode sends an escape sequence on both button press and release. Modifier key (shift, ctrl, meta) information is also sent. It is enabled by specifying parameter 1000 to DECSET. On button press or release, xterm sends CSI M C b C x C y . The low two bits of C b encode button information: 0=MB1 pressed, 1=MB2 pressed, 2=MB3 pressed, 3=release. The next three bits encode the modifiers which were down when the button was pressed and are added together: 4=Shift, 8=Meta, 16=Control

    let padding = if encoding == &MouseEncoding::X11 {
        32
    } else {
        0
    };

    let motion = if report_motion { 32 } else { 0 };
    let mut cb: usize = padding;

    cb += encode_mouse_for_x11(&MouseEvent::Button(button), pressed);
    cb += encode_modifiers_for_x11(modifiers);

    let x = pos.x_as_character_column + padding + motion;
    let y = pos.y_as_character_row + padding + motion;

    if encoding == &MouseEncoding::X11 {
        let cb = cb.approx_as::<u8>().unwrap_or_else(|_| {
            error!("Failed to convert {} to char. Using default of 256", cb);
            255
        });
        let x = x.approx_as::<u8>().unwrap_or_else(|_| {
            error!("Failed to convert {} to char. Using default of 256", x);
            255
        });
        let y = y.approx_as::<u8>().unwrap_or_else(|_| {
            error!("Failed to convert {} to char. Using default of 256", y);
            255
        });

        collect_text(&format!("\x1b[M{}{}{}", cb as char, x as char, y as char))
    } else {
        collect_text(&format!(
            "\x1b[<{};{};{}{}",
            cb,
            x,
            y,
            if pressed { "M" } else { "m" }
        ))
    }
}

fn paint_cursor(label_rect: Rect, character_size: (f32, f32), cursor_pos: &CursorPos, ui: &Ui) {
    let painter = ui.painter();

    let top = label_rect.top();
    let left = label_rect.left();
    let y_offset: f32 = f32::value_from(cursor_pos.y).unwrap() * character_size.1;
    let x_offset = f32::value_from(cursor_pos.x).unwrap() * character_size.0;
    painter.rect_filled(
        Rect::from_min_size(
            egui::pos2(left + x_offset, top + y_offset),
            egui::vec2(character_size.0, character_size.1),
        ),
        0.0,
        Color32::GRAY,
    );
}

fn setup_bg_fill(ctx: &egui::Context) {
    ctx.style_mut(|style| {
        style.visuals.window_fill = egui::Color32::BLACK;
        style.visuals.panel_fill = egui::Color32::BLACK;
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
            line_wrap_mode: tag.line_wrap_mode.clone(),
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
    let default_color = textformat.color;
    let default_background = textformat.background;
    let terminal_fonts = TerminalFont::new();

    for tag in adjusted_format_data {
        let mut range = tag.start..tag.end;
        let color = tag.colors.get_color();
        let background_color = tag.colors.get_background_color();
        let underline_color = tag.colors.get_underline_color();

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
        textformat.color =
            internal_color_to_egui(default_color, default_background, color, make_faint);
        // FIXME: ????? should background be faint? I feel like no, but....
        textformat.background = internal_color_to_egui(
            default_background,
            default_background,
            background_color,
            make_faint,
        );
        if tag.font_decorations.contains(&FontDecorations::Underline) {
            let underline_color_converted = internal_color_to_egui(
                textformat.color,
                default_background,
                underline_color,
                make_faint,
            );

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

fn add_terminal_data_to_ui(
    ui: &mut Ui,
    data: &UiData,
    font_size: f32,
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
            let response = UiJobAction {
                text: data_utf8,
                adjusted_format_data,
            };
            Ok((ui.label(job), Some(response)))
        }
        UiData::PreviousPass(_) => Ok((ui.label(job), None)),
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
                ));

                // We want the program to crash here if we're testing
                #[cfg(feature = "validation")]
                return TerminalOutputRenderResponse {
                    canvas_area: canvas_response.0,
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
    ctx: Context,
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
            ctx: ctx.clone(),
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
        let frame_response = egui::Frame::none().show(ui, |ui| {
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

            if let Some(title) = terminal_emulator.get_window_title() {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Title(title));
                terminal_emulator.clear_window_title();
            }

            let (left_mouse_button_pressed, new_mouse_pos) = ui.input(|input_state| {
                write_input_to_terminal(
                    input_state,
                    terminal_emulator,
                    self.character_size.0,
                    self.character_size.1,
                    self.previous_mouse_state.clone(),
                )
            });
            self.previous_mouse_state = new_mouse_pos;

            if terminal_emulator.needs_redraw() {
                self.previous_pass =
                    render_terminal_output(ui, terminal_emulator, self.font_size, None);
            } else {
                debug!("Reusing previous terminal output");
                let _response = render_terminal_output(
                    ui,
                    terminal_emulator,
                    self.font_size,
                    Some(&self.previous_pass),
                );
            }

            #[cfg(debug_assertions)]
            self.debug_renderer
                .render(ui, self.previous_pass.canvas_area, Color32::BLUE);

            if terminal_emulator.show_cursor() {
                paint_cursor(
                    self.previous_pass.canvas_area,
                    self.character_size,
                    &terminal_emulator.cursor_pos(),
                    ui,
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
                            output.open_url = Some(OpenUrl {
                                url: url.to_string(),
                                new_tab: true,
                            });
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
