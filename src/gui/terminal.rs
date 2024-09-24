// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::error::backtraced_err;
use crate::terminal_emulator::{
    CursorPos, FontDecorations, FontWeight, FormatTag, FreminalTermInputOutput, TerminalColor,
    TerminalEmulator, TerminalInput,
};
use eframe::egui::{
    self, text::LayoutJob, Color32, Context, DragValue, Event, FontData, FontDefinitions,
    FontFamily, FontId, InputState, Key, Modifiers, Rect, TextFormat, TextStyle, Ui,
};

use conv::{ConvAsUtil, ValueFrom};
use std::borrow::Cow;

const REGULAR_FONT_NAME: &str = "hack";
const BOLD_FONT_NAME: &str = "hack-bold";
const ITALIC_FONT_NAME: &str = "hack-italic";
const BOLD_ITALIC_FONT_NAME: &str = "hack-bold-italic";

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
                error!(
                    "Failed to write input to terminal emulator: {}",
                    backtraced_err(&*e)
                );
            }
        }
    }
}

fn get_char_size(ctx: &egui::Context, font_size: f32) -> (f32, f32) {
    let font_id = FontId {
        size: font_size,
        family: FontFamily::Name(REGULAR_FONT_NAME.into()),
    };

    // NOTE: Using glyph width and row height do not give accurate results. Even using the mesh
    // bounds of a single character is not reasonable. Instead we layout 16 rows and 16 cols and
    // divide by 16. This seems to work better at all font scales
    ctx.fonts(move |fonts| {
        let rect = fonts
            .layout(
                "asdfasdfasdfasdf\n\
                 asdfasdfasdfasdf\n\
                 asdfasdfasdfasdf\n\
                 asdfasdfasdfasdf\n\
                 asdfasdfasdfasdf\n\
                 asdfasdfasdfasdf\n\
                 asdfasdfasdfasdf\n\
                 asdfasdfasdfasdf\n\
                 asdfasdfasdfasdf\n\
                 asdfasdfasdfasdf\n\
                 asdfasdfasdfasdf\n\
                 asdfasdfasdfasdf\n\
                 asdfasdfasdfasdf\n\
                 asdfasdfasdfasdf\n\
                 asdfasdfasdfasdf\n\
                 asdfasdfasdfasdf"
                    .to_string(),
                font_id,
                Color32::WHITE,
                f32::INFINITY,
            )
            .rect;

        let width = rect.width() / 16.0;
        let height = rect.height() / 16.0;

        (width, height)
    })
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

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        REGULAR_FONT_NAME.to_owned(),
        FontData::from_static(include_bytes!("../../res/MesloLGSNerdFontMono-Regular.ttf")),
    );

    fonts.font_data.insert(
        BOLD_FONT_NAME.to_owned(),
        FontData::from_static(include_bytes!("../../res/MesloLGSNerdFontMono-Bold.ttf")),
    );

    fonts.font_data.insert(
        ITALIC_FONT_NAME.to_owned(),
        FontData::from_static(include_bytes!("../../res/MesloLGSNerdFontMono-Italic.ttf")),
    );

    fonts.font_data.insert(
        BOLD_ITALIC_FONT_NAME.to_owned(),
        FontData::from_static(include_bytes!(
            "../../res/MesloLGSNerdFontMono-BoldItalic.ttf"
        )),
    );

    fonts
        .families
        .get_mut(&FontFamily::Monospace)
        .expect("egui should provide a monospace font")
        .insert(0, REGULAR_FONT_NAME.to_owned());

    fonts.families.insert(
        FontFamily::Name(REGULAR_FONT_NAME.to_string().into()),
        vec![REGULAR_FONT_NAME.to_string()],
    );
    fonts.families.insert(
        FontFamily::Name(BOLD_FONT_NAME.to_string().into()),
        vec![BOLD_FONT_NAME.to_string()],
    );
    fonts.families.insert(
        FontFamily::Name(ITALIC_FONT_NAME.to_string().into()),
        vec![ITALIC_FONT_NAME.to_string()],
    );
    fonts.families.insert(
        FontFamily::Name(BOLD_ITALIC_FONT_NAME.to_string().into()),
        vec![BOLD_ITALIC_FONT_NAME.to_string()],
    );

    ctx.set_fonts(fonts);
}

struct TerminalFonts {
    regular: FontFamily,
    bold: FontFamily,
    italic: FontFamily,
    bold_italic: FontFamily,
}

impl TerminalFonts {
    fn new() -> Self {
        let bold = FontFamily::Name(BOLD_FONT_NAME.to_string().into());
        let regular = FontFamily::Name(REGULAR_FONT_NAME.to_string().into());
        let italic = FontFamily::Name(ITALIC_FONT_NAME.to_string().into());
        let bold_italic = FontFamily::Name(BOLD_ITALIC_FONT_NAME.to_string().into());

        Self {
            regular,
            bold,
            italic,
            bold_italic,
        }
    }

    fn get_family(&self, font_decs: &[FontDecorations], weight: &FontWeight) -> FontFamily {
        // FIXME: We need to support a faint weight
        // FIXME: We probably need to support underline here too
        match (weight, font_decs.contains(&FontDecorations::Italic)) {
            (FontWeight::Bold, false) => self.bold.clone(),
            (FontWeight::Normal, false) => self.regular.clone(),
            (FontWeight::Normal, true) => self.italic.clone(),
            (FontWeight::Bold, true) => self.bold_italic.clone(),
        }
    }
}

pub fn internal_color_to_egui(
    default_color: Color32,
    color: TerminalColor,
    make_faint: bool,
) -> Color32 {
    let color_before_faimt = match color {
        TerminalColor::Default => default_color,
        TerminalColor::Black => Color32::BLACK,
        TerminalColor::Red => Color32::RED,
        TerminalColor::Green => Color32::GREEN,
        TerminalColor::Yellow => Color32::YELLOW,
        TerminalColor::Blue => Color32::BLUE,
        TerminalColor::Magenta => Color32::from_rgb(255, 0, 255),
        TerminalColor::Cyan => Color32::from_rgb(0, 255, 255),
        TerminalColor::White => Color32::WHITE,
        TerminalColor::BrightYellow => Color32::from_rgb(255, 234, 0),
        TerminalColor::BrightRed => Color32::from_rgb(238, 75, 43),
        TerminalColor::BrightGreen => Color32::from_rgb(170, 255, 0),
        TerminalColor::BrightBlue => Color32::from_rgb(0, 150, 255),
        TerminalColor::BrightMagenta => Color32::from_rgb(255, 0, 205),
        TerminalColor::BrightCyan => Color32::from_rgb(65, 253, 254),
        TerminalColor::BrightWhite => Color32::from_rgb(253, 254, 255),
        TerminalColor::BrightBlack => Color32::from_rgb(34, 32, 36),
        TerminalColor::Custom(r, g, b) => Color32::from_rgb(r, g, b),
    };

    if make_faint {
        color_before_faimt.gamma_multiply(0.5)
    } else {
        color_before_faimt
    }
}

pub const fn lookup_256_color_by_index(index: usize) -> (usize, usize, usize) {
    // https://stackoverflow.com/questions/69138165/how-to-get-the-rgb-values-of-a-256-color-palette-terminal-color
    match index {
        // standard colors 0 -15, as well as their bright counterparts 8-15
        // And the other values that map to them further up the color table
        1 => (128, 0, 0),
        2 => (0, 128, 0),
        3 => (128, 128, 0),
        4 => (0, 0, 128),
        5 => (128, 0, 128),
        6 => (0, 128, 128),
        7 => (192, 192, 192),
        8 | 244 => (128, 128, 128),
        9 | 196 => (255, 0, 0),
        10 | 46 => (0, 255, 0),
        11 | 226 => (255, 255, 0),
        12 | 21 => (0, 0, 255),
        13 | 201 => (255, 0, 255),
        14 | 51 => (0, 255, 255),
        15 | 231 => (255, 255, 255),
        // gray scale
        232..=255 => {
            let value = (2056 + 2570 * (index - 232)) / 256;

            (value, value, value)
        }
        // the blacks
        0 | 16 | 256.. => (0, 0, 0),
        // programtic colors
        _ => {
            let r = cube_component(index, 36);
            let g = cube_component(index, 6);
            let b = cube_component(index, 1);
            (r, g, b)
        }
    }
}

const fn cube_component(value: usize, modifier: usize) -> usize {
    let i = ((value - 16) / modifier) % 6;

    if i == 0 {
        0
    } else {
        (14135 + 10280 * i) / 256
    }
}

fn create_terminal_output_layout_job(
    style: &egui::Style,
    width: f32,
    data: &[u8],
) -> Result<(LayoutJob, TextFormat), std::str::Utf8Error> {
    let text_style = &style.text_styles[&TextStyle::Monospace];
    let data_utf8 = std::str::from_utf8(data)?;
    let mut job = egui::text::LayoutJob::simple(
        data_utf8.to_string(),
        text_style.clone(),
        style.visuals.text_color(),
        width,
    );

    job.wrap.break_anywhere = true;
    let textformat = job.sections[0].format.clone();
    job.sections.clear();
    Ok((job, textformat))
}

fn add_terminal_data_to_ui(
    ui: &mut Ui,
    data: &[u8],
    format_data: &[FormatTag],
    font_size: f32,
) -> Result<egui::Response, std::str::Utf8Error> {
    let (mut job, mut textformat) =
        create_terminal_output_layout_job(ui.style(), ui.available_width(), data)?;

    let default_color = textformat.color;
    let default_background = textformat.background;
    let terminal_fonts = TerminalFonts::new();

    for tag in format_data {
        let mut range = tag.start..tag.end;
        let color = tag.color;
        let background_color = tag.background_color;

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
        textformat.color = internal_color_to_egui(default_color, color, make_faint);
        // FIXME: ????? should background be faint? I feel like no, but....
        textformat.background = internal_color_to_egui(default_background, background_color, false);

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
    if scrollback_data.ends_with(b"\n") {
        scrollback_data = &scrollback_data[0..scrollback_data.len() - 1];
        if let Some(last_tag) = format_data.scrollback.last_mut() {
            last_tag.end = last_tag.end.min(scrollback_data.len());
        }
    }

    if canvas_data.ends_with(b"\n") {
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
                        error!("failed to add terminal data to ui: {}", backtraced_err(&e));
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
        setup_fonts(ctx);
        setup_bg_fill(ctx);

        Self {
            font_size: 12.0,
            debug_renderer: DebugRenderer::new(),
        }
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
