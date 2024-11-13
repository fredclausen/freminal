// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::terminal_emulator::interface::TerminalEmulator;
use crate::terminal_emulator::io::FreminalPtyInputOutput;
use anyhow::Result;
use eframe::egui::{self, CentralPanel};
use fonts::get_char_size;
use terminal::FreminalTerminalWidget;
pub mod colors;
pub mod fonts;
pub mod terminal;

fn set_egui_options(ctx: &egui::Context) {
    ctx.style_mut(|style| {
        style.visuals.window_fill = egui::Color32::BLACK;
        style.visuals.panel_fill = egui::Color32::BLACK;
    });
    ctx.options_mut(|options| {
        options.zoom_with_keyboard = false;
    });

    // ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
}
struct FreminalGui {
    terminal_emulator: TerminalEmulator<FreminalPtyInputOutput>,
    terminal_widget: FreminalTerminalWidget,
}

impl FreminalGui {
    fn new(
        cc: &eframe::CreationContext<'_>,
        terminal_emulator: TerminalEmulator<FreminalPtyInputOutput>,
    ) -> Self {
        set_egui_options(&cc.egui_ctx);

        Self {
            terminal_emulator,
            terminal_widget: FreminalTerminalWidget::new(&cc.egui_ctx),
        }
    }
}

impl eframe::App for FreminalGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let panel_response = CentralPanel::default().show(ctx, |ui| {
            let (width_chars, height_chars) = self.terminal_widget.calculate_available_size(ui);
            let (font_width, font_height) =
                get_char_size(ui.ctx(), self.terminal_widget.get_font_size());
            //FIXME: I know the value for font_width and font_height is going to fit within the usize range
            // Shut up clippy lint for now
            // but I want to idoimatically convert it to usize
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let font_width = font_width.round() as usize;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let font_height = font_height.round() as usize;

            if let Err(e) = self.terminal_emulator.set_win_size(
                width_chars,
                height_chars,
                font_width,
                font_height,
            ) {
                error!("failed to set window size {e}");
            }

            self.terminal_widget.show(ui, &mut self.terminal_emulator);
        });

        panel_response.response.context_menu(|ui| {
            self.terminal_widget.show_options(ui);
        });
    }
}

/// Run the GUI
///
/// # Errors
/// Will return an error if the GUI fails to run
pub fn run(
    terminal_emulator: TerminalEmulator<FreminalPtyInputOutput>,
) -> Result<(), Box<dyn std::error::Error>> {
    let native_options = eframe::NativeOptions::default();

    eframe::run_native(
        "Freminal",
        native_options,
        Box::new(move |cc| Ok(Box::new(FreminalGui::new(cc, terminal_emulator)))),
    )?;
    Ok(())
}
