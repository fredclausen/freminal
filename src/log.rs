// Copyright (C) 2024 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::{cell::UnsafeCell, collections::HashMap, fmt::Arguments, mem::MaybeUninit, str::FromStr};

macro_rules! log {
    ($level:expr, $($arg:tt)+) => {
        if $level >= $crate::log::level(module_path!()) {
            $crate::log::log(
                $level,
                file!(),
                line!(),
                format_args!($($arg)+))
        }
    }
}

macro_rules! debug {
    ($($arg:tt)+) => {
        log!($crate::log::Level::Debug, $($arg)+)
    }
}

macro_rules! info {
    ($($arg:tt)+) => {
        log!($crate::log::Level::Info, $($arg)+)
    }
}

macro_rules! warn {
    ($($arg:tt)+) => {
        log!($crate::log::Level::Warn, $($arg)+)
    }
}

macro_rules! error {
    ($($arg:tt)+) => {
        log!($crate::log::Level::Error, $($arg)+)
    }
}

macro_rules! trace {
    ($($arg:tt)+) => {
        log!($crate::log::Level::Trace, $($arg)+)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
    Trace,
}

impl FromStr for Level {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warn" => Ok(Self::Warn),
            "error" => Ok(Self::Error),
            "trace" => Ok(Self::Trace),
            _ => Err(()),
        }
    }
}

impl Level {
    const fn log_str(self) -> &'static str {
        match self {
            Self::Debug => "\x1b[32;1mDEBUG\x1b[m",
            Self::Info => "\x1b[34;1m INFO\x1b[m",
            Self::Warn => "\x1b[33;1m WARN\x1b[m",
            Self::Error => "\x1b[91;1mERROR\x1b[m",
            Self::Trace => "\x1b[35;1mTRACE\x1b[m",
        }
    }
}

struct StaticLogLevels(UnsafeCell<MaybeUninit<HashMap<String, Level>>>);
unsafe impl Sync for StaticLogLevels {}

static LOG_LEVELS: StaticLogLevels = StaticLogLevels(UnsafeCell::new(MaybeUninit::uninit()));

pub fn init() {
    let log_str = std::env::var("FREMINAL_LOG");
    let Ok(log_str) = log_str else {
        unsafe {
            (*LOG_LEVELS.0.get()).write(HashMap::new());
        }
        return;
    };

    let mut levels = HashMap::new();

    for kv in log_str.split(';') {
        let last_equals = kv
            .chars()
            .enumerate()
            .filter(|(_i, c)| *c == '=')
            .map(|(i, _c)| i)
            .last();
        let Some(last_equals) = last_equals else {
            continue;
        };

        let (module, level) = kv.split_at(last_equals);
        let Ok(level) = level[1..].parse() else {
            continue;
        };
        levels.insert(module.to_string(), level);
    }

    unsafe {
        (*LOG_LEVELS.0.get()).write(levels);
    }
}

pub fn log(level: Level, file: &str, line: u32, args: Arguments) {
    print!("[{}] {file}:{line} ", level.log_str());
    println!("{args}");
}

pub fn level(module_path: &str) -> Level {
    unsafe {
        let levels = (*LOG_LEVELS.0.get()).assume_init_ref();
        *levels.get(module_path).unwrap_or(&Level::Info)
    }
}
