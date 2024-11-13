// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use thiserror::Error;

#[derive(Debug, Error, Eq, PartialEq)]
#[error(transparent)]
pub enum ParserFailures {
    #[error("Parsed pushed to once finished")]
    ParsedPushedToOnceFinished,
    #[error("Unhandled Inner Escape: {0}")]
    UnhandledInnerEscape(String),
    #[error("Invalid cursor (CHA) set position sequence: {0}")]
    UnhandledCHACommand(String),
    #[error("Invalid cursor (CUU) set position sequence: {0}")]
    UnhandledCUUCommand(String),
    #[error("Invalid cursor (CUB) set position sequence: {0}")]
    UnhandledCUBCommand(String),
    #[error("Invalid cursor (CUD) set position sequence: {0}")]
    UnhandledCUDCommand(String),
    #[error("Invalid cursor (CUF) set position sequence: {0}")]
    UnhandledCUFCommand(String),
    #[error("Invalid cursor (CUP) set position sequence: {0:?}")]
    UnhandledCUPCommand(Vec<u8>),
    #[error("Invalid cursor (DCH) set position sequence: {0}")]
    UnhandledDCHCommand(String),
    #[error("Invalid cursor (ED) set position sequence: {0}")]
    UnhandledEDCommand(String),
    #[error("Invalid cursor (EL) set position sequence: {0}")]
    UnhandledELCommand(String),
    #[error("Invalid cursor (IL) set position sequence: {0}")]
    UnhandledILCommand(String),
    #[error("Invalid cursor (SGR) set position sequence: {0}")]
    UnhandledSGRCommand(String),
    #[error("Invalid cursor (ICH) set position sequence: {0}")]
    UnhandledICHCommand(String),
    #[error("Invalid TChar: {0:?}")]
    InvalidTChar(Vec<u8>),
}
