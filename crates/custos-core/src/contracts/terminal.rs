//! Terminal Port Contract
//!
//! Provides execution-scoped PTY and pseudo-terminal lifecycle management:
//! - Spawns PTY sessions strictly bounded to ExecutionWorkspace root paths
//! - Bounded output capture with monotonic sequence cursors
//! - Dynamic terminal resize and signal termination
//! - Real-time stdin input transmission

use async_trait::async_trait;
use custos_domain::{DomainError, TerminalOutputChunk, TerminalSession};
use std::path::Path;

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
