# Notebook Document, Scoped Kernel Binding and Migration (F06)

> SADE Master Specification — Phase 5 / F06 Architectural Blueprint
> Governs: `custos-domain`, `custos-core`, `custos-persistence`, `custos-runtime`, `custos-daemon`, `custos-sdk`, and `ui/desktop`.

---

## 1. Context & Motivation

Prior to F06, notebook cells and kernel states were keyed exclusively by `session_id`. This created three critical architectural defects:
1. **Namespace Collision & Cross-Notebook Cell Deletion:** If multiple notebooks belonged to the same chat session, saving/replacing cells in one notebook executed `DELETE FROM notebook_cells WHERE session_id = ?1`, instantly wiping all cells belonging to peer notebooks in that session.
2. **Kernel Process Memory Pollution:** Two distinct notebooks in the same chat shared the same active Python kernel. Variables defined in `Notebook_A` (e.g. `x = 42`) were silently readable or mutable in `Notebook_B`.
3. **Stale Execution & Idempotency Gaps:** Modifying a cell's source code did not invalidate previously evaluated output artifacts (`stdout`, `stderr`, `output_image`), and duplicate client execution requests caused multiple redundant evaluations.

F06 resolves these deficiencies with strict document-level identity, scoped kernel session mapping, cell versioning, and zero-loss legacy data migration.

---

## 2. Domain Entities (`custos-domain`)

### 2.1 `NotebookDocument` & `NotebookKernelBinding`

```rust
pub struct NotebookDocument {
    pub id: String,
    pub session_id: String,
    pub title: String,
    pub path: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct NotebookKernelBinding {
    pub notebook_id: String,
    pub kernel_id: String,
    pub session_id: String,
    pub bound_at: i64,
}
```

### 2.2 `NotebookCell` Invalidation & Versioning

```rust
impl NotebookCell {
    /// Increments `cell_version`, sets status to `Idle`, and clears outputs if source changed.
    pub fn invalidate_output_on_edit(&mut self, new_source: &str, updated_at: i64) -> bool {
        if self.source == new_source {
            return false;
        }
        self.source = new_source.to_string();
        self.status = CellExecutionStatus::Idle;
        self.stdout = None;
        self.stderr = None;
        self.output_image = None;
        self.execution_count = None;
        self.cell_version = Some(self.cell_version.unwrap_or(1) + 1);
        self.updated_at = updated_at;
        true
    }

    /// Verifies optimistic concurrency against an expected version.
    pub fn is_version_mismatch(&self, expected_version: Option<u32>) -> bool {
        match expected_version {
            Some(expected) => self.cell_version.unwrap_or(1) != expected,
            None => false,
        }
    }
}
```

---

## 3. Storage Contracts & Persistence Layer (`custos-core` & `custos-persistence`)

### 3.1 Migration 0041: Schema Evolution

```sql
-- Migration 0041: Notebook Document & Kernel Scoping
ALTER TABLE notebook_cells ADD COLUMN notebook_id TEXT;
ALTER TABLE notebook_cells ADD COLUMN cell_version INTEGER DEFAULT 1;

CREATE INDEX IF NOT EXISTS idx_notebook_cells_scoped
    ON notebook_cells(session_id, notebook_id, cell_index);

ALTER TABLE notebook_kernel_sessions ADD COLUMN notebook_id TEXT;
ALTER TABLE notebook_kernel_sessions ADD COLUMN kernel_id TEXT;
```

### 3.2 Scoped Repository Operations

- `list_notebook_cells_scoped(session_id, notebook_id)`: Filters cells by both `session_id` and optional `notebook_id`.
- `replace_notebook_cells_scoped(session_id, notebook_id, cells)`: Uses target-scoped deletion:
  ```sql
  DELETE FROM notebook_cells WHERE session_id = ?1 AND notebook_id = ?2
  ```
  Cells in peer notebooks belonging to the same session remain untouched.
- `get_notebook_kernel_state_scoped(session_id, notebook_id)`: Composite keying `{session_id}::{notebook_id}` provides isolated kernel states in SQLite while retaining full backward compatibility for un-scoped legacy sessions.

---

## 4. Runtime & Daemon Coordination (`custos-runtime` & `custos-daemon`)

1. **Kernel Keying:** `PythonKernelCoordinator::kernel_key(session_id, notebook_id)` yields `{session_id}::{notebook_id}` when `notebook_id` is present, or falls back to `session_id`.
2. **Idempotency Cache:** `ActiveKernel` tracks `recent_requests: HashMap<String, ExecuteCellResult>` keyed by `execution_request_id`. Duplicate executions are returned immediately from memory.
3. **Pre-Execution Concurrency Gate:** Daemon application handler checks `cell.is_version_mismatch(params.cell_version)` before handing off execution to the coordinator. Stale execution attempts return `DomainError::Validation`.

---

## 5. Golden Acceptance Gate (G06)

Verification is codified in `tests/e2e/tests/notebook_binding_golden_test.rs`:
- `test_g06_two_notebooks_in_same_session_have_isolated_kernels`: Confirms memory isolation.
- `test_g06_two_panes_opening_same_notebook_share_kernel_memory`: Confirms handle sharing across multiple UI views.
- `test_g06_cell_source_edit_invalidates_output_and_bumps_version`: Confirms output invalidation and version increment.
- `test_g06_duplicate_execution_request_id_returns_cached_result`: Confirms request deduplication.
- `test_g06_multi_notebook_persistence_isolation_in_same_session`: Confirms persistence scoping.
