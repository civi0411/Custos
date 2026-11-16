/// Truncate a String to at most `max_bytes` without slicing inside a multi-byte UTF-8 char.
pub fn truncate_to_char_boundary(s: &mut String, max_bytes: usize) {
    custos_domain::truncate_to_char_boundary(s, max_bytes);
}

/// A streaming writer that discards bytes beyond `limit` to prevent unbounded memory allocation
/// during JSON serialization of large tool outputs.
pub struct LimitedWriter<W> {
    inner: W,
    limit: usize,
    written: usize,
    capped: bool,
}

impl<W: std::io::Write> LimitedWriter<W> {
    pub fn new(inner: W, limit: usize) -> Self {
        Self {
            inner,
            limit,
            written: 0,
            capped: false,
        }
    }

    pub fn capped(&self) -> bool {
        self.capped
    }

    pub fn written(&self) -> usize {
        self.written
    }

    pub fn into_inner(self) -> (W, bool) {
        (self.inner, self.capped)
    }
}

impl<W: std::io::Write> std::io::Write for LimitedWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.written >= self.limit {
            self.capped = true;
            return Ok(buf.len());
        }
        let remaining = self.limit - self.written;
        let to_write = buf.len().min(remaining);
        let n = self.inner.write(&buf[..to_write])?;
        self.written += n;
        if self.written >= self.limit {
            self.capped = true;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
