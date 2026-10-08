//! Terminal Coordinator
//!
//! Orchestrates bounded PTY streams scoped to ExecutionWorkspaces:
//! - Implements `custos_core::contracts::terminal::TerminalPort`
//! - Validates workspace scoping and working directory bounds
//! - Retains bounded output history per session for reliable reconnection
//! - Handles dynamic resizing, stdin writes, and clean termination

use super::buffer::BoundedTerminalBuffer;
use super::pty::PtyHandle;
use async_trait::async_trait;
use custos_core::contracts::terminal::TerminalPort;
use custos_domain::{
    DomainError, TerminalOutputChunk, TerminalSession, TerminalSessionId, TerminalSessionStatus,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::info;

/// Internal state of a live or retained terminal session.
struct TerminalSessionInstance {
    session: RwLock<TerminalSession>,
    pty: RwLock<Option<PtyHandle>>,
    buffer: RwLock<BoundedTerminalBuffer>,
}

/// Thread-safe coordinator for execution-scoped terminal sessions.
#[derive(Clone)]
pub struct TerminalCoordinator {
    sessions: Arc<RwLock<HashMap<TerminalSessionId, Arc<TerminalSessionInstance>>>>,
    max_buffer_bytes: usize,
}

impl TerminalCoordinator {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            max_buffer_bytes: 512 * 1024, // 512 KB scrollback retention
        }
    }

    pub fn with_max_buffer_bytes(mut self, bytes: usize) -> Self {
        self.max_buffer_bytes = bytes;
        self
    }
}

impl Default for TerminalCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TerminalPort for TerminalCoordinator {
    async fn spawn(
        &self,
        workspace_id: &str,
        working_dir: &Path,
        command: Option<&str>,
        cols: u16,
        rows: u16,
    ) -> Result<TerminalSession, DomainError> {
        if workspace_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "workspace_id cannot be empty for terminal spawn".into(),
            ));
        }

        if !working_dir.exists() {
            return Err(DomainError::NotFound {
                kind: "working_directory".into(),
                id: working_dir.display().to_string(),
            });
        }

        let canonical_dir = working_dir.canonicalize().map_err(|e| {
            DomainError::Validation(format!(
                "failed to resolve working directory {}: {e}",
                working_dir.display()
            ))
        })?;

        let session_id = format!("term_{}", uuid::Uuid::new_v4().simple());
        let command_str = command
            .filter(|c| !c.trim().is_empty())
            .unwrap_or("$SHELL")
            .to_string();

        let initial_session = TerminalSession {
            id: session_id.clone(),
            workspace_id: workspace_id.to_string(),
            working_dir: canonical_dir.display().to_string(),
            command: command_str,
            cols: cols.max(10),
            rows: rows.max(5),
            status: TerminalSessionStatus::Active,
            created_at: chrono::Utc::now().timestamp(),
            closed_at: None,
        };

        let (output_tx, mut output_rx) = mpsc::channel::<Vec<u8>>(128);

        let (pty_handle, exit_rx) = PtyHandle::spawn(
            &canonical_dir,
            command,
            initial_session.cols,
            initial_session.rows,
            output_tx,
        )
        .map_err(|e| {
            DomainError::Validation(format!("failed to spawn PTY process: {e}"))
        })?;

        let instance = Arc::new(TerminalSessionInstance {
            session: RwLock::new(initial_session.clone()),
            pty: RwLock::new(Some(pty_handle)),
            buffer: RwLock::new(BoundedTerminalBuffer::new(self.max_buffer_bytes)),
        });

        // Register session
        {
            let mut lock = self.sessions.write().await;
            lock.insert(session_id.clone(), instance.clone());
        }

        // Spawn background task to collect output chunks into bounded buffer
        let instance_buf = instance.clone();
        tokio::spawn(async move {
            while let Some(chunk) = output_rx.recv().await {
                let mut buf = instance_buf.buffer.write().await;
                buf.push(&chunk);
            }
        });

        // Spawn background task to monitor process exit
        let instance_exit = instance.clone();
        let sid = session_id.clone();
        tokio::spawn(async move {
            let exit_code = exit_rx.await.unwrap_or(None);
            info!(session_id = %sid, ?exit_code, "Terminal process exited");

            {
                let mut buf = instance_exit.buffer.write().await;
                buf.mark_eof();
            }

            {
                let mut sess = instance_exit.session.write().await;
                sess.status = TerminalSessionStatus::Exited { exit_code };
                sess.closed_at = Some(chrono::Utc::now().timestamp());
            }

            {
                let mut pty_guard = instance_exit.pty.write().await;
                pty_guard.take(); // drop handle and clean up
            }
        });

        Ok(initial_session)
    }

    async fn write(&self, session_id: &str, data: &[u8]) -> Result<usize, DomainError> {
        let instance = {
            let lock = self.sessions.read().await;
            lock.get(session_id).cloned().ok_or_else(|| {
                DomainError::NotFound {
                    kind: "terminal_session".into(),
                    id: session_id.to_string(),
                }
            })?
        };

        let pty_guard = instance.pty.read().await;
        if let Some(pty) = pty_guard.as_ref() {
            pty.write_all(data).map_err(|e| {
                DomainError::Validation(format!("failed to write to terminal stdin: {e}"))
            })
        } else {
            Err(DomainError::Conflict(format!(
                "terminal session {session_id} is no longer active"
            )))
        }
    }

    async fn resize(&self, session_id: &str, cols: u16, rows: u16) -> Result<(), DomainError> {
        let instance = {
            let lock = self.sessions.read().await;
            lock.get(session_id).cloned().ok_or_else(|| {
                DomainError::NotFound {
                    kind: "terminal_session".into(),
                    id: session_id.to_string(),
                }
            })?
        };

        {
            let mut sess = instance.session.write().await;
            sess.cols = cols;
            sess.rows = rows;
        }

        let pty_guard = instance.pty.read().await;
        if let Some(pty) = pty_guard.as_ref() {
            pty.resize(cols, rows).map_err(|e| {
                DomainError::Validation(format!("failed to resize terminal: {e}"))
            })?;
        }
        Ok(())
    }

    async fn read_output(
        &self,
        session_id: &str,
        from_seq: u64,
        max_bytes: usize,
    ) -> Result<TerminalOutputChunk, DomainError> {
        let instance = {
            let lock = self.sessions.read().await;
            lock.get(session_id).cloned().ok_or_else(|| {
                DomainError::NotFound {
                    kind: "terminal_session".into(),
                    id: session_id.to_string(),
                }
            })?
        };

        let chunk = {
            let buf = instance.buffer.read().await;
            buf.read_from(session_id, from_seq, max_bytes)
        };
        Ok(chunk)
    }

    async fn terminate(&self, session_id: &str) -> Result<(), DomainError> {
        let instance = {
            let lock = self.sessions.read().await;
            lock.get(session_id).cloned().ok_or_else(|| {
                DomainError::NotFound {
                    kind: "terminal_session".into(),
                    id: session_id.to_string(),
                }
            })?
        };

        {
            let mut pty_guard = instance.pty.write().await;
            if let Some(pty) = pty_guard.take() {
                let _ = pty.terminate();
            }
        }

        {
            let mut buf = instance.buffer.write().await;
            buf.mark_eof();
        }

        {
            let mut sess = instance.session.write().await;
            sess.status = TerminalSessionStatus::Terminated;
            sess.closed_at = Some(chrono::Utc::now().timestamp());
        }

        Ok(())
    }

    async fn list_sessions(
        &self,
        workspace_id: Option<&str>,
    ) -> Result<Vec<TerminalSession>, DomainError> {
        let lock = self.sessions.read().await;
        let mut result = Vec::new();

        for inst in lock.values() {
            let sess = inst.session.read().await.clone();
            if let Some(wid) = workspace_id {
                if sess.workspace_id == wid {
                    result.push(sess);
                }
            } else {
                result.push(sess);
            }
        }

        // Sort latest first
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(result)
    }

    async fn get_session(&self, session_id: &str) -> Result<TerminalSession, DomainError> {
        let inst = {
            let lock = self.sessions.read().await;
            lock.get(session_id).cloned().ok_or_else(|| {
                DomainError::NotFound {
                    kind: "terminal_session".into(),
                    id: session_id.to_string(),
                }
            })?
        };

        let sess = inst.session.read().await.clone();
        Ok(sess)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[cfg(unix)]
    async fn test_terminal_coordinator_lifecycle() {
        let temp = tempfile::tempdir().unwrap();
        let coord = TerminalCoordinator::new();

        // 1. Spawn echo command
        let session = coord
            .spawn(
                "ws-test",
                temp.path(),
                Some("echo hello_world"),
                80,
                24,
            )
            .await
            .unwrap();

        assert_eq!(session.workspace_id, "ws-test");
        assert_eq!(session.cols, 80);

        // 2. Poll output
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        let chunk = coord.read_output(&session.id, 0, 4096).await.unwrap();
        assert!(chunk.data.contains("hello_world"));

        // 3. List sessions
        let list = coord.list_sessions(Some("ws-test")).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, session.id);

        // 4. Terminate
        coord.terminate(&session.id).await.unwrap();
        let updated = coord.get_session(&session.id).await.unwrap();
        assert_eq!(updated.status, TerminalSessionStatus::Terminated);
    }

    #[tokio::test]
    async fn test_terminal_coordinator_rejects_missing_directory() {
        let coord = TerminalCoordinator::new();
        let res = coord
            .spawn(
                "ws-test",
                Path::new("/nonexistent/directory/that/does/not/exist"),
                None,
                80,
                24,
            )
            .await;

        assert!(matches!(res, Err(DomainError::NotFound { .. })));
    }
}
