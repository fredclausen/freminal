// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use super::mode::SetMode;

pub mod decawm;
pub mod decckm;
pub mod dectcem;
pub mod rl_bracket;
pub mod xtcblink;
pub mod xtextscrn;
pub mod xtmsewin;

pub trait ReportMode {
    fn report(&self, override_mode: Option<SetMode>) -> String;
}

pub trait MouseModeNumber {
    fn mouse_mode_number(&self) -> usize;
}
