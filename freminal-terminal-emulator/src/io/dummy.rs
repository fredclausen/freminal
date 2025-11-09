// Copyright (C) 2024–2025 Fred Clausen
// MIT license.

use crate::io::FreminalTermInputOutput;

/// A no-op implementation of `FreminalTermInputOutput` used for headless benchmarking.
///
/// This struct satisfies the trait bounds required by `TerminalEmulator<Io>`
/// without performing any actual I/O.  It is completely deterministic and
/// safe to use in Criterion benchmarks.
#[derive(Default, Debug, Clone, Copy)]
pub struct DummyIo;

impl FreminalTermInputOutput for DummyIo {
    // The trait currently has no required methods.
    // If in the future it gains any, implement them here as no-ops.
}
