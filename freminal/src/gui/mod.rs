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

#[allow(clippy::too_many_lines)]
fn handle_window_manipulation(
    ui: &egui::Ui,
    terminal_emulator: &mut TerminalEmulator<FreminalPtyInputOutput>,
    font_width: usize,
    font_height: usize,
    window_width: egui::Rect,
) {
    let window_commands: Vec<_> = terminal_emulator
        .internal
        .window_commands
        .drain(..)
        .collect();
    for window_event in window_commands {
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
                    .send_viewport_cmd(ViewportCommand::InnerSize(Vec2::new(width, height)));
            }
            WindowManipulation::MaximizeWindow => {
                ui.ctx().send_viewport_cmd(ViewportCommand::Maximized(true));
            }
            WindowManipulation::RestoreNonMaximizedWindow => {
                ui.ctx()
                    .send_viewport_cmd(ViewportCommand::Maximized(false));
            }
            WindowManipulation::ResizeWindowToLinesAndColumns(input_height, input_width) => {
                let available_height = ui.available_height();
                let available_width = ui.available_width();
                let width_difference = window_width.width() - available_width;
                let height_difference = window_width.height() - available_height;
                let width = input_width * font_width;
                let height = input_height * font_height;

                let width = width.approx_as::<f32>().unwrap_or_default() + width_difference;
                let height = height.approx_as::<f32>().unwrap_or_default() + height_difference;

                // FIXME: We can have an off by one because of all the rounding that happens with font height/width

                ui.ctx()
                    .send_viewport_cmd(ViewportCommand::InnerSize(Vec2::new(width, height)));
            }
            WindowManipulation::NotFullScreen => {
                ui.ctx()
                    .send_viewport_cmd(ViewportCommand::Fullscreen(false));
            }
            WindowManipulation::FullScreen => {
                ui.ctx()
                    .send_viewport_cmd(ViewportCommand::Fullscreen(true));
            }
            WindowManipulation::ToggleFullScreen => {
                let current_status = ui.ctx().input(|i| i.viewport().fullscreen.unwrap_or(false));
                ui.ctx()
                    .send_viewport_cmd(ViewportCommand::Fullscreen(!current_status));
            }
            WindowManipulation::ReportWindowState => {
                let current_status = ui.ctx().input(|i| i.viewport().minimized.unwrap_or(false));
                terminal_emulator
                    .internal
                    .report_window_state(current_status);
            }
            WindowManipulation::ReportWindowPositionWholeWindow => {
                let position = ui
                    .ctx()
                    .input(|i| {
                        i.raw.viewport().outer_rect.unwrap_or_else(|| {
                            error!("Failed to get viewport position. Using 0 as default");
                            egui::Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(0.0, 0.0))
                        })
                    })
                    .min;

                let pos_x = position.x.approx_as::<usize>().unwrap_or_else(|e| {
                    error!("Failed to convert position x to usize: {e}. Using 0 as default");
                    0
                });
                let pos_y = position.y.approx_as::<usize>().unwrap_or_else(|e| {
                    error!("Failed to convert position y to usize: {e}. Using 0 as default");
                    0
                });

                terminal_emulator
                    .internal
                    .report_window_position(pos_x, pos_y);
            }
            WindowManipulation::ReportWindowPositionTextArea => {
                let position = ui
                    .ctx()
                    .input(|i| {
                        i.raw.viewport().outer_rect.unwrap_or_else(|| {
                            error!("Failed to get viewport position. Using 0 as default");
                            egui::Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(0.0, 0.0))
                        })
                    })
                    .min;

                let available_height = ui.available_height();
                let available_width = ui.available_width();
                let width_difference = window_width.width() - available_width;
                let height_difference = window_width.height() - available_height;
                let pos_x = (position.y + height_difference)
                    .approx_as::<usize>()
                    .unwrap_or_else(|e| {
                        error!("Failed to convert position x to usize: {e}. Using 0 as default");
                        0
                    });
                let pos_y = (position.y + width_difference)
                    .approx_as::<usize>()
                    .unwrap_or_else(|e| {
                        error!("Failed to convert position y to usize: {e}. Using 0 as default");
                        0
                    });

                terminal_emulator
                    .internal
                    .report_window_position(pos_x, pos_y);
            }
            // These are ignored. eGui doesn't give us a stacking order thing (that I can tell)
            // refresh window is already happening because we ended up here.
            WindowManipulation::RefreshWindow
            | WindowManipulation::LowerWindowToBottomOfStackingOrder
            | WindowManipulation::RaiseWindowToTopOfStackingOrder => (),
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

            let font_width = font_width.round().approx_as::<usize>().unwrap_or_else(|e| {
                error!("Failed to convert font width to usize: {e}. Using 12 as default");
                12
            });

            let font_height = font_height
                .round()
                .approx_as::<usize>()
                .unwrap_or_else(|e| {
                    error!("Failed to convert font height to usize: {e}. Using 12 as default");
                    12
                });

            let mut lock = self.terminal_emulator.lock();
            if let Err(e) = lock.set_win_size(width_chars, height_chars, font_width, font_height) {
                error!("failed to set window size {e}");
            }

            let window_width = ctx.input(|i: &egui::InputState| i.screen_rect());
            handle_window_manipulation(ui, &mut lock, font_width, font_height, window_width);
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
) -> Result<()> {
    let native_options = eframe::NativeOptions::default();

    match eframe::run_native(
        "Freminal",
        native_options,
        Box::new(move |cc| Ok(Box::new(FreminalGui::new(cc, terminal_emulator)))),
    ) {
        Ok(()) => Ok(()),
        Err(e) => Err(anyhow::anyhow!(e.to_string())),
    }
}
