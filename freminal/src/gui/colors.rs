// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use eframe::egui::Color32;
use freminal_common::colors::TerminalColor;

#[must_use]
pub fn internal_color_to_egui(
    default_foreground_color: Color32,
    default_background_color: Color32,
    color: TerminalColor,
    make_faint: bool,
) -> Color32 {
    let color_before_faint = match color {
        TerminalColor::Default
        | TerminalColor::DefaultUnderlineColor
        | TerminalColor::DefaultCursorColor => default_foreground_color,
        TerminalColor::DefaultBackground => default_background_color,
        TerminalColor::Black => Color32::from_rgb(0, 0, 0),
        TerminalColor::Red => Color32::from_rgb(205, 0, 0),
        TerminalColor::Green => Color32::from_rgb(0, 205, 0),
        TerminalColor::Yellow => Color32::from_rgb(205, 205, 0),
        TerminalColor::Blue => Color32::from_rgb(0, 0, 238),
        TerminalColor::Magenta => Color32::from_rgb(205, 0, 205),
        TerminalColor::Cyan => Color32::from_rgb(0, 205, 205),
        TerminalColor::White => Color32::from_rgb(229, 229, 229),
        TerminalColor::BrightYellow => Color32::from_rgb(255, 255, 0),
        TerminalColor::BrightRed => Color32::from_rgb(255, 0, 0),
        TerminalColor::BrightGreen => Color32::from_rgb(0, 255, 0),
        TerminalColor::BrightBlue => Color32::from_rgb(92, 92, 255),
        TerminalColor::BrightMagenta => Color32::from_rgb(255, 0, 255),
        TerminalColor::BrightCyan => Color32::from_rgb(0, 255, 255),
        TerminalColor::BrightWhite => Color32::from_rgb(255, 255, 255),
        TerminalColor::BrightBlack => Color32::from_rgb(127, 127, 127),
        TerminalColor::Custom(r, g, b) => Color32::from_rgb(r, g, b),
    };

    if make_faint {
        color_before_faint.gamma_multiply(0.5)
    } else {
        color_before_faint
    }
}
