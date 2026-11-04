use super::ResearchRepository;
use async_trait::async_trait;
use chrono::Utc;
use custos_core::contracts::storage::NotebookStoragePort;
use custos_domain::{
    CellExecutionStatus, DomainError, KernelStatus, NotebookCell, NotebookCellType,
    NotebookKernelState,
};
use rusqlite::params;

impl ResearchRepository {
    // Notebook Cells & Kernel Sessions
    pub fn list_notebook_cells(&self, session_id: &str) -> Result<Vec<NotebookCell>, DomainError> {
        self.list_notebook_cells_scoped(session_id, None)
    }

    pub fn list_notebook_cells_scoped(
        &self,
        session_id: &str,
        notebook_id: Option<&str>,
    ) -> Result<Vec<NotebookCell>, DomainError> {
        let conn = self.db.lock()?;
        let (sql, use_filter) = match notebook_id {
            Some(nb) if !nb.trim().is_empty() => (
                "SELECT id, session_id, cell_type, source, cell_index, execution_count, status, stdout, stderr, output_image, wall_ms, epoch, updated_at, notebook_id, cell_version
                 FROM notebook_cells WHERE session_id = ?1 AND notebook_id = ?2 ORDER BY cell_index ASC",
                true,
            ),
            _ => (
                "SELECT id, session_id, cell_type, source, cell_index, execution_count, status, stdout, stderr, output_image, wall_ms, epoch, updated_at, notebook_id, cell_version
                 FROM notebook_cells WHERE session_id = ?1 ORDER BY cell_index ASC",
                false,
            ),
        };
        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let map_row = |row: &rusqlite::Row| {
            let cell_type_str: String = row.get(2)?;
            let cell_type = match cell_type_str.as_str() {
                "markdown" => NotebookCellType::Markdown,
                _ => NotebookCellType::Code,
            };
            let status_str: String = row.get(6)?;
            let status = match status_str.as_str() {
                "running" => CellExecutionStatus::Running,
                "success" => CellExecutionStatus::Success,
                "error" => CellExecutionStatus::Error,
                _ => CellExecutionStatus::Idle,
            };
            let cell_idx: i64 = row.get(4)?;
            let exec_cnt: Option<i64> = row.get(5)?;
            let wall: Option<i64> = row.get(10)?;
            let ep: i64 = row.get(11)?;
            let nb_id: Option<String> = row.get(13)?;
            let version: Option<i64> = row.get(14)?;

            Ok(NotebookCell {
                id: row.get(0)?,
                session_id: row.get(1)?,
                cell_type,
                source: row.get(3)?,
                cell_index: cell_idx as u32,
                execution_count: exec_cnt.map(|c| c as u32),
                status,
                stdout: row.get(7)?,
                stderr: row.get(8)?,
                output_image: row.get(9)?,
                wall_ms: wall.map(|w| w as u64),
                epoch: ep as u32,
                updated_at: row.get(12)?,
                notebook_id: nb_id,
                cell_version: version.map(|v| v as u32),
            })
        };

        let rows = if use_filter {
            stmt.query_map(params![session_id, notebook_id], map_row)
        } else {
            stmt.query_map(params![session_id], map_row)
        }
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut res = Vec::new();
        for r in rows {
            res.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(res)
    }

    pub fn save_notebook_cell(&self, cell: &NotebookCell) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|error| DomainError::Validation(error.to_string()))?;
        Self::write_notebook_cell(&tx, cell)?;
        tx.execute(
            "INSERT INTO outbox_messages (id, topic, payload, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                custos_domain::new_id("cell_event"),
                "notebook.cell.saved",
                serde_json::json!({"cell_id": cell.id, "session_id": cell.session_id, "epoch": cell.epoch}).to_string(),
                Utc::now().timestamp()
            ],
        )
        .map_err(|error| DomainError::Validation(error.to_string()))?;
        tx.commit()
            .map_err(|error| DomainError::Validation(error.to_string()))?;
        Ok(())
    }

    pub(crate) fn write_notebook_cell(
        conn: &rusqlite::Connection,
        cell: &NotebookCell,
    ) -> Result<(), DomainError> {
        let cell_type_str = match cell.cell_type {
            NotebookCellType::Code => "code",
            NotebookCellType::Markdown => "markdown",
        };
        let status_str = match cell.status {
            CellExecutionStatus::Idle => "idle",
            CellExecutionStatus::Running => "running",
            CellExecutionStatus::Success => "success",
            CellExecutionStatus::Error => "error",
        };

        let affected = conn
            .execute(
                "INSERT INTO notebook_cells (
                id, session_id, cell_type, source, cell_index, execution_count, status, stdout, stderr, output_image, wall_ms, epoch, updated_at, notebook_id, cell_version
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            ON CONFLICT(id) DO UPDATE SET
                cell_type = excluded.cell_type,
                source = excluded.source,
                cell_index = excluded.cell_index,
                execution_count = excluded.execution_count,
                status = excluded.status,
                stdout = excluded.stdout,
                stderr = excluded.stderr,
                output_image = excluded.output_image,
                wall_ms = excluded.wall_ms,
                epoch = excluded.epoch,
                updated_at = excluded.updated_at,
                notebook_id = excluded.notebook_id,
                cell_version = excluded.cell_version
            WHERE notebook_cells.session_id = excluded.session_id",
                params![
                    cell.id,
                    cell.session_id,
                    cell_type_str,
                    cell.source,
                    cell.cell_index as i64,
                    cell.execution_count.map(|c| c as i64),
                    status_str,
                    cell.stdout,
                    cell.stderr,
                    cell.output_image,
                    cell.wall_ms.map(|w| w as i64),
                    cell.epoch as i64,
                    cell.updated_at,
                    cell.notebook_id,
                    cell.cell_version.map(|v| v as i64),
                ],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        if affected == 0 {
            return Err(DomainError::Validation(
                "Notebook cell belongs to a different session".into(),
            ));
        }

        Ok(())
    }

    pub fn save_notebook_cells(&self, cells: &[NotebookCell]) -> Result<(), DomainError> {
        for cell in cells {
            self.save_notebook_cell(cell)?;
        }
        Ok(())
    }

    /// Replaces a session projection atomically, including explicit cell deletions.
    pub fn replace_notebook_cells(
        &self,
        session_id: &str,
        cells: &[NotebookCell],
    ) -> Result<(), DomainError> {
        self.replace_notebook_cells_scoped(session_id, None, cells)
    }

    pub fn replace_notebook_cells_scoped(
        &self,
        session_id: &str,
        notebook_id: Option<&str>,
        cells: &[NotebookCell],
    ) -> Result<(), DomainError> {
        let ids: std::collections::HashSet<_> =
            cells.iter().map(|cell| cell.id.as_str()).collect();
        if session_id.trim().is_empty()
            || cells.len() > 256
            || ids.len() != cells.len()
            || cells
                .iter()
                .any(|cell| cell.id.is_empty() || cell.session_id != session_id)
        {
            return Err(DomainError::Validation(
                "Invalid notebook cell identities or session scope".into(),
            ));
        }
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        match notebook_id {
            Some(nb) if !nb.trim().is_empty() => {
                tx.execute(
                    "DELETE FROM notebook_cells WHERE session_id = ?1 AND notebook_id = ?2",
                    params![session_id, nb],
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            }
            _ => {
                tx.execute(
                    "DELETE FROM notebook_cells WHERE session_id = ?1",
                    params![session_id],
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            }
        }

        for cell in cells {
            Self::write_notebook_cell(&tx, cell)?;
        }
        let payload = serde_json::json!({
            "session_id": session_id,
            "notebook_id": notebook_id,
            "cell_ids": cells.iter().map(|cell| &cell.id).collect::<Vec<_>>()
        })
        .to_string();
        tx.execute(
            "INSERT INTO outbox_messages (id, topic, payload, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                custos_domain::new_id("nb"),
                "notebook.cells.replaced",
                payload,
                Utc::now().timestamp()
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        tx.commit()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub fn delete_notebook_cell(&self, cell_id: &str) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute("DELETE FROM notebook_cells WHERE id = ?1", params![cell_id])
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub fn get_notebook_kernel_state(
        &self,
        session_id: &str,
    ) -> Result<NotebookKernelState, DomainError> {
        self.get_notebook_kernel_state_scoped(session_id, None)
    }

    pub fn get_notebook_kernel_state_scoped(
        &self,
        session_id: &str,
        notebook_id: Option<&str>,
    ) -> Result<NotebookKernelState, DomainError> {
        let key = match notebook_id {
            Some(nb) if !nb.trim().is_empty() => format!("{session_id}::{nb}"),
            _ => session_id.to_string(),
        };
        let existing = {
            let conn = self.db.lock()?;
            let mut stmt = conn
                .prepare(
                    "SELECT session_id, epoch, status, python_version, execution_counter, created_at, updated_at, notebook_id, kernel_id
                     FROM notebook_kernel_sessions WHERE session_id = ?1",
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            let mut rows = stmt
                .query_map(params![key], |row| {
                    let status_str: String = row.get(2)?;
                    let status = match status_str.as_str() {
                        "busy" => KernelStatus::Busy,
                        "interrupted" => KernelStatus::Interrupted,
                        _ => KernelStatus::Idle,
                    };
                    let ep: i64 = row.get(1)?;
                    let counter: i64 = row.get(4)?;
                    let nb: Option<String> = row.get(7)?;
                    let kid: Option<String> = row.get(8)?;
                    Ok(NotebookKernelState {
                        session_id: session_id.to_string(),
                        epoch: ep as u32,
                        status,
                        python_version: row.get(3)?,
                        execution_counter: counter as u32,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                        notebook_id: nb.or_else(|| notebook_id.map(|s| s.to_string())),
                        kernel_id: kid,
                    })
                })
                .map_err(|e| DomainError::Validation(e.to_string()))?;

            if let Some(r) = rows.next() {
                Some(r.map_err(|e| DomainError::Validation(e.to_string()))?)
            } else {
                None
            }
        };

        if let Some(state) = existing {
            Ok(state)
        } else {
            let now = chrono::Utc::now().timestamp();
            let default_state = NotebookKernelState {
                session_id: session_id.to_string(),
                notebook_id: notebook_id.map(|s| s.to_string()),
                epoch: 1,
                status: KernelStatus::Idle,
                python_version: "Python 3.14".to_string(),
                execution_counter: 0,
                created_at: now,
                updated_at: now,
                ..Default::default()
            };
            self.save_notebook_kernel_state(&default_state)?;
            Ok(default_state)
        }
    }

    pub fn save_notebook_kernel_state(
        &self,
        state: &NotebookKernelState,
    ) -> Result<(), DomainError> {
        let key = match state.notebook_id.as_deref() {
            Some(nb) if !nb.trim().is_empty() && !state.session_id.contains("::") => {
                format!("{}::{}", state.session_id, nb)
            }
            _ => state.session_id.clone(),
        };
        let conn = self.db.lock()?;
        let status_str = match state.status {
            KernelStatus::Idle => "idle",
            KernelStatus::Busy => "busy",
            KernelStatus::Interrupted => "interrupted",
        };
        conn.execute(
            "INSERT INTO notebook_kernel_sessions (
                session_id, epoch, status, python_version, execution_counter, created_at, updated_at, notebook_id, kernel_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(session_id) DO UPDATE SET
                epoch = excluded.epoch,
                status = excluded.status,
                python_version = excluded.python_version,
                execution_counter = excluded.execution_counter,
                updated_at = excluded.updated_at,
                notebook_id = excluded.notebook_id,
                kernel_id = excluded.kernel_id",
            params![
                key,
                state.epoch as i64,
                status_str,
                state.python_version,
                state.execution_counter as i64,
                state.created_at,
                state.updated_at,
                state.notebook_id,
                state.kernel_id,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl NotebookStoragePort for ResearchRepository {
    async fn list_notebook_cells(
        &self,
        session_id: &str,
    ) -> Result<Vec<NotebookCell>, DomainError> {
        self.list_notebook_cells(session_id)
    }

    async fn list_notebook_cells_scoped(
        &self,
        session_id: &str,
        notebook_id: Option<&str>,
    ) -> Result<Vec<NotebookCell>, DomainError> {
        self.list_notebook_cells_scoped(session_id, notebook_id)
    }

    async fn replace_notebook_cells(
        &self,
        session_id: &str,
        cells: &[NotebookCell],
    ) -> Result<(), DomainError> {
        self.replace_notebook_cells(session_id, cells)
    }

    async fn replace_notebook_cells_scoped(
        &self,
        session_id: &str,
        notebook_id: Option<&str>,
        cells: &[NotebookCell],
    ) -> Result<(), DomainError> {
        self.replace_notebook_cells_scoped(session_id, notebook_id, cells)
    }

    async fn save_notebook_cell(&self, cell: &NotebookCell) -> Result<(), DomainError> {
        self.save_notebook_cell(cell)
    }

    async fn get_notebook_kernel_state(
        &self,
        session_id: &str,
    ) -> Result<NotebookKernelState, DomainError> {
        self.get_notebook_kernel_state(session_id)
    }

    async fn get_notebook_kernel_state_scoped(
        &self,
        session_id: &str,
        notebook_id: Option<&str>,
    ) -> Result<NotebookKernelState, DomainError> {
        self.get_notebook_kernel_state_scoped(session_id, notebook_id)
    }

    async fn save_notebook_kernel_state(
        &self,
        state: &NotebookKernelState,
    ) -> Result<(), DomainError> {
        self.save_notebook_kernel_state(state)
    }
}
