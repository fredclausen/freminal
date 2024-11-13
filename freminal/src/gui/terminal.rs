// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::gui::TerminalEmulator;
use crate::terminal_emulator::{
    format_tracker::FormatTag,
    interface::TerminalInput,
    io::FreminalTermInputOutput,
    state::{cursor::CursorPos, fonts::FontDecorations, term_char::TChar},
};

use eframe::egui::{
    self, scroll_area::ScrollBarVisibility, text::LayoutJob, Color32, Context, DragValue, Event,
    InputState, Key, Modifiers, Rect, Stroke, TextFormat, TextStyle, Ui,
};

use conv::{ConvUtil, ValueFrom};
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
    terminal_emulator: &mut TerminalEmulator<Io>,
) {
    if input.raw.events.is_empty() {
        return;
    }

    terminal_emulator.set_previous_pass_invalid();

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
    data: &[TChar],
    format_data: &[FormatTag],
) -> Result<(String, Vec<FormatTag>), std::str::Utf8Error> {
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
            return Err(e);
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
            data_converted.len() - 1
        };

        let end = if tag.start == tag.end {
            start
        } else if tag.end == usize::MAX {
            data_converted.len() - 1
        } else if tag.end >= offset.len() {
            offset.last().unwrap().to_owned()
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
            color: tag.color,
            background_color: tag.background_color,
            underline_color: tag.underline_color,
            font_weight: tag.font_weight.clone(),
            font_decorations: tag.font_decorations.clone(),
            line_wrap_mode: tag.line_wrap_mode.clone(),
        });
    }

    Ok((data_utf8.to_string(), format_data_shifted))
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
) {
    let default_color = textformat.color;
    let default_background = textformat.background;
    let terminal_fonts = TerminalFont::new();

    for tag in adjusted_format_data {
        let mut range = tag.start..tag.end;
        let color = tag.color;
        let background_color = tag.background_color;
        let underline_color = tag.underline_color;

        if range.end == usize::MAX {
            range.end = data_len;
        }

        match range.start.cmp(&data_len) {
            std::cmp::Ordering::Greater => {
                debug!("Skipping unusable format data");
                continue;
            }
            std::cmp::Ordering::Equal => {
                continue;
            }
            std::cmp::Ordering::Less => (),
        }

        if range.end > data_len {
            debug!("Truncating format data end");
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
) -> Result<(egui::Response, Option<UiJobAction>), std::str::Utf8Error> {
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
    // let (data_utf8, adjusted_format_data) =
    //     create_terminal_output_layout_job(data, format_data)?;

    let (mut job, mut textformat) = setup_job(ui, &data_utf8);
    process_tags(
        &adjusted_format_data,
        data_len,
        &mut textformat,
        font_size,
        &mut job,
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
    scrollback_area: Rect,
    canvas_area: Rect,
    scrollback: UiJobAction,
    canvas: UiJobAction,
}

fn render_terminal_output<Io: FreminalTermInputOutput>(
    ui: &mut egui::Ui,
    terminal_emulator: &TerminalEmulator<Io>,
    font_size: f32,
    previous_pass: Option<&TerminalOutputRenderResponse>,
) -> TerminalOutputRenderResponse {
    let response = egui::ScrollArea::new([false, true])
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .animated(false)
        .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            let error_logged_rect = |response: Result<
                (egui::Response, Option<UiJobAction>),
                std::str::Utf8Error,
            >| match response {
                Ok((v, action)) => (v.rect, action),
                Err(e) => {
                    error!("failed to add terminal data to ui: {}", e);
                    (Rect::NOTHING, None)
                }
            };

            let scrollback_response: (Rect, Option<UiJobAction>);
            let canvas_response: (Rect, Option<UiJobAction>);

            if let Some(previous_pass) = previous_pass {
                _ = error_logged_rect(add_terminal_data_to_ui(
                    ui,
                    &UiData::PreviousPass(previous_pass.scrollback.clone()),
                    font_size,
                ));
                _ = error_logged_rect(add_terminal_data_to_ui(
                    ui,
                    &UiData::PreviousPass(previous_pass.canvas.clone()),
                    font_size,
                ));

                (*previous_pass).clone()
            } else {
                let terminal_data = terminal_emulator.data();
                let scrollback_data = terminal_data.scrollback;
                let mut canvas_data = terminal_data.visible;
                let format_data = terminal_emulator.format_data();

                if canvas_data.ends_with(&[TChar::NewLine]) {
                    canvas_data = canvas_data[0..canvas_data.len() - 1].to_vec();
                }
                scrollback_response = error_logged_rect(add_terminal_data_to_ui(
                    ui,
                    &UiData::NewPass(&NewJobAction {
                        text: &scrollback_data,
                        format_data: format_data.scrollback.clone(),
                    }),
                    font_size,
                ));
                canvas_response = error_logged_rect(add_terminal_data_to_ui(
                    ui,
                    &UiData::NewPass(&NewJobAction {
                        text: &canvas_data,
                        format_data: format_data.visible,
                    }),
                    font_size,
                ));

                TerminalOutputRenderResponse {
                    scrollback_area: scrollback_response.0,
                    canvas_area: canvas_response.0,
                    scrollback: scrollback_response.1.unwrap(),
                    canvas: canvas_response.1.unwrap(),
                }
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
    previous_pass: TerminalOutputRenderResponse,
    ctx: Context,
}

impl FreminalTerminalWidget {
    #[must_use]
    pub fn new(ctx: &Context) -> Self {
        setup_font_files(ctx);
        setup_bg_fill(ctx);

        Self {
            font_size: 12.0,
            debug_renderer: DebugRenderer::new(),
            previous_pass: TerminalOutputRenderResponse {
                scrollback_area: Rect::NOTHING,
                canvas_area: Rect::NOTHING,
                scrollback: UiJobAction::default(),
                canvas: UiJobAction::default(),
            },
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
                Ok(v) => v,
                Err(e) => {
                    error!("Failed to calculate height chars: {}", e);
                    10
                }
            };

        (width_chars, height_chars)
    }

    pub fn show<Io: FreminalTermInputOutput>(
        &mut self,
        ui: &mut Ui,
        terminal_emulator: &mut TerminalEmulator<Io>,
    ) {
        let character_size = get_char_size(ui.ctx(), self.font_size);
        terminal_emulator.set_egui_ctx_if_missing(self.ctx.clone());

        let frame_response = egui::Frame::none().show(ui, |ui| {
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

            ui.set_width((width_chars + 0.5) * character_size.0);
            ui.set_height((height_chars + 0.5) * character_size.1);

            if let Some(title) = terminal_emulator.get_window_title() {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Title(title));
                terminal_emulator.clear_window_title();
            }

            ui.input(|input_state| {
                write_input_to_terminal(input_state, terminal_emulator);
            });

            if terminal_emulator.needs_redraw() {
                debug!("Redrawing terminal output");
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

            self.debug_renderer
                .render(ui, self.previous_pass.canvas_area, Color32::BLUE);

            self.debug_renderer
                .render(ui, self.previous_pass.scrollback_area, Color32::YELLOW);

            paint_cursor(
                self.previous_pass.canvas_area,
                character_size,
                &terminal_emulator.cursor_pos(),
                ui,
            );
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
        ui.checkbox(&mut self.debug_renderer.enable, "Debug render");
    }
}