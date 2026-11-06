//! Port implementations for SqliteTaskStore.
//!
//! Implements:
//! - TaskStore & SessionStore (Kernel storage contracts)
//! - OutboxPort & EffectLedgerPort (Outbox transaction and idempotency contracts)
//! - RunPort, DecisionPort, WorkflowRevisionPort, ReplanPort (Execution contracts)
//! - CommandLedgerPort (Worktree command idempotency)
//! - NotebookStoragePort & WorkspaceRepoPort (Research & Workspace contracts)
//! - QuantPort (Quantitative trading tearsheet and order ticket persistence)

use super::SqliteTaskStore;
use async_trait::async_trait;
use custos_core::contracts::storage::{
    CommandAdmission, CommandLedgerEntry, CommandLedgerPort, DecisionPort, EffectLedgerPort,
    NotebookStoragePort, OutboxEntry, OutboxPort, OutboxStatus, QuantPort, ReplanPort, RunPort,
    WorkflowRevisionPort,
};
use custos_core::contracts::workspace::WorkspaceRepository as WorkspaceRepoPort;
use custos_core::kernel::{SessionStore, TaskEvent, TaskStore};
use custos_domain::{
    ContinuationPacket, DecisionRecord, DispatchClaim, DomainError, ExecutionReceipt,
    ExecutionWorkspace, LaunchAttempt, NodePlacement, NotebookCell, NotebookKernelState,
    ReplanRecord, Run, Session, SessionId, SessionJournalEntry, Span, Task, WorkerRun,
    WorkflowRevision, WorkspaceId, WorkspaceStatus,
};

#[async_trait]
impl TaskStore for SqliteTaskStore {
    async fn get_task(&self, task_id: &str) -> Result<Option<Task>, DomainError> {
        self.task_repo.get_task(task_id)
    }

    async fn list_tasks(&self) -> Result<Vec<Task>, DomainError> {
        self.task_repo.list_tasks()
    }

    async fn save_task(&self, task: &Task) -> Result<(), DomainError> {
        self.task_repo.save_task(task)
    }

    async fn commit_task_event(&self, event: &TaskEvent, task: &Task) -> Result<(), DomainError> {
        self.task_repo.save_task_with_event(task, event)
    }

    async fn get_task_events(&self, task_id: &str) -> Result<Vec<TaskEvent>, DomainError> {
        self.task_repo.get_task_events(task_id)
    }

    async fn get_span(&self, span_id: &str) -> Result<Option<Span>, DomainError> {
        self.span_repo.get_span(span_id)
    }

    async fn save_span(&self, span: &Span) -> Result<(), DomainError> {
        self.span_repo.save_span(span)
    }

    async fn list_spans(&self, task_id: &str) -> Result<Vec<Span>, DomainError> {
        self.span_repo.list_spans(task_id)
    }

    async fn save_continuation(&self, packet: &ContinuationPacket) -> Result<(), DomainError> {
        self.continuation_repo.save_continuation(packet)
    }

    async fn get_continuation(
        &self,
        task_id: &str,
        to_span: u32,
    ) -> Result<Option<ContinuationPacket>, DomainError> {
        self.continuation_repo.get_continuation(task_id, to_span)
    }

    async fn get_latest_continuation(
        &self,
        task_id: &str,
    ) -> Result<Option<ContinuationPacket>, DomainError> {
        self.continuation_repo.get_latest_continuation(task_id)
    }
}

#[async_trait]
impl SessionStore for SqliteTaskStore {
    async fn get_session(&self, session_id: &SessionId) -> Result<Option<Session>, DomainError> {
        self.session_repo.get_session(session_id)
    }

    async fn list_sessions(&self) -> Result<Vec<Session>, DomainError> {
        self.session_repo.list_sessions()
    }

    async fn save_session(&self, session: &Session) -> Result<(), DomainError> {
        self.session_repo.save_session(session)
    }

    async fn delete_session(&self, session_id: &SessionId) -> Result<bool, DomainError> {
        self.session_repo.delete_session(session_id)
    }

    async fn append_journal(&self, entry: &SessionJournalEntry) -> Result<i64, DomainError> {
        self.session_repo.append_journal(entry)
    }

    async fn get_journal(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<SessionJournalEntry>, DomainError> {
        self.session_repo.get_journal(session_id)
    }
}

#[async_trait]
impl OutboxPort for SqliteTaskStore {
    async fn enqueue(&self, entry: OutboxEntry) -> Result<(), DomainError> {
        self.outbox_repo.enqueue(entry).await
    }

    async fn get(&self, id: &str) -> Result<Option<OutboxEntry>, DomainError> {
        self.outbox_repo.get(id).await
    }

    async fn mark_dispatching(&self, id: &str) -> Result<(), DomainError> {
        self.outbox_repo.mark_dispatching(id).await
    }

    async fn mark_approved(&self, id: &str) -> Result<(), DomainError> {
        self.outbox_repo.mark_approved(id).await
    }

    async fn mark_receipted(&self, id: &str, receipt: ExecutionReceipt) -> Result<(), DomainError> {
        self.outbox_repo.mark_receipted(id, receipt).await
    }

    async fn mark_uncertain(&self, id: &str) -> Result<(), DomainError> {
        self.outbox_repo.mark_uncertain(id).await
    }

    async fn mark_failed(&self, id: &str, error: &str) -> Result<(), DomainError> {
        self.outbox_repo.mark_failed(id, error).await
    }

    async fn list_by_status(&self, status: OutboxStatus) -> Result<Vec<OutboxEntry>, DomainError> {
        self.outbox_repo.list_by_status(status).await
    }

    async fn list_by_task_and_status(
        &self,
        task_id: &str,
        status: OutboxStatus,
    ) -> Result<Vec<OutboxEntry>, DomainError> {
        self.outbox_repo
            .list_by_task_and_status(task_id, status)
            .await
    }
}

#[async_trait]
impl EffectLedgerPort for SqliteTaskStore {
    async fn record_effect(
        &self,
        effect: &custos_domain::EffectAttempt,
    ) -> Result<(), DomainError> {
        self.outbox_repo.record_effect(effect)
    }

    async fn update_effect_status(
        &self,
        id: &str,
        status: custos_domain::EffectStatus,
        receipt: Option<&ExecutionReceipt>,
    ) -> Result<(), DomainError> {
        self.outbox_repo.update_effect_status(id, status, receipt)
    }

    async fn get_effect_by_idempotency_key(
        &self,
        key: &str,
    ) -> Result<Option<custos_domain::EffectAttempt>, DomainError> {
        self.outbox_repo.get_effect_by_idempotency_key(key)
    }

    async fn reconcile_on_startup(&self) -> Result<usize, DomainError> {
        self.outbox_repo.reconcile_on_startup()
    }
}

#[async_trait]
impl RunPort for SqliteTaskStore {
    async fn save_run(&self, run: &mut Run) -> Result<(), DomainError> {
        self.run_repo.save_run(run)
    }

    async fn get_run(&self, run_id: &str) -> Result<Option<Run>, DomainError> {
        self.run_repo.get_run(run_id)
    }

    async fn list_runs_for_task(&self, task_id: &str) -> Result<Vec<Run>, DomainError> {
        self.run_repo.list_runs_for_task(task_id)
    }

    async fn save_worker_run(&self, wrun: &mut WorkerRun) -> Result<(), DomainError> {
        self.run_repo.save_worker_run(wrun)
    }

    async fn get_worker_run(&self, wrun_id: &str) -> Result<Option<WorkerRun>, DomainError> {
        self.run_repo.get_worker_run(wrun_id)
    }

    async fn list_worker_runs_for_task(
        &self,
        task_id: &str,
    ) -> Result<Vec<WorkerRun>, DomainError> {
        self.run_repo.list_worker_runs_for_task(task_id)
    }

    async fn list_worker_runs_for_run(&self, run_id: &str) -> Result<Vec<WorkerRun>, DomainError> {
        self.run_repo.list_worker_runs_for_run(run_id)
    }

    async fn claim_ready(&self, claim: &DispatchClaim) -> Result<(), DomainError> {
        self.run_repo.claim_ready(claim)
    }

    async fn update_claim(&self, claim: &DispatchClaim) -> Result<(), DomainError> {
        self.run_repo.update_claim(claim)
    }

    async fn get_claim(&self, claim_id: &str) -> Result<Option<DispatchClaim>, DomainError> {
        self.run_repo.get_claim(claim_id)
    }

    async fn get_active_claim(
        &self,
        task_id: &str,
        node_id: Option<&str>,
    ) -> Result<Option<DispatchClaim>, DomainError> {
        self.run_repo.get_active_claim(task_id, node_id)
    }

    async fn save_launch_attempt(&self, attempt: &LaunchAttempt) -> Result<(), DomainError> {
        self.run_repo.save_launch_attempt(attempt)
    }

    async fn get_launch_attempt(
        &self,
        attempt_id: &str,
    ) -> Result<Option<LaunchAttempt>, DomainError> {
        self.run_repo.get_launch_attempt(attempt_id)
    }

    async fn list_active_claims(&self) -> Result<Vec<DispatchClaim>, DomainError> {
        self.run_repo.list_active_claims()
    }

    async fn fence_claim(&self, claim_id: &str) -> Result<(), DomainError> {
        self.run_repo.fence_claim(claim_id)
    }

    async fn reconcile_runs_on_startup(&self) -> Result<usize, DomainError> {
        self.run_repo.reconcile_runs_on_startup()
    }
}

#[async_trait]
impl DecisionPort for SqliteTaskStore {
    async fn record_decision(&self, record: &DecisionRecord) -> Result<(), DomainError> {
        self.decision_repo.save_decision(record)
    }

    async fn get_decision(&self, id: &str) -> Result<Option<DecisionRecord>, DomainError> {
        self.decision_repo.get_decision(id)
    }

    async fn list_decisions_for_task(
        &self,
        task_id: &str,
    ) -> Result<Vec<DecisionRecord>, DomainError> {
        self.decision_repo.list_decisions_for_task(task_id)
    }
}

#[async_trait]
impl WorkflowRevisionPort for SqliteTaskStore {
    async fn save_revision(
        &self,
        revision: &WorkflowRevision,
        placements: &[NodePlacement],
    ) -> Result<(), DomainError> {
        self.workflow_repo.save_revision(revision, placements)
    }

    async fn get_revision(
        &self,
        revision_id: &str,
    ) -> Result<Option<WorkflowRevision>, DomainError> {
        self.workflow_repo.get_revision(revision_id)
    }

    async fn list_revisions_for_task(
        &self,
        task_id: &str,
    ) -> Result<Vec<WorkflowRevision>, DomainError> {
        self.workflow_repo.list_revisions_for_task(task_id)
    }

    async fn get_placements_for_revision(
        &self,
        revision_id: &str,
    ) -> Result<Vec<NodePlacement>, DomainError> {
        self.workflow_repo.get_placements_for_revision(revision_id)
    }
}

#[async_trait]
impl ReplanPort for SqliteTaskStore {
    async fn record_replan(&self, record: &ReplanRecord) -> Result<(), DomainError> {
        self.replan_repo.save_replan(record)
    }

    async fn get_replan(&self, id: &str) -> Result<Option<ReplanRecord>, DomainError> {
        self.replan_repo.get_replan(id)
    }

    async fn list_replans_for_task(&self, task_id: &str) -> Result<Vec<ReplanRecord>, DomainError> {
        self.replan_repo.list_replans_for_task(task_id)
    }
}

#[async_trait]
impl CommandLedgerPort for SqliteTaskStore {
    async fn get_command(
        &self,
        command_id: &str,
    ) -> Result<Option<CommandLedgerEntry>, DomainError> {
        self.command_ledger_repo.get_command(command_id)
    }

    async fn record_command(&self, entry: &CommandLedgerEntry) -> Result<(), DomainError> {
        self.command_ledger_repo.record_command(entry)
    }

    async fn admit(
        &self,
        command_id: &str,
        method: &str,
        payload_digest: &str,
        scope: Option<&str>,
        now_ms: i64,
        ttl_ms: i64,
    ) -> Result<CommandAdmission, DomainError> {
        self.command_ledger_repo
            .admit(command_id, method, payload_digest, scope, now_ms, ttl_ms)
    }

    async fn complete(&self, command_id: &str, result_json: &str) -> Result<(), DomainError> {
        self.command_ledger_repo.complete(command_id, result_json)
    }

    async fn abandon(&self, command_id: &str) -> Result<(), DomainError> {
        self.command_ledger_repo.abandon(command_id)
    }
}

#[async_trait]
impl NotebookStoragePort for SqliteTaskStore {
    async fn list_notebook_cells(
        &self,
        session_id: &str,
    ) -> Result<Vec<NotebookCell>, DomainError> {
        self.research_repo.list_notebook_cells(session_id)
    }

    async fn list_notebook_cells_scoped(
        &self,
        session_id: &str,
        notebook_id: Option<&str>,
    ) -> Result<Vec<NotebookCell>, DomainError> {
        self.research_repo
            .list_notebook_cells_scoped(session_id, notebook_id)
    }

    async fn replace_notebook_cells(
        &self,
        session_id: &str,
        cells: &[NotebookCell],
    ) -> Result<(), DomainError> {
        self.research_repo.replace_notebook_cells(session_id, cells)
    }

    async fn replace_notebook_cells_scoped(
        &self,
        session_id: &str,
        notebook_id: Option<&str>,
        cells: &[NotebookCell],
    ) -> Result<(), DomainError> {
        self.research_repo
            .replace_notebook_cells_scoped(session_id, notebook_id, cells)
    }

    async fn save_notebook_cell(&self, cell: &NotebookCell) -> Result<(), DomainError> {
        self.research_repo.save_notebook_cell(cell)
    }

    async fn get_notebook_kernel_state(
        &self,
        session_id: &str,
    ) -> Result<NotebookKernelState, DomainError> {
        self.research_repo.get_notebook_kernel_state(session_id)
    }

    async fn get_notebook_kernel_state_scoped(
        &self,
        session_id: &str,
        notebook_id: Option<&str>,
    ) -> Result<NotebookKernelState, DomainError> {
        self.research_repo
            .get_notebook_kernel_state_scoped(session_id, notebook_id)
    }

    async fn save_notebook_kernel_state(
        &self,
        state: &NotebookKernelState,
    ) -> Result<(), DomainError> {
        self.research_repo.save_notebook_kernel_state(state)
    }
}

#[async_trait]
impl WorkspaceRepoPort for SqliteTaskStore {
    async fn save_workspace(&self, workspace: &ExecutionWorkspace) -> Result<(), DomainError> {
        self.workspace_repo.save_workspace(workspace)
    }

    async fn get_workspace(
        &self,
        id: &WorkspaceId,
    ) -> Result<Option<ExecutionWorkspace>, DomainError> {
        self.workspace_repo.get_workspace(id)
    }

    async fn list_workspaces(&self) -> Result<Vec<ExecutionWorkspace>, DomainError> {
        self.workspace_repo.list_workspaces()
    }

    async fn update_workspace_status(
        &self,
        id: &WorkspaceId,
        status: WorkspaceStatus,
    ) -> Result<(), DomainError> {
        self.workspace_repo.update_workspace_status(id, status)
    }

    async fn delete_workspace(&self, id: &WorkspaceId) -> Result<(), DomainError> {
        self.workspace_repo.delete_workspace(id)
    }
}

#[async_trait]
impl QuantPort for SqliteTaskStore {
    async fn save_tearsheet(
        &self,
        tearsheet: &custos_domain::quant::BacktestTearsheet,
        task_id: &str,
    ) -> Result<(), DomainError> {
        self.quant_repo.save_tearsheet(tearsheet, task_id).await
    }

    async fn get_latest_tearsheet(
        &self,
        strategy_id: &str,
    ) -> Result<Option<custos_domain::quant::BacktestTearsheet>, DomainError> {
        self.quant_repo.get_latest_tearsheet(strategy_id).await
    }

    async fn save_order_ticket(
        &self,
        ticket: &custos_domain::quant::OrderTicket,
        task_id: &str,
    ) -> Result<(), DomainError> {
        self.quant_repo.save_order_ticket(ticket, task_id).await
    }

    async fn list_order_tickets(
        &self,
        strategy_id: Option<&str>,
        task_id: Option<&str>,
    ) -> Result<Vec<custos_domain::quant::OrderTicket>, DomainError> {
        self.quant_repo
            .list_order_tickets(strategy_id, task_id)
            .await
    }

    async fn get_order_ticket(
        &self,
        order_id: &str,
    ) -> Result<Option<custos_domain::quant::OrderTicket>, DomainError> {
        self.quant_repo.get_order_ticket(order_id).await
    }

    async fn update_order_ticket(
        &self,
        ticket: &custos_domain::quant::OrderTicket,
    ) -> Result<(), DomainError> {
        self.quant_repo.update_order_ticket(ticket).await
    }

    async fn save_market_bars(
        &self,
        symbol: &str,
        timeframe: &str,
        bars: &[custos_domain::quant::OhlcvBar],
    ) -> Result<(), DomainError> {
        self.quant_repo
            .save_market_bars(symbol, timeframe, bars)
            .await
    }

    async fn get_market_bars(
        &self,
        symbol: &str,
        timeframe: &str,
        limit: usize,
    ) -> Result<Vec<custos_domain::quant::OhlcvBar>, DomainError> {
        self.quant_repo
            .get_market_bars(symbol, timeframe, limit)
            .await
    }
}
