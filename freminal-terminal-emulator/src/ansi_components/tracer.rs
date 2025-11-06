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
}
