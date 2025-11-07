//! Internal, lightweight ring buffer for capturing the most recent input bytes.
//! Kept fully internal (pub(crate)) and allocation-free on the hot path.

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SequenceTracer {
    buf: [u8; 128],
    len: usize,
    idx: usize,
}

impl Default for SequenceTracer {
    fn default() -> Self {
        Self::new()
    }
}

impl SequenceTracer {
    pub(crate) const fn new() -> Self {
        Self {
            buf: [0; 128],
            len: 0,
            idx: 0,
        }
    }

    #[allow(dead_code)]
    pub(crate) const fn clear(&mut self) {
        self.len = 0;
        self.idx = 0;
    }

    pub(crate) const fn push(&mut self, b: u8) {
        self.buf[self.idx] = b;
        self.idx = (self.idx + 1) % self.buf.len();
        if self.len < self.buf.len() {
            self.len += 1;
        }
    }

    pub(crate) fn as_str(&self) -> String {
        if self.len == 0 {
            return String::new();
        }
        let end = self.idx;
        let start = (self.idx + self.buf.len() - self.len) % self.buf.len();
        let mut out = Vec::with_capacity(self.len);
        if start < end {
            out.extend_from_slice(&self.buf[start..end]);
        } else {
            out.extend_from_slice(&self.buf[start..]);
            out.extend_from_slice(&self.buf[..end]);
        }
        String::from_utf8_lossy(&out).into_owned()
    }

    /// Trim trailing control terminators (ESC, '\', BEL) from the end of the trace.
    pub(crate) const fn trim_control_tail(&mut self) {
        while self.len > 0 {
            let end_idx = if self.idx == 0 {
                self.buf.len() - 1
            } else {
                self.idx - 1
            };
            let c = self.buf[end_idx];
            if matches!(c, 0x1B | 0x5C | 0x07) {
                self.idx = end_idx;
                self.len -= 1;
            } else {
                break;
            }
        }
    }
}

/// A small helper trait that standardizes how parsers collect and present
/// the raw bytes of the *current* sequence they are parsing.
#[allow(dead_code)]
pub(crate) trait SequenceTraceable {
    /// Mutable access to the underlying sequence trace buffer.
    fn seq_trace(&mut self) -> &mut SequenceTracer;
    /// Immutable access to the underlying sequence trace buffer.
    fn seq_trace_ref(&self) -> &SequenceTracer;

    /// Append a single byte to the sequence trace.
    fn append_trace(&mut self, b: u8) {
        self.seq_trace().push(b);
    }

    /// Clear the current sequence trace (typically on Finished/Invalid/Reset).
    fn clear_trace(&mut self) {
        self.seq_trace().clear();
    }

    /// Render the current trace as a lossy UTF-8 string for diagnostics.
    fn current_trace_str(&self) -> String {
        self.seq_trace_ref().as_str()
    }
}
