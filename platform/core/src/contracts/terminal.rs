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
