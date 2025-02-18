// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use eframe::egui::Color32;
use freminal_common::colors::TerminalColor;

// We use the default Wez color scheme
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
        TerminalColor::Black => Color32::from_hex("#000000").unwrap_or_default(),
        TerminalColor::Red => Color32::from_hex("#cc5555").unwrap_or_default(),
        TerminalColor::Green => Color32::from_hex("#55cc55").unwrap_or_default(),
        TerminalColor::Yellow => Color32::from_hex("#cdcd55").unwrap_or_default(),
        TerminalColor::Blue => Color32::from_hex("#5555cc").unwrap_or_default(),
        TerminalColor::Magenta => Color32::from_hex("#cc55cc").unwrap_or_default(),
        TerminalColor::Cyan => Color32::from_hex("#7acaca").unwrap_or_default(),
        TerminalColor::White => Color32::from_hex("#b3b3b3").unwrap_or_default(),
        TerminalColor::BrightBlack => Color32::from_hex("#555555").unwrap_or_default(),
        TerminalColor::BrightRed => Color32::from_hex("#ff5555").unwrap_or_default(),
        TerminalColor::BrightGreen => Color32::from_hex("#55ff55").unwrap_or_default(),
        TerminalColor::BrightYellow => Color32::from_hex("#ffff55").unwrap_or_default(),
        TerminalColor::BrightBlue => Color32::from_hex("#5555ff").unwrap_or_default(),
        TerminalColor::BrightMagenta => Color32::from_hex("#ff55ff").unwrap_or_default(),
        TerminalColor::BrightCyan => Color32::from_hex("#55ffff").unwrap_or_default(),
        TerminalColor::BrightWhite => Color32::from_hex("#ffffff").unwrap_or_default(),
        TerminalColor::Custom(r, g, b) => Color32::from_rgb(r, g, b),
    };

    if make_faint {
        color_before_faint.gamma_multiply(0.5)
    } else {
        color_before_faint
    }
}

// https://github.com/mbadolato/iTerm2-Color-Schemes/blob/master/wezterm/Wez.toml
// # Wez
// [colors]
// foreground = "#b3b3b3"
// background = "#000000"
// cursor_bg = "#53ae71"
// cursor_border = "#53ae71"
// cursor_fg = "#000000"
// selection_bg = "#4d52f8"
// selection_fg = "#000000"

// ansi = ["#000000","#cc5555","#55cc55","#cdcd55","#5555cc","#cc55cc","#7acaca","#cccccc"]
// brights = ["#555555","#ff5555","#55ff55","#ffff55","#5555ff","#ff55ff","#55ffff","#ffffff"]
