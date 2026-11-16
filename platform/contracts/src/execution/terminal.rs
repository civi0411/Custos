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

use async_trait::async_trait;
use std::path::Path;
use crate::identity::error::DomainError;

#[async_trait]
pub trait TerminalPort: Send + Sync {
    /// Spawns a new terminal session bound to a workspace directory.
    async fn spawn(
        &self,
        workspace_id: &str,
        working_dir: &Path,
        command: Option<&str>,
        cols: u16,
        rows: u16,
    ) -> Result<TerminalSession, DomainError>;

    /// Writes raw input bytes (keystrokes, commands) to the terminal stdin.
    async fn write(&self, session_id: &str, data: &[u8]) -> Result<usize, DomainError>;

    /// Resizes the pseudo-terminal window dimensions.
    async fn resize(&self, session_id: &str, cols: u16, rows: u16) -> Result<(), DomainError>;

    /// Reads buffered terminal output starting from `from_seq`.
    async fn read_output(
        &self,
        session_id: &str,
        from_seq: u64,
        max_bytes: usize,
    ) -> Result<TerminalOutputChunk, DomainError>;

    /// Terminates the terminal session process.
    async fn terminate(&self, session_id: &str) -> Result<(), DomainError>;

    /// Lists active terminal sessions, optionally filtered by workspace_id.
    async fn list_sessions(
        &self,
        workspace_id: Option<&str>,
    ) -> Result<Vec<TerminalSession>, DomainError>;

    /// Retrieves status of a single terminal session.
    async fn get_session(&self, session_id: &str) -> Result<TerminalSession, DomainError>;
}

/// Abstract handle for an active physical terminal process.
pub trait TerminalProcessHandle: Send + Sync {
    /// Writes raw bytes to stdin.
    fn write_all(&self, data: &[u8]) -> std::io::Result<usize>;

    /// Resizes the pseudo-terminal window dimensions.
    fn resize(&self, cols: u16, rows: u16) -> std::io::Result<()>;

    /// Terminates the process gracefully.
    fn terminate(&self) -> std::io::Result<()>;

    /// Force terminates the process and its process tree.
    fn force_terminate(&self) -> std::io::Result<()>;

    /// Returns process ID.
    fn pid(&self) -> u32;

    /// Checks if the process has exited.
    fn has_exited(&self) -> bool;
}

/// Callback consumer for terminal output chunks and process exit notifications.
pub trait TerminalOutputConsumer: Send + Sync + 'static {
    /// Receives raw output chunk from the terminal stream.
    fn on_output(&self, chunk: &[u8]);

    /// Receives process exit notification.
    fn on_exit(&self, exit_code: Option<i32>);
}

/// Physical driver port for pseudo-terminal process spawning.
#[async_trait]
pub trait TerminalDriverPort: Send + Sync {
    /// Spawns a new pseudo-terminal process handle with streaming consumer.
    async fn spawn_pty(
        &self,
        working_dir: &Path,
        command_override: Option<&str>,
        cols: u16,
        rows: u16,
        consumer: std::sync::Arc<dyn TerminalOutputConsumer>,
    ) -> std::io::Result<Box<dyn TerminalProcessHandle>>;
}
