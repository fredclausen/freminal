// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::{
    error::backtraced_err,
    terminal_emulator::{
        ControlAction, FreminalPtyInputOutput, FreminalRecordingHandle, FreminalReplayControl,
        FreminalReplayIo, LoadRecordingError, LoadSnapshotError, Recording, TerminalEmulator,
    },
};
use eframe::egui::{self, CentralPanel};
use terminal::FreminalTerminalWidget;
use thiserror::Error;

use std::path::{Path, PathBuf};

mod terminal;

fn set_egui_options(ctx: &egui::Context) {
    ctx.options_mut(|options| {
        options.zoom_with_keyboard = false;
    });
}

struct LoadReplayResponse {
    terminal_emulator: TerminalEmulator<FreminalReplayIo>,
    replay_control: FreminalReplayControl,
}

#[derive(Debug, Error)]
enum LoadReplayError {
    #[error("failed to load recording")]
    Recording(LoadRecordingError),
    #[error("failed to construct terminal emulator")]
    CreateTerminalEmulator(LoadSnapshotError),
}

fn load_replay(path: &Path) -> Result<LoadReplayResponse, LoadReplayError> {
    let recording = Recording::load(path).map_err(LoadReplayError::Recording)?;
    let mut replay_control = FreminalReplayControl::new(recording);
    let io_handle = replay_control.io_handle();
    let snapshot = replay_control.initial_state();
    let terminal_emulator = TerminalEmulator::from_snapshot(snapshot, io_handle)
        .map_err(LoadReplayError::CreateTerminalEmulator)?;
    Ok(LoadReplayResponse {
        terminal_emulator,
        replay_control,
    })
}

struct ReplayFreminalGui {
    terminal_emulator: TerminalEmulator<FreminalReplayIo>,
    terminal_widget: FreminalTerminalWidget,
    replay_path: PathBuf,
    replay_control: FreminalReplayControl,
    slider_pos: usize,
}

impl ReplayFreminalGui {
    fn new(
        cc: &eframe::CreationContext<'_>,
        replay_path: PathBuf,
        terminal_emulator: TerminalEmulator<FreminalReplayIo>,
        replay_control: FreminalReplayControl,
    ) -> Self {
        set_egui_options(&cc.egui_ctx);

        Self {
            terminal_emulator,
            terminal_widget: FreminalTerminalWidget::new(&cc.egui_ctx),
            replay_path,
            replay_control,
            slider_pos: 0,
        }
    }

    fn step_replay(&mut self) {
        let action = self.replay_control.next();
        match action {
            ControlAction::Resize { width, height } => {
                if let Err(e) = self.terminal_emulator.set_win_size(width, height) {
                    error!("failed to set window size: {}", backtraced_err(&*e));
                }
            }
            ControlAction::None => (),
        }
    }
}

impl eframe::App for ReplayFreminalGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let current_pos = self.replay_control.current_pos();
        if current_pos > self.slider_pos {
            match load_replay(&self.replay_path) {
                Ok(response) => {
                    self.terminal_emulator = response.terminal_emulator;
                    self.replay_control = response.replay_control;
                }
                Err(e) => {
                    error!("failed to reload replay: {}", backtraced_err(&e));
                }
            }
        }

        let current_pos = self.replay_control.current_pos();
        if current_pos < self.slider_pos {
            for _ in 0..self.slider_pos - current_pos {
                self.step_replay();
            }
        }

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            if ui.button("next").clicked() {
                self.step_replay();
                self.slider_pos += 1;
            }
        });

        egui::TopBottomPanel::bottom("seek").show(ctx, |ui| {
            ui.style_mut().spacing.slider_width = ui.available_width();
            let slider = egui::Slider::new(&mut self.slider_pos, 0..=self.replay_control.len())
                .show_value(false)
                .clamp_to_range(true);
            ui.add(slider);
        });

        let panel_response = CentralPanel::default().show(ctx, |ui| {
            self.terminal_widget.show(ui, &mut self.terminal_emulator);
        });

        panel_response.response.context_menu(|ui| {
            self.terminal_widget.show_options(ui);
        });
    }
}

struct FreminalGui {
    terminal_emulator: TerminalEmulator<FreminalPtyInputOutput>,
    terminal_widget: FreminalTerminalWidget,
    recording_handle: Option<FreminalRecordingHandle>,
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
            recording_handle: None,
        }
    }
}

impl eframe::App for FreminalGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let panel_response = CentralPanel::default().show(ctx, |ui| {
            let (width_chars, height_chars) = self.terminal_widget.calculate_available_size(ui);

            if let Err(e) = self
                .terminal_emulator
                .set_win_size(width_chars, height_chars)
            {
                error!("failed to set window size {}", backtraced_err(&*e));
            }

            self.terminal_widget.show(ui, &mut self.terminal_emulator);
        });

        panel_response.response.context_menu(|ui| {
            self.terminal_widget.show_options(ui);

            if self.recording_handle.is_some() {
                if ui.button("Stop recording").clicked() {
                    self.recording_handle = None;
                }
            } else if ui.button("Start recording").clicked() {
                match self.terminal_emulator.start_recording() {
                    Ok(v) => {
                        self.recording_handle = Some(v);
                    }
                    Err(e) => {
                        error!("failed to start recording: {}", backtraced_err(&e));
                    }
                }
            }
        });
    }
}

pub fn run_replay(replay_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let native_options = eframe::NativeOptions::default();

    let LoadReplayResponse {
        terminal_emulator,
        replay_control,
    } = load_replay(&replay_path)?;

    eframe::run_native(
        "Freminal",
        native_options,
        Box::new(move |cc| {
            Box::new(ReplayFreminalGui::new(
                cc,
                replay_path,
                terminal_emulator,
                replay_control,
            ))
        }),
    )?;

    Ok(())
}

pub fn run(
    terminal_emulator: TerminalEmulator<FreminalPtyInputOutput>,
) -> Result<(), Box<dyn std::error::Error>> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Freminal",
        native_options,
        Box::new(move |cc| Box::new(FreminalGui::new(cc, terminal_emulator))),
    )?;
    Ok(())
}
