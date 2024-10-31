// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::terminal_emulator::{
    term_char::TChar, CursorPos, FontDecorations, FormatTag, FreminalTermInputOutput,
    TerminalEmulator, TerminalInput,
};
use eframe::egui::{
    self, text::LayoutJob, Color32, Context, DragValue, Event, InputState, Key, Modifiers, Rect,
    Stroke, TextFormat, TextStyle, Ui,
};

use conv::{ConvAsUtil, ValueFrom};
use std::borrow::Cow;

use super::{
    colors::internal_color_to_egui,
    fonts::{get_char_size, setup_font_files, TerminalFont},
};

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

fn write_input_to_terminal<Io: FreminalTermInputOutput>(
    input: &InputState,
    terminal_emulator: &TerminalEmulator<Io>,
) {
    for event in &input.raw.events {
        let inputs: Cow<'static, [TerminalInput]> = match event {
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
                    info!("Unexpected ctrl key: {}", key.name());
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
            _ => {
                continue;
            }
        };

        for input in inputs.as_ref() {
            if let Err(e) = terminal_emulator.write(input) {
                error!("Failed to write input to terminal emulator: {}", e);
            }
        }
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
    style: &egui::Style,
    width: f32,
    data: &[TChar],
    format_data: &[FormatTag],
) -> Result<(LayoutJob, TextFormat, Vec<FormatTag>), std::str::Utf8Error> {
    let text_style = &style.text_styles[&TextStyle::Monospace];
    // convert data in to an array of bytes
    let mut data_converted = vec![];

    for c in data {
        match c {
            TChar::NewLine => data_converted.push(b'\n'),
            TChar::Space => data_converted.push(b' '),
            TChar::Ascii(c) => data_converted.push(*c),
            TChar::Utf8(all) => data_converted.extend_from_slice(all),
        }
    }

    let data_utf8 = match std::str::from_utf8(&data_converted) {
        Ok(v) => v,
        Err(e) => {
            error!(
                "Create output job: Failed to convert terminal data to utf8: {}",
                e
            );
            return Err(e);
        }
    };

    // we need to map the format data to the utf8 data
    // We need to shift the format data for the number of added bytes (uft8) for any Tchar found in the input data

    let mut format_data_shifted = vec![];

    for tag in format_data {
        // for each tag go through and find all of the utf8 characters that are before the tag
        // and in the tag range
        // and shift start by the number of utf8 characters before the tag
        // and the end by the number of utf8 characters in the tag range + the number of utf8 characters before the tag

        let mut new_start = 0;
        let mut new_end = 0;

        for (i, c) in data.iter().enumerate() {
            if i >= tag.start {
                break;
            }

            new_start += match c {
                TChar::NewLine | TChar::Space | TChar::Ascii(_) => 1,
                TChar::Utf8(v) => v.len(),
            }
        }

        for (i, c) in data.iter().enumerate() {
            if i >= tag.end || tag.end == usize::MAX {
                break;
            }

            new_end += match c {
                TChar::NewLine | TChar::Space | TChar::Ascii(_) => 1,
                TChar::Utf8(v) => v.len(),
            }
        }

        format_data_shifted.push(FormatTag {
            start: new_start,
            end: new_end,
            color: tag.color,
            background_color: tag.background_color,
            underline_color: tag.underline_color,
            font_weight: tag.font_weight.clone(),
            font_decorations: tag.font_decorations.clone(),
            line_wrap_mode: tag.line_wrap_mode.clone(),
        });
    }

    let mut job = egui::text::LayoutJob::simple(
        data_utf8.to_string(),
        text_style.clone(),
        style.visuals.text_color(),
        width,
    );

    job.wrap.break_anywhere = true;
    let textformat = job.sections[0].format.clone();
    job.sections.clear();
    Ok((job, textformat, format_data_shifted))
}

fn add_terminal_data_to_ui(
    ui: &mut Ui,
    data: &[TChar],
    format_data: &[FormatTag],
    font_size: f32,
) -> Result<egui::Response, std::str::Utf8Error> {
    let (mut job, mut textformat, adjusted_format_data) =
        create_terminal_output_layout_job(ui.style(), ui.available_width(), data, format_data)?;

    let default_color = textformat.color;
    let default_background = textformat.background;
    let terminal_fonts = TerminalFont::new();

    for tag in adjusted_format_data {
        let mut range = tag.start..tag.end;
        let color = tag.color;
        let background_color = tag.background_color;
        let underline_color = tag.underline_color;

        if range.end == usize::MAX {
            range.end = data.len();
        }

        match range.start.cmp(&data.len()) {
            std::cmp::Ordering::Greater => {
                debug!("Skipping unusable format data");
                continue;
            }
            std::cmp::Ordering::Equal => {
                continue;
            }
            std::cmp::Ordering::Less => (),
        }

        if range.end > data.len() {
            debug!("Truncating format data end");
            range.end = data.len();
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

        job.sections.push(egui::text::LayoutSection {
            leading_space: 0.0f32,
            byte_range: range,
            format: textformat.clone(),
        });
    }

    Ok(ui.label(job))
}

struct TerminalOutputRenderResponse {
    scrollback_area: Rect,
    canvas_area: Rect,
}

fn render_terminal_output<Io: FreminalTermInputOutput>(
    ui: &mut egui::Ui,
    terminal_emulator: &TerminalEmulator<Io>,
    font_size: f32,
) -> TerminalOutputRenderResponse {
    let terminal_data = terminal_emulator.data();
    let mut scrollback_data = terminal_data.scrollback;
    let mut canvas_data = terminal_data.visible;
    let mut format_data = terminal_emulator.format_data();

    // Arguably incorrect. Scrollback does end with a newline, and that newline causes a blank
    // space between widgets. Should we strip it here, or in the terminal emulator output?
    if scrollback_data.ends_with(&[TChar::NewLine]) {
        scrollback_data = &scrollback_data[0..scrollback_data.len() - 1];
        if let Some(last_tag) = format_data.scrollback.last_mut() {
            last_tag.end = last_tag.end.min(scrollback_data.len());
        }
    }

    if canvas_data.ends_with(&[TChar::NewLine]) {
        canvas_data = &canvas_data[0..canvas_data.len() - 1];
    }

    let response = egui::ScrollArea::new([false, true])
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(ui, |ui| {
            let error_logged_rect =
                |response: Result<egui::Response, std::str::Utf8Error>| match response {
                    Ok(v) => v.rect,
                    Err(e) => {
                        error!("failed to add terminal data to ui: {}", e);
                        Rect::NOTHING
                    }
                };
            let scrollback_area = error_logged_rect(add_terminal_data_to_ui(
                ui,
                scrollback_data,
                &format_data.scrollback,
                font_size,
            ));
            let canvas_area = error_logged_rect(add_terminal_data_to_ui(
                ui,
                canvas_data,
                &format_data.visible,
                font_size,
            ));
            TerminalOutputRenderResponse {
                scrollback_area,
                canvas_area,
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
    debug_renderer: DebugRenderer,
}

impl FreminalTerminalWidget {
    pub fn new(ctx: &Context) -> Self {
        setup_font_files(ctx);
        setup_bg_fill(ctx);

        Self {
            font_size: 12.0,
            debug_renderer: DebugRenderer::new(),
        }
    }

    pub const fn get_font_size(&self) -> f32 {
        self.font_size
    }

    pub fn calculate_available_size(&self, ui: &Ui) -> (usize, usize) {
        let character_size = get_char_size(ui.ctx(), self.font_size);
        let width_chars = (ui.available_width() / character_size.0)
            .floor()
            .approx()
            .unwrap();
        let height_chars = (ui.available_height() / character_size.1)
            .floor()
            .approx()
            .unwrap();
        (width_chars, height_chars)
    }

    pub fn show<Io: FreminalTermInputOutput>(
        &self,
        ui: &mut Ui,
        terminal_emulator: &mut TerminalEmulator<Io>,
    ) {
        let character_size = get_char_size(ui.ctx(), self.font_size);

        terminal_emulator.read();

        let frame_response = egui::Frame::none().show(ui, |ui| {
            let (width_chars, height_chars) = terminal_emulator.get_win_size();
            let width_chars = f32::value_from(width_chars).unwrap();
            let height_chars = f32::value_from(height_chars).unwrap();

            ui.set_width((width_chars + 0.5) * character_size.0);
            ui.set_height((height_chars + 0.5) * character_size.1);

            if let Some(title) = terminal_emulator.get_window_title() {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Title(title));
            }

            ui.input(|input_state| {
                write_input_to_terminal(input_state, terminal_emulator);
            });

            let output_response = render_terminal_output(ui, terminal_emulator, self.font_size);
            self.debug_renderer
                .render(ui, output_response.canvas_area, Color32::BLUE);

            self.debug_renderer
                .render(ui, output_response.scrollback_area, Color32::YELLOW);

            paint_cursor(
                output_response.canvas_area,
                character_size,
                &terminal_emulator.cursor_pos(),
                ui,
            );
        });

        self.debug_renderer
            .render(ui, frame_response.response.rect, Color32::RED);
    }

    pub fn show_options(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Font size:");
            ui.add(DragValue::new(&mut self.font_size).range(1.0..=100.0));
        });
        ui.checkbox(&mut self.debug_renderer.enable, "Debug render");
    }
}
