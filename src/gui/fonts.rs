// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, FontId};

use crate::terminal_emulator::{FontDecorations, FontWeight};

const REGULAR_FONT_NAME: &str = "hack";
const BOLD_FONT_NAME: &str = "hack-bold";
const ITALIC_FONT_NAME: &str = "hack-italic";
const BOLD_ITALIC_FONT_NAME: &str = "hack-bold-italic";

pub fn setup_font_files(ctx: &egui::Context) {
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

pub struct TerminalFont {
    regular: FontFamily,
    bold: FontFamily,
    italic: FontFamily,
    bold_italic: FontFamily,
}

impl TerminalFont {
    pub fn new() -> Self {
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

    pub fn get_family(&self, font_decs: &[FontDecorations], weight: &FontWeight) -> FontFamily {
        match (weight, font_decs.contains(&FontDecorations::Italic)) {
            (FontWeight::Bold, false) => self.bold.clone(),
            (FontWeight::Normal, false) => self.regular.clone(),
            (FontWeight::Normal, true) => self.italic.clone(),
            (FontWeight::Bold, true) => self.bold_italic.clone(),
        }
    }
}

pub fn get_char_size(ctx: &egui::Context, font_size: f32) -> (f32, f32) {
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