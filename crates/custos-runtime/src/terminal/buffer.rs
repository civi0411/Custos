//! Bounded Stream Buffer for Terminal Sessions
//!
//! Enforces memory bounds on retained terminal output while supporting
//! monotonic sequence cursors for reliable client reconnection and streaming.

use custos_domain::TerminalOutputChunk;
use std::collections::VecDeque;

#[derive(Debug)]
pub struct BoundedTerminalBuffer {
    max_bytes: usize,
    chunks: VecDeque<(u64, Vec<u8>)>,
    current_seq: u64,
    total_bytes: usize,
    is_eof: bool,
}

impl BoundedTerminalBuffer {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes: max_bytes.max(4096),
            chunks: VecDeque::new(),
            current_seq: 0,
            total_bytes: 0,
            is_eof: false,
        }
    }

    /// Appends raw bytes from terminal master read.
    pub fn push(&mut self, data: &[u8]) -> (u64, u64) {
        if data.is_empty() {
            return (self.current_seq, self.current_seq);
        }

        let start_seq = self.current_seq;
        let next_seq = start_seq + data.len() as u64;
        self.chunks.push_back((start_seq, data.to_vec()));
        self.total_bytes += data.len();
        self.current_seq = next_seq;

        // Evict oldest chunks if total bytes exceed limit
        while self.total_bytes > self.max_bytes && self.chunks.len() > 1 {
            if let Some((_, popped)) = self.chunks.pop_front() {
                self.total_bytes = self.total_bytes.saturating_sub(popped.len());
            }
        }

        (start_seq, next_seq)
    }

    /// Marks the stream as having reached end-of-file (process exited).
    pub fn mark_eof(&mut self) {
        self.is_eof = true;
    }

    pub fn is_eof(&self) -> bool {
        self.is_eof
    }

    pub fn current_seq(&self) -> u64 {
        self.current_seq
    }

    /// Reads output starting from `from_seq` up to `max_bytes`.
    pub fn read_from(&self, session_id: &str, from_seq: u64, max_bytes: usize) -> TerminalOutputChunk {
        let max_bytes = if max_bytes == 0 { 64 * 1024 } else { max_bytes };

        if self.chunks.is_empty() {
            return TerminalOutputChunk {
                session_id: session_id.to_string(),
                start_seq: self.current_seq,
                next_seq: self.current_seq,
                data: String::new(),
                is_eof: self.is_eof,
            };
        }

        // If client requested a sequence beyond what we have:
        if from_seq >= self.current_seq {
            return TerminalOutputChunk {
                session_id: session_id.to_string(),
                start_seq: self.current_seq,
                next_seq: self.current_seq,
                data: String::new(),
                is_eof: self.is_eof,
            };
        }

        let mut collected = Vec::new();
        let mut actual_start = None;
        let mut actual_next = from_seq;

        for (chunk_start, chunk_data) in &self.chunks {
            let chunk_end = *chunk_start + chunk_data.len() as u64;
            if chunk_end <= from_seq {
                continue;
            }

            if actual_start.is_none() {
                actual_start = Some(from_seq.max(*chunk_start));
            }

            let slice_start = if from_seq > *chunk_start {
                (from_seq - *chunk_start) as usize
            } else {
                0
            };

            let remaining_budget = max_bytes.saturating_sub(collected.len());
            if remaining_budget == 0 {
                break;
            }

            let slice_end = (slice_start + remaining_budget).min(chunk_data.len());
            if slice_start < slice_end {
                collected.extend_from_slice(&chunk_data[slice_start..slice_end]);
                actual_next = *chunk_start + slice_end as u64;
            }

            if collected.len() >= max_bytes {
                break;
            }
        }

        let start_seq = actual_start.unwrap_or(from_seq);
        let reaches_end = actual_next >= self.current_seq;

        TerminalOutputChunk {
            session_id: session_id.to_string(),
            start_seq,
            next_seq: actual_next,
            data: String::from_utf8_lossy(&collected).to_string(),
            is_eof: self.is_eof && reaches_end,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_push_and_read() {
        let mut buf = BoundedTerminalBuffer::new(64 * 1024);
        let (s1, n1) = buf.push(b"hello ");
        assert_eq!(s1, 0);
        assert_eq!(n1, 6);

        let (s2, n2) = buf.push(b"world\n");
        assert_eq!(s2, 6);
        assert_eq!(n2, 12);

        let chunk = buf.read_from("s1", 0, 1024);
        assert_eq!(chunk.start_seq, 0);
        assert_eq!(chunk.next_seq, 12);
        assert_eq!(chunk.data, "hello world\n");
        assert!(!chunk.is_eof);

        buf.mark_eof();
        let chunk2 = buf.read_from("s1", 12, 1024);
        assert_eq!(chunk2.data, "");
        assert!(chunk2.is_eof);
    }

    #[test]
    fn test_buffer_partial_read() {
        let mut buf = BoundedTerminalBuffer::new(64 * 1024);
        buf.push(b"0123456789");

        let chunk = buf.read_from("s1", 3, 4);
        assert_eq!(chunk.start_seq, 3);
        assert_eq!(chunk.next_seq, 7);
        assert_eq!(chunk.data, "3456");
    }

    #[test]
    fn test_buffer_eviction() {
        let mut buf = BoundedTerminalBuffer::new(4096);
        let big = vec![b'a'; 3000];
        buf.push(&big);
        buf.push(&big);

        assert!(buf.total_bytes <= 4096 || buf.chunks.len() <= 2);
    }
}
