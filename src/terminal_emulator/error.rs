// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use thiserror::Error;

#[derive(Debug, Error)]
#[error(transparent)]
pub enum ParserFailures {
    #[error("Unhandled Inner Escape: {0}")]
    UnhandledInnerEscape(String),
}
