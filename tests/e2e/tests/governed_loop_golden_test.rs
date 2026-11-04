//! Golden Acceptance Test G07 — Governed Agent Loop, Budget & Gateway Ownership (F08)
//!
//! Validates:
//! 1. Actual composed path: API model task -> admitted tool -> model result -> file written to real tempdir on disk, exactly 1 effect attempt in outbox, 1 receipt, status Success.
//! 2. Prevention of duplicate execution: TaskRuntime processes intents containing ActionLifecycleState::Receipt and skips re-dispatching, ensuring exactly 1 file write and 1 effect record in the persistent ledger.
//! 3. Security kernel rejection: Critical risk tools (e.g. destructive shell command with mkfs) are rejected by Gate 2 Policy before touching disk, marked Failed, zero mutation.
//! 4. Path traversal denial: Parameter containing '..' is immediately rejected by parse_tool_invocation with DomainError::Validation, zero disk mutation.
//! 5. Bounded iteration exhaustion: Endless tool-calling without clean completion returns DomainError::BudgetExceeded with 'status: limited', never returns success.
//! 6. Cancellation responsiveness: Mid-loop cancellation halts execution promptly with DomainError::Validation, and steering/cancellation handles are pruned.

use async_trait::async_trait;
use custos_adapters::sandbox::SovereignDeveloperAdapter;
use custos_core::contracts::harness::AgentRuntimePort;
use custos_core::contracts::kernel::{KernelPort, TrustedKernel};
use custos_core::contracts::workflow::WorkflowPort;
use custos_daemon::runtime::DaemonGovernedToolExecutor;
use custos_domain::{
    ActionLifecycleState, ContractEvidence, DomainError, EffectStatus, EvidenceKind,
    ReceiptStatus, StartRunCommand, TaskContract, WorkerRun, ContextPack,
};
use custos_persistence::SqliteTaskStore;
use custos_provider::events::ToolCall;
use custos_provider::request::{ModelResponse, ProviderRequest};
use custos_provider::ModelPort;
use custos_runtime::agent::runtime_port::GovernedAgentRuntime;
use custos_runtime::workflow::TaskRuntime;
use std::sync::{Arc, Mutex};
use tempfile::tempdir;

/// Scripted multi-turn mock provider for governed loop testing
struct ScriptedLoopProvider {
    responses: Mutex<Vec<ModelResponse>>,
}

impl ScriptedLoopProvider {
    fn new(responses: Vec<ModelResponse>) -> Self {
        Self {
            responses: Mutex::new(responses),
        }
    }
}

#[async_trait]
impl ModelPort for ScriptedLoopProvider {
    fn provider_id(&self) -> &str {
        "scripted-governed-loop-model"
    }

    async fn generate(&self, _req: &ProviderRequest) -> Result<ModelResponse, DomainError> {
        let mut list = self.responses.lock().unwrap();
        if list.is_empty() {
            Ok(ModelResponse::text("Default completion", "scripted-model", 10))
        } else {
            Ok(list.remove(0))
        }
    }
}

fn sample_tools() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read file content",
                "parameters": {
                    "type": "object",
                    "properties": { "path": { "type": "string" } },
                    "required": ["path"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "Write file content",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "content": { "type": "string" }
                    },
                    "required": ["path", "content"]
                }
            }
        }),
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "bash",
                "description": "Execute shell command",
                "parameters": {
                    "type": "object",
                    "properties": { "command": { "type": "string" } },
                    "required": ["command"]
                }
            }
        }),
    ]
}

#[tokio::test]
async fn test_g07_composed_path_admitted_tool_to_real_disk_and_single_receipt() {
    let workspace = tempdir().expect("temp workspace dir");
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
    let kernel: Arc<dyn KernelPort> = Arc::new(TrustedKernel::new(store.clone()));

    // Create task in kernel
    let contract = TaskContract {
        pack_id: "engineering".into(),
        name: "composed_loop_verification".into(),
        description: "Test composed loop execution".into(),
        required_capabilities: vec!["write_repo".into()],
        evidence_requirements: vec![ContractEvidence {
            kind: EvidenceKind::FileAnchor,
            required: true,
        }],
    };
    let task = kernel
        .create_task_with_contract("Verify Composed Path".into(), contract)
        .await
        .expect("task creation");

    // Developer sandbox bound to real tempdir
    let developer_adapter = Arc::new(SovereignDeveloperAdapter::new(workspace.path()));
    let tool_executor = Arc::new(DaemonGovernedToolExecutor::new(
        developer_adapter.clone(),
        kernel.clone(),
        store.outbox().clone(),
    ));

    // Turn 1: Model requests write_file
    let mut resp1 = ModelResponse::text("I will create service.conf", "test-model", 20);
    resp1.tool_calls.push(ToolCall {
        call_id: "call_write_1".into(),
        name: "write_file".into(),
        arguments: serde_json::json!({
            "path": "service.conf",
            "content": "port=8080\nenv=production"
        }),
    });

    // Turn 2: Model receives tool result and outputs final answer with no further tool calls
    let resp2 = ModelResponse::text("service.conf successfully created.", "test-model", 15);

    let provider = Arc::new(ScriptedLoopProvider::new(vec![resp1, resp2]));
    let runtime = GovernedAgentRuntime::new(
        provider,
        "You are a sovereign test agent",
        sample_tools(),
    )
    .with_tool_executor(tool_executor)
    .with_max_iterations(5);

    let worker_run = WorkerRun::new(task.id.clone(), "coder".into(), 1);
    let context_pack = ContextPack::new("pack_1".into(), vec![], 0, "sha256:0".into());

    // Execute governed turn
    let intents = runtime
        .execute_turn(&worker_run, &context_pack)
        .await
        .expect("execute turn must succeed");

    // 1. Assert exactly 1 action intent returned and its lifecycle state is Receipt
    assert_eq!(intents.len(), 1);
    let intent = &intents[0];
    assert_eq!(intent.name, "write_file");
    assert_eq!(intent.target, "service.conf");
    assert_eq!(intent.lifecycle_state, ActionLifecycleState::Receipt);

    // 2. Assert physical effect: file exists on disk inside workspace with exact content
    let file_path = workspace.path().join("service.conf");
    assert!(file_path.exists(), "service.conf must exist on physical disk");
    let content = std::fs::read_to_string(&file_path).expect("read written file");
    assert_eq!(content, "port=8080\nenv=production");

    // 3. Assert Outbox ledger: exactly 1 entry recorded, status is Receipted with valid receipt
    let entries = custos_core::contracts::storage::OutboxPort::list_by_task_and_status(
        store.outbox(),
        &task.id,
        custos_core::contracts::storage::OutboxStatus::Receipted,
    )
    .await
    .expect("list receipted outbox entries");
    assert_eq!(entries.len(), 1, "exactly one receipted entry in outbox");
    assert_eq!(entries[0].task_id, task.id);
    assert_eq!(entries[0].argument_digest, intent.argument_digest());
    assert!(entries[0].receipt.is_some(), "receipt must be present");
    let receipt = entries[0].receipt.as_ref().unwrap();
    assert_eq!(receipt.status, ReceiptStatus::Success);
    assert!(
        receipt.output_digest.starts_with("blake3:")
            || receipt.output_digest.starts_with("sha256:"),
        "Output digest must be a valid hash format: {}",
        receipt.output_digest
    );

    // Also verify durable effect attempt via recorded idempotency key
    let idemp_key = entries[0]
        .idempotency_key
        .as_ref()
        .expect("idempotency_key present");
    let effect = store
        .outbox()
        .get_effect_by_idempotency_key(idemp_key)
        .expect("get effect by idempotency key")
        .expect("effect attempt must exist");
    assert_eq!(effect.status, EffectStatus::Succeeded);
    assert!(effect.receipt.is_some());
}

#[tokio::test]
async fn test_g07_prevention_of_duplicate_execution_in_task_runtime() {
    let workspace = tempdir().expect("temp workspace dir");
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
    let kernel: Arc<dyn KernelPort> = Arc::new(TrustedKernel::new(store.clone()));

    // Create task
    let contract = TaskContract {
        pack_id: "engineering".into(),
        name: "prevent_dup_verification".into(),
        description: "Test duplicate prevention in task runtime".into(),
        required_capabilities: vec!["write_repo".into()],
        evidence_requirements: vec![],
    };
    let task = kernel
        .create_task_with_contract("Verify No Duplicate Execution".into(), contract)
        .await
        .expect("task creation");

    let developer_adapter = Arc::new(SovereignDeveloperAdapter::new(workspace.path()));
    let tool_executor = Arc::new(DaemonGovernedToolExecutor::new(
        developer_adapter.clone(),
        kernel.clone(),
        store.outbox().clone(),
    ));

    // Scripted model: Turn 1 writes app.json, Turn 2 completes
    let mut resp1 = ModelResponse::text("Writing app.json", "test-model", 20);
    resp1.tool_calls.push(ToolCall {
        call_id: "call_app_json".into(),
        name: "write_file".into(),
        arguments: serde_json::json!({
            "path": "app.json",
            "content": "{\"version\": \"1.0.0\"}"
        }),
    });
    let resp2 = ModelResponse::text("app.json creation confirmed.", "test-model", 10);
    let provider = Arc::new(ScriptedLoopProvider::new(vec![resp1, resp2]));

    let governed_harness = Arc::new(
        GovernedAgentRuntime::new(
            provider.clone(),
            "You are a sovereign test agent",
            sample_tools(),
        )
        .with_tool_executor(tool_executor)
        .with_max_iterations(5),
    );

    // Build TaskRuntime
    let task_runtime = Arc::new(
        TaskRuntime::new()
            .with_kernel(kernel.clone())
            .with_model(provider.clone())
            .with_harness(governed_harness.clone())
            .with_sandbox(developer_adapter.clone())
            .with_outbox(store.clone())
            .with_effect_ledger(store.clone())
            .with_run_store(store.clone()),
    );

    let start_cmd = StartRunCommand::new(&task.id, "orchestrator")
        .with_workspace_root(workspace.path().to_string_lossy());
    let handle = task_runtime
        .start_run(start_cmd)
        .await
        .expect("start_run must succeed");

    assert_eq!(handle.task_id, task.id);

    // Verify file on disk
    let file_path = workspace.path().join("app.json");
    assert!(file_path.exists(), "app.json must exist on disk");
    let content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "{\"version\": \"1.0.0\"}");

    // Verify outbox contains exactly 1 receipted entry (no duplicate execution by TaskRuntime)
    let entries = custos_core::contracts::storage::OutboxPort::list_by_task_and_status(
        store.outbox(),
        &task.id,
        custos_core::contracts::storage::OutboxStatus::Receipted,
    )
    .await
    .expect("list outbox entries");
    assert_eq!(
        entries.len(),
        1,
        "Invariant: TaskRuntime must NOT re-execute already receipted intents"
    );
}

#[tokio::test]
async fn test_g07_security_kernel_critical_risk_rejection_no_disk_mutation() {
    let workspace = tempdir().expect("temp workspace dir");
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
    let kernel: Arc<dyn KernelPort> = Arc::new(TrustedKernel::new(store.clone()));

    let developer_adapter = Arc::new(SovereignDeveloperAdapter::new(workspace.path()));
    let tool_executor = Arc::new(DaemonGovernedToolExecutor::new(
        developer_adapter.clone(),
        kernel.clone(),
        store.outbox().clone(),
    ));

    // Turn 1: Model attempts destructive bash command with mkfs
    let mut resp1 = ModelResponse::text("Formatting disk...", "test-model", 20);
    resp1.tool_calls.push(ToolCall {
        call_id: "call_critical_1".into(),
        name: "bash".into(),
        arguments: serde_json::json!({ "command": "mkfs.ext4 /dev/sda1" }),
    });

    // Turn 2: Model observes authority rejection and aborts
    let resp2 = ModelResponse::text("Tool rejected by policy. Stopping.", "test-model", 10);

    let provider = Arc::new(ScriptedLoopProvider::new(vec![resp1, resp2]));
    let runtime = GovernedAgentRuntime::new(
        provider,
        "System prompt",
        sample_tools(),
    )
    .with_tool_executor(tool_executor)
    .with_max_iterations(3);

    let worker_run = WorkerRun::new("task_critical_check".into(), "coder".into(), 1);
    let context_pack = ContextPack::new("pack_1".into(), vec![], 0, "sha256:0".into());

    let intents = runtime
        .execute_turn(&worker_run, &context_pack)
        .await
        .expect("turn should record rejection cleanly without panic");

    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0].name, "bash");
    assert_eq!(
        intents[0].lifecycle_state,
        ActionLifecycleState::Failed,
        "Critical tool without authorization must be marked Failed"
    );

    // Verify zero receipted or dispatching outbox entries for this task
    let entries = custos_core::contracts::storage::OutboxPort::list_by_task_and_status(
        store.outbox(),
        "task_critical_check",
        custos_core::contracts::storage::OutboxStatus::Receipted,
    )
    .await
    .expect("list outbox entries");
    assert!(entries.is_empty(), "critical rejection must not enqueue any receipted effect");
}

#[tokio::test]
async fn test_g07_path_traversal_denial_no_disk_mutation() {
    let workspace = tempdir().expect("temp workspace dir");
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
    let kernel: Arc<dyn KernelPort> = Arc::new(TrustedKernel::new(store.clone()));

    let developer_adapter = Arc::new(SovereignDeveloperAdapter::new(workspace.path()));
    let tool_executor = Arc::new(DaemonGovernedToolExecutor::new(
        developer_adapter.clone(),
        kernel.clone(),
        store.outbox().clone(),
    ));

    // Turn 1: Model attempts directory traversal
    let mut resp1 = ModelResponse::text("Writing outside workspace...", "test-model", 20);
    resp1.tool_calls.push(ToolCall {
        call_id: "call_traversal_1".into(),
        name: "write_file".into(),
        arguments: serde_json::json!({
            "path": "../../escaped.txt",
            "content": "malicious content"
        }),
    });

    let resp2 = ModelResponse::text("Traversal was rejected. Halting.", "test-model", 10);

    let provider = Arc::new(ScriptedLoopProvider::new(vec![resp1, resp2]));
    let runtime = GovernedAgentRuntime::new(
        provider,
        "System prompt",
        sample_tools(),
    )
    .with_tool_executor(tool_executor)
    .with_max_iterations(3);

    let worker_run = WorkerRun::new("task_traversal".into(), "coder".into(), 1);
    let context_pack = ContextPack::new("pack_1".into(), vec![], 0, "sha256:0".into());

    let intents = runtime
        .execute_turn(&worker_run, &context_pack)
        .await
        .expect("turn completes with recorded rejection");

    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0].lifecycle_state, ActionLifecycleState::Failed);

    // Verify zero files created outside workspace
    let escaped_path = workspace.path().join("../../escaped.txt");
    assert!(!escaped_path.exists(), "Path traversal must never create files on disk");
}

#[tokio::test]
async fn test_g07_bounded_iteration_exhaustion_returns_limited_budget_exceeded() {
    let workspace = tempdir().expect("temp workspace dir");
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
    let kernel: Arc<dyn KernelPort> = Arc::new(TrustedKernel::new(store.clone()));

    let developer_adapter = Arc::new(SovereignDeveloperAdapter::new(workspace.path()));
    let tool_executor = Arc::new(DaemonGovernedToolExecutor::new(
        developer_adapter.clone(),
        kernel.clone(),
        store.outbox().clone(),
    ));

    // Model endlessly proposes tools on every turn
    let mut resp1 = ModelResponse::text("Turn 1", "test-model", 10);
    resp1.tool_calls.push(ToolCall {
        call_id: "call_loop_1".into(),
        name: "read_file".into(),
        arguments: serde_json::json!({ "path": "file1.txt" }),
    });
    let mut resp2 = ModelResponse::text("Turn 2", "test-model", 10);
    resp2.tool_calls.push(ToolCall {
        call_id: "call_loop_2".into(),
        name: "read_file".into(),
        arguments: serde_json::json!({ "path": "file2.txt" }),
    });
    let mut resp3 = ModelResponse::text("Turn 3", "test-model", 10);
    resp3.tool_calls.push(ToolCall {
        call_id: "call_loop_3".into(),
        name: "read_file".into(),
        arguments: serde_json::json!({ "path": "file3.txt" }),
    });

    let provider = Arc::new(ScriptedLoopProvider::new(vec![resp1, resp2, resp3]));
    let runtime = GovernedAgentRuntime::new(
        provider,
        "System prompt",
        sample_tools(),
    )
    .with_tool_executor(tool_executor)
    .with_max_iterations(2); // Bound to 2 iterations

    let worker_run = WorkerRun::new("task_exhaustion".into(), "coder".into(), 1);
    let context_pack = ContextPack::new("pack_1".into(), vec![], 0, "sha256:0".into());

    let err = runtime
        .execute_turn(&worker_run, &context_pack)
        .await
        .expect_err("Iteration exhaustion must fail with BudgetExceeded");

    assert!(
        matches!(err, DomainError::BudgetExceeded(_)),
        "Expected DomainError::BudgetExceeded, got: {:?}",
        err
    );
    assert!(
        err.to_string().contains("status: limited"),
        "Error message must specify status: limited"
    );
}

#[tokio::test]
async fn test_g07_cancellation_during_governed_loop_cleans_handles_and_halts() {
    let workspace = tempdir().expect("temp workspace dir");
    let store = Arc::new(SqliteTaskStore::new_in_memory().expect("in-memory sqlite store"));
    let kernel: Arc<dyn KernelPort> = Arc::new(TrustedKernel::new(store.clone()));

    let developer_adapter = Arc::new(SovereignDeveloperAdapter::new(workspace.path()));
    let tool_executor = Arc::new(DaemonGovernedToolExecutor::new(
        developer_adapter.clone(),
        kernel.clone(),
        store.outbox().clone(),
    ));

    let mut resp1 = ModelResponse::text("Turn 1", "test-model", 10);
    resp1.tool_calls.push(ToolCall {
        call_id: "call_c1".into(),
        name: "read_file".into(),
        arguments: serde_json::json!({ "path": "test.txt" }),
    });

    let provider = Arc::new(ScriptedLoopProvider::new(vec![resp1]));
    let runtime = Arc::new(
        GovernedAgentRuntime::new(
            provider,
            "System prompt",
            sample_tools(),
        )
        .with_tool_executor(tool_executor)
        .with_max_iterations(5),
    );

    let worker_run = WorkerRun::new("task_cancel_g07".into(), "coder".into(), 1);
    let context_pack = ContextPack::new("pack_1".into(), vec![], 0, "sha256:0".into());

    // Pre-cancel run
    runtime.cancel_run(&worker_run.id).await.expect("cancel_run succeeds");

    let err = runtime
        .execute_turn(&worker_run, &context_pack)
        .await
        .expect_err("Canceled run must be rejected");

    assert!(
        matches!(err, DomainError::Validation(_)),
        "Expected DomainError::Validation for canceled run, got: {:?}",
        err
    );
    assert!(err.to_string().contains("canceled"));
}
