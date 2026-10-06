//! Gate-A vertical slice proving the 6 ports are wired end-to-end in RAM:
//! Kernel -> Context Compiler -> Model -> Authority -> Sandbox -> Outbox -> Evidence -> Kernel
//! plus the negative paths that Custos.md's invariants demand.

use custos_core::context::{CompileContextRequest, ContextCompiler};
use custos_core::contracts::workflow::WorkflowPort;
use custos_core::contracts::{
    KernelPort, MemoryPort, OutboxEntry, OutboxPort, OutboxStatus, SandboxPort,
};
use custos_core::evidence::EvidenceBundle;
use custos_domain::{
    new_id, ActionIntent, ContractEvidence, EvidenceKind, FactProposal, MemoryScope,
    ProposalStatus, RecallQuery, RiskLevel, StartRunCommand, TaskContract, TaskStatus,
};
use custos_provider::request::ProviderRequest;
use custos_provider::ModelProvider;
use custos_testkit::Harness;

fn exec_intent(task_id: &str, risk: RiskLevel) -> ActionIntent {
    ActionIntent::new(
        new_id("act"),
        "sandbox_exec".into(),
        "workspace".into(),
        serde_json::json!({ "program": "cargo", "args": ["test"], "cwd": "." }),
        risk,
    )
    .with_task_id(task_id)
}

#[tokio::test]
async fn vertical_slice_links_all_ports() {
    let h = Harness::new();
    h.write_file(
        "src/lib.rs",
        "pub fn add(a: i32, b: i32) -> i32 { a + b }\n",
    );

    // 1. KernelPort: task Draft -> Queued -> Running
    let task = h
        .kernel
        .create_task("explain src/lib.rs".into())
        .await
        .unwrap();
    h.kernel
        .transition_task(&task.id, TaskStatus::Queued)
        .await
        .unwrap();
    h.kernel
        .transition_task(&task.id, TaskStatus::Running)
        .await
        .unwrap();

    // 2. StoragePort (via kernel): task is durable in SQLite
    assert!(h.kernel.get_task(&task.id).await.unwrap().is_some());

    // 3. Context Compiler (Vĩ): 8-step pipeline produces a sealed ContextPack
    let pack = ContextCompiler::new(&h.workspace)
        .unwrap()
        .compile(&CompileContextRequest {
            task_id: task.id.clone(),
            task_goal: "explain add".into(),
            candidate_files: vec!["src/lib.rs".into(), "../../etc/passwd".into()],
            max_tokens: 500,
            untrusted_sources: vec![],
        })
        .unwrap();
    assert_eq!(
        pack.items.len(),
        1,
        "traversal path must be dropped at step 1"
    );
    assert!(pack.context_digest.starts_with("sha256:"));

    // 4. ModelPort (Vĩ): context goes in, an intent proposal comes out
    h.model
        .push_reply(r#"{"program":"cargo","args":["test"],"cwd":"."}"#);
    let prompt = pack
        .items
        .iter()
        .map(|i| i.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let reply = h
        .model
        .generate(&ProviderRequest::new(
            new_id("req"),
            &task.id,
            1,
            prompt,
            "mock",
        ))
        .await
        .unwrap();
    assert!(reply.content.contains("cargo"));
    assert!(h.model.seen_prompts.lock().unwrap()[0].contains("pub fn add"));

    // 5. Authority (Vĩ): intent -> single-use permit bound to argument digest
    let intent = exec_intent(&task.id, RiskLevel::Medium);
    let permit = h
        .kernel
        .request_permit(&task.id, &intent, "worker")
        .await
        .unwrap();
    assert_eq!(permit.max_uses, 1);
    assert_eq!(permit.argument_digest, intent.argument_digest());

    // 6. Outbox (Trường, T3): durable record BEFORE the effect
    let entry_id = new_id("ob");
    h.outbox
        .enqueue(OutboxEntry {
            id: entry_id.clone(),
            task_id: task.id.clone(),
            action_id: intent.id.clone(),
            permit_id: permit.id.clone(),
            argument_digest: permit.argument_digest.clone(),
            idempotency_key: None,
            status: OutboxStatus::Pending,
            created_at: chrono::Utc::now(),
            receipt: None,
        })
        .await
        .unwrap();
    let burned = h.kernel.consume_permit(&permit.id, &intent).await.unwrap();
    h.outbox.mark_dispatching(&entry_id).await.unwrap();

    // 7. SandboxPort (Vinh): effect executes only with the burned, bound permit
    let receipt = h.sandbox.execute(&intent, &burned).await.unwrap();
    h.outbox
        .mark_receipted(&entry_id, receipt.clone())
        .await
        .unwrap();
    assert!(h
        .outbox
        .list_by_status(OutboxStatus::Dispatching)
        .await
        .unwrap()
        .is_empty());

    // 8. Evidence (Vĩ): deterministic verifier on the receipt; model's "done" is not evidence
    let claim = h
        .kernel
        .verify_evidence(&EvidenceBundle::new(
            &task.id,
            "command_exit_code",
            "Tests pass",
            receipt.output_data.clone().unwrap(),
        ))
        .await
        .unwrap();
    assert!(claim.passed);

    // 9. MemoryPort (Vĩ): model may only PROPOSE
    let r = h
        .memory
        .propose_fact(
            FactProposal {
                subject: "repo".into(),
                predicate: "has_fn".into(),
                object: "add".into(),
                confidence: 0.9,
                evidence_refs: vec![receipt.output_digest.clone()],
            },
            new_id("wrun"),
        )
        .await
        .unwrap();
    assert_eq!(r.status, ProposalStatus::Pending);
    assert!(
        h.memory
            .recall_context(
                &RecallQuery {
                    text: "add".into(),
                    limit: 5,
                    token_budget: None
                },
                &MemoryScope::default()
            )
            .await
            .unwrap()
            .is_empty(),
        "a proposal must not be recallable until the Kernel accepts it"
    );

    // 10. Completion Gate: only path to Succeeded; the claim is the proof
    let done = h
        .kernel
        .complete_task(&task.id, "add() explained; tests green".into(), vec![claim])
        .await
        .unwrap();
    assert_eq!(done.status, TaskStatus::Succeeded);
}

fn contract_requiring_test_result() -> TaskContract {
    TaskContract {
        pack_id: "engineering".into(),
        name: "fix_bug".into(),
        description: "requires passing tests".into(),
        required_capabilities: vec!["sandbox_exec".into()],
        evidence_requirements: vec![ContractEvidence {
            kind: EvidenceKind::TestResult,
            required: true,
        }],
    }
}

#[tokio::test]
async fn completion_is_refused_without_required_evidence() {
    let h = Harness::new();
    let task = h
        .kernel
        .create_task_with_contract("fix".into(), contract_requiring_test_result())
        .await
        .unwrap();
    h.kernel
        .transition_task(&task.id, TaskStatus::Queued)
        .await
        .unwrap();
    h.kernel
        .transition_task(&task.id, TaskStatus::Running)
        .await
        .unwrap();

    // Model says "done" with no proof -> refused (INV-02)
    assert!(h
        .kernel
        .complete_task(&task.id, "done".into(), vec![])
        .await
        .is_err());
    // A failing test run is not proof either
    let failing = h
        .kernel
        .verify_evidence(&EvidenceBundle::new(
            &task.id,
            "command_exit_code",
            "Tests pass",
            serde_json::json!({ "exit_code": 1 }),
        ))
        .await
        .unwrap();
    assert!(!failing.passed);
    assert!(h
        .kernel
        .complete_task(&task.id, "done".into(), vec![failing])
        .await
        .is_err());
    // A passing run is
    let passing = h
        .kernel
        .verify_evidence(&EvidenceBundle::new(
            &task.id,
            "command_exit_code",
            "Tests pass",
            serde_json::json!({ "exit_code": 0 }),
        ))
        .await
        .unwrap();
    assert!(h
        .kernel
        .complete_task(&task.id, "done".into(), vec![passing])
        .await
        .is_ok());
}

#[tokio::test]
async fn plain_transition_cannot_bypass_the_completion_gate() {
    let h = Harness::new();
    let task = h.kernel.create_task("t".into()).await.unwrap();
    h.kernel
        .transition_task(&task.id, TaskStatus::Queued)
        .await
        .unwrap();
    h.kernel
        .transition_task(&task.id, TaskStatus::Running)
        .await
        .unwrap();
    assert!(h
        .kernel
        .transition_task(&task.id, TaskStatus::Succeeded)
        .await
        .is_err());
}

#[tokio::test]
async fn permit_is_single_use() {
    let h = Harness::new();
    let task = h.kernel.create_task("t".into()).await.unwrap();
    let intent = exec_intent(&task.id, RiskLevel::Low);
    let permit = h
        .kernel
        .request_permit(&task.id, &intent, "w")
        .await
        .unwrap();
    h.kernel.consume_permit(&permit.id, &intent).await.unwrap();
    assert!(
        h.kernel.consume_permit(&permit.id, &intent).await.is_err(),
        "replay must fail"
    );
}

#[tokio::test]
async fn tampered_arguments_are_rejected() {
    let h = Harness::new();
    let task = h.kernel.create_task("t".into()).await.unwrap();
    let intent = exec_intent(&task.id, RiskLevel::Low);
    let permit = h
        .kernel
        .request_permit(&task.id, &intent, "w")
        .await
        .unwrap();

    let mut evil = intent.clone();
    evil.parameters = serde_json::json!({ "program": "rm", "args": ["-rf", "/"], "cwd": "." });
    assert!(
        h.kernel.consume_permit(&permit.id, &evil).await.is_err(),
        "kernel digest guard"
    );
    assert!(
        h.sandbox.execute(&evil, &permit).await.is_err(),
        "sandbox digest guard"
    );
    assert!(h.sandbox.executed.lock().unwrap().is_empty());
}

#[tokio::test]
async fn risky_actions_are_not_auto_permitted() {
    let h = Harness::new();
    let task = h.kernel.create_task("t".into()).await.unwrap();
    assert!(h
        .kernel
        .request_permit(&task.id, &exec_intent(&task.id, RiskLevel::High), "w")
        .await
        .is_err());
    assert!(h
        .kernel
        .request_permit(&task.id, &exec_intent(&task.id, RiskLevel::Critical), "w")
        .await
        .is_err());
}

#[tokio::test]
async fn illegal_task_transition_is_rejected() {
    let h = Harness::new();
    let task = h.kernel.create_task("t".into()).await.unwrap();
    assert!(
        h.kernel
            .transition_task(&task.id, TaskStatus::Succeeded)
            .await
            .is_err(),
        "Draft cannot jump to Succeeded"
    );
}

#[tokio::test]
async fn test_workflow_port_lifecycle() {
    let h = Harness::new();
    let task = h
        .kernel
        .create_task("run workflow lifecycle".into())
        .await
        .unwrap();

    // 1. start_run via WorkflowPort
    let cmd = StartRunCommand::new(&task.id, "worker_1");
    let handle = h.workflow.start_run(cmd).await.unwrap();
    assert_eq!(handle.task_id, task.id);
    assert_eq!(handle.status, custos_domain::RunStatus::Active);

    // 2. checkpoint
    let cp = h.workflow.checkpoint(&handle.run_id).await.unwrap();
    assert_eq!(cp.task_id, "task_mock");

    // 3. cancel
    let cancel = h
        .workflow
        .request_cancel(&handle.run_id, "test abort")
        .await
        .unwrap();
    assert_eq!(cancel.run_id, handle.run_id);
    assert_eq!(cancel.reason, "test abort");
}
