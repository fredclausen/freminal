// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

// FIXME: we should probably not do this?
#![allow(clippy::significant_drop_tightening)]

use std::sync::Arc;

use anyhow::Result;
use conv::ConvUtil;
use eframe::egui::{self, CentralPanel, Pos2, Vec2, ViewportCommand};
use fonts::get_char_size;
use freminal_common::window_manipulation::WindowManipulation;
use freminal_terminal_emulator::interface::TerminalEmulator;
use freminal_terminal_emulator::io::FreminalPtyInputOutput;
use parking_lot::FairMutex;
use terminal::FreminalTerminalWidget;
pub mod colors;
pub mod fonts;
pub mod mouse;
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
    terminal_emulator: Arc<FairMutex<TerminalEmulator<FreminalPtyInputOutput>>>,
    terminal_widget: FreminalTerminalWidget,
}

impl FreminalGui {
    fn new(
        cc: &eframe::CreationContext<'_>,
        terminal_emulator: Arc<FairMutex<TerminalEmulator<FreminalPtyInputOutput>>>,
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
        // log the frame time
        // time now
        debug!("Starting new frame");
        #[cfg(debug_assertions)]
        let now = std::time::Instant::now();

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

            let mut lock = self.terminal_emulator.lock();
            if let Err(e) = lock.set_win_size(width_chars, height_chars, font_width, font_height) {
                error!("failed to set window size {e}");
            }

            for window_event in lock.internal.window_commands.drain(..) {
                match window_event {
                    WindowManipulation::DeIconifyWindow => {
                        ui.ctx()
                            .send_viewport_cmd(ViewportCommand::Minimized(false));
                    }
                    WindowManipulation::MinimizeWindow => {
                        ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
                    }
                    WindowManipulation::MoveWindow(x, y) => {
                        let x = x.approx_as::<f32>().unwrap_or_default();
                        let y = y.approx_as::<f32>().unwrap_or_default();

                        ui.ctx()
                            .send_viewport_cmd(ViewportCommand::OuterPosition(Pos2::new(x, y)));
                    }
                    WindowManipulation::ResizeWindow(width, height) => {
                        let width = width.approx_as::<f32>().unwrap_or_default();
                        let height = height.approx_as::<f32>().unwrap_or_default();

                        ui.ctx()
                            .send_viewport_cmd(ViewportCommand::InnerSize(Vec2::new(
                                width, height,
                            )));
                    }
                    // These are ignored. eGui doesn't give us a stacking order thing (that I can tell)
                    // refresh window is already happening because we ended up here.
                    WindowManipulation::RefreshWindow
                    | WindowManipulation::LowerWindowToBottomOfStackingOrder
                    | WindowManipulation::RaiseWindowToTopOfStackingOrder => (),
                }
            }

            self.terminal_widget.show(ui, &mut lock);
        });

        panel_response.response.context_menu(|ui| {
            self.terminal_widget.show_options(ui);
        });

        #[cfg(debug_assertions)]
        // log the frame time
        let elapsed = now.elapsed();
        #[cfg(debug_assertions)]
        // show either elapsed as micros or millis, depending on the duration
        if elapsed.as_millis() > 0 {
            debug!("Frame time: {}ms", elapsed.as_millis());
        } else {
            debug!("Frame time: {}μs", elapsed.as_micros());
        }
    }
}

/// Run the GUI
///
/// # Errors
/// Will return an error if the GUI fails to run
pub fn run(
    terminal_emulator: Arc<FairMutex<TerminalEmulator<FreminalPtyInputOutput>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let native_options = eframe::NativeOptions::default();

    eframe::run_native(
        "Freminal",
        native_options,
        Box::new(move |cc| Ok(Box::new(FreminalGui::new(cc, terminal_emulator)))),
    )?;
    Ok(())
}
