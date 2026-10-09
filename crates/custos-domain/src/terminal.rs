//! Terminal Domain Models
//!
//! Provides bounded PTY and terminal session models scoped to ExecutionWorkspaces:
//! - Terminal session lifecycle (Active, Exited, Terminated)
//! - Scoped working directory enforcement
//! - Bounded output chunks with monotonic sequence cursors

use serde::{Deserialize, Serialize};

/// Unique identifier for a terminal session.
pub type TerminalSessionId = String;

/// Current status of a terminal session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TerminalSessionStatus {
    Active,
    Exited { exit_code: Option<i32> },
    Terminated,
}

impl TerminalSessionStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }
}

/// Metadata and state of an execution-scoped terminal session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSession {
    /// Unique session identifier.
    pub id: TerminalSessionId,
    /// ID of the execution workspace this terminal is bound to.
    pub workspace_id: String,
    /// Absolute path to the working directory on the host.
    pub working_dir: String,
    /// Initial shell or command executed (e.g., "$SHELL" or "zsh").
    pub command: String,
    /// Terminal column count.
    pub cols: u16,
    /// Terminal row count.
    pub rows: u16,
    /// Lifecycle status.
    pub status: TerminalSessionStatus,
    /// Timestamp when session was created (Unix epoch seconds).
    pub created_at: i64,
    /// Timestamp when session ended, if applicable.
    pub closed_at: Option<i64>,
}

/// Bounded output chunk returned when polling or streaming terminal output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalOutputChunk {
    /// Associated session identifier.
    pub session_id: TerminalSessionId,
    /// Monotonic sequence cursor for the first byte of this chunk.
    pub start_seq: u64,
    /// Monotonic sequence cursor after this chunk.
    pub next_seq: u64,
    /// Raw terminal output text (lossy UTF-8).
    pub data: String,
    /// Flag indicating whether the terminal process has terminated and stream has reached EOF.
    pub is_eof: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_session_serialization_roundtrip() {
        let session = TerminalSession {
            id: "term-123".into(),
            workspace_id: "ws-abc".into(),
            working_dir: "/tmp/ws".into(),
            command: "/bin/zsh".into(),
            cols: 80,
            rows: 24,
            status: TerminalSessionStatus::Active,
            created_at: 1700000000,
            closed_at: None,
        };

        let json = serde_json::to_string(&session).unwrap();
        let decoded: TerminalSession = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, "term-123");
        assert_eq!(decoded.cols, 80);
        assert_eq!(decoded.status, TerminalSessionStatus::Active);
    }

    #[test]
    fn test_terminal_chunk_seq_continuity() {
        let chunk = TerminalOutputChunk {
            session_id: "term-123".into(),
            start_seq: 100,
            next_seq: 150,
            data: "hello world".into(),
            is_eof: false,
        };
        assert_eq!(chunk.next_seq, 150);
        assert!(!chunk.is_eof);
    }
}
