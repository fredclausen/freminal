// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::{
    error::backtraced_err,
    terminal_emulator::{FreminalPtyInputOutput, TerminalEmulator},
    Args,
};
use easy_cast::ConvApprox;
use eframe::egui::{self, CentralPanel};
use terminal::{get_char_size, FreminalTerminalWidget};

pub mod terminal;

fn set_egui_options(ctx: &egui::Context, start_maximized: bool) {
    ctx.style_mut(|style| {
        style.visuals.window_fill = egui::Color32::BLACK;
        style.visuals.panel_fill = egui::Color32::BLACK;
    });
    ctx.options_mut(|options| {
        options.zoom_with_keyboard = false;
    });

    if start_maximized {
        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
    }
}
struct FreminalGui {
    terminal_emulator: TerminalEmulator<FreminalPtyInputOutput>,
    terminal_widget: FreminalTerminalWidget,
}

impl FreminalGui {
    fn new(
        cc: &eframe::CreationContext<'_>,
        terminal_emulator: TerminalEmulator<FreminalPtyInputOutput>,
        args: &Args,
    ) -> Self {
        set_egui_options(&cc.egui_ctx, args.start_maximized);

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
                get_char_size(ctx, self.terminal_widget.get_font_size());

            let rounded_font_width = usize::conv_approx(font_width.round());
            let rounded_font_height = usize::conv_approx(font_height.round());

            if let Err(e) = self.terminal_emulator.set_win_size(
                width_chars,
                height_chars,
                rounded_font_width,
                rounded_font_height,
            ) {
                error!("failed to set window size {}", backtraced_err(&*e));
            }

            self.terminal_widget.show(ui, &self.terminal_emulator);
        });

        panel_response.response.context_menu(|ui| {
            self.terminal_widget.show_options(ui);
        });
    }
}

pub fn run(
    terminal_emulator: TerminalEmulator<FreminalPtyInputOutput>,
    args: Args,
) -> Result<(), Box<dyn std::error::Error>> {
    let native_options = eframe::NativeOptions::default();

    eframe::run_native(
        "Freminal",
        native_options,
        Box::new(move |cc| Ok(Box::new(FreminalGui::new(cc, terminal_emulator, &args)))),
    )?;
    Ok(())
}
