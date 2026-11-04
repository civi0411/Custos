use super::*;
    use custos_core::kernel::{AdvanceTask, CreateTask, TaskReducer, TaskService};
    use custos_domain::{ContractEvidence, EvidenceKind, SpanState, TaskContract, TaskStatus};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_task_crud_and_list() {
        let store = SqliteTaskStore::new_in_memory().unwrap();
        let task = Task::new("test_task_1".to_string(), "Test Task 1".to_string());

        store.save_task(&task).await.unwrap();

        let retrieved = store.get_task(&task.id).await.unwrap().unwrap();
        assert_eq!(retrieved.id, task.id);
        assert_eq!(retrieved.title, "Test Task 1");
        assert_eq!(retrieved.status, TaskStatus::Draft);

        let all = store.list_tasks().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, task.id);
    }

    #[tokio::test]
    async fn task_contract_survives_persistence_and_list() {
        let store = SqliteTaskStore::new_in_memory().unwrap();
        let mut task = Task::new("task_contract".into(), "Contract test".into());
        task.contract = Some(TaskContract {
            pack_id: "engineering".into(),
            name: "Contract test".into(),
            description: "Preserve contract through SQLite".into(),
            required_capabilities: vec!["fs_read".into(), "fs_write".into()],
            evidence_requirements: vec![ContractEvidence {
                kind: EvidenceKind::TestResult,
                required: true,
            }],
        });

        store.save_task(&task).await.unwrap();
        let loaded = store.get_task(&task.id).await.unwrap().unwrap();
        assert_eq!(loaded.contract.as_ref().unwrap().pack_id, "engineering");
        assert_eq!(
            loaded
                .contract
                .as_ref()
                .unwrap()
                .required_capabilities
                .len(),
            2
        );

        let listed = store.list_tasks().unwrap();
        assert_eq!(
            listed[0].contract.as_ref().unwrap().description,
            "Preserve contract through SQLite"
        );
    }

    #[tokio::test]
    async fn task_events_commit_with_snapshot_and_replay_exact_state() {
        let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
        let service = TaskService::new(store.clone());
        let (created, created_event) = service
            .execute_create(CreateTask {
                title: "Replay task".into(),
                metadata: Some(serde_json::json!({"origin": "test"})),
                contract: Some(TaskContract {
                    pack_id: "engineering".into(),
                    name: "Replay task".into(),
                    description: "Rebuild from events".into(),
                    required_capabilities: vec!["fs_read".into()],
                    evidence_requirements: vec![ContractEvidence {
                        kind: EvidenceKind::TestResult,
                        required: true,
                    }],
                }),
            })
            .await
            .unwrap();
        service
            .execute_advance(AdvanceTask {
                task_id: created.id.clone(),
                expected_epoch: created.epoch,
                next_status: TaskStatus::Queued,
                rationale: Some("test replay".into()),
            })
            .await
            .unwrap();

        let events = store.get_task_events(&created.id).await.unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], TaskEvent::Created(_)));
        assert!(matches!(events[1], TaskEvent::Advanced(_)));

        let replayed = TaskReducer::replay(&events).unwrap();
        let persisted = store.get_task(&created.id).await.unwrap().unwrap();
        assert_eq!(replayed.id, persisted.id);
        assert_eq!(replayed.status, persisted.status);
        assert_eq!(replayed.epoch, persisted.epoch);
        assert_eq!(replayed.metadata, persisted.metadata);
        assert_eq!(replayed.contract.unwrap().pack_id, "engineering");

        let same_event = TaskEvent::Created(created_event);
        store
            .commit_task_event(&same_event, &created)
            .await
            .unwrap();
        assert!(store.get_task_events(&created.id).await.unwrap().len() == 2);
    }

    #[tokio::test]
    async fn test_span_lifecycle() {
        let store = SqliteTaskStore::new_in_memory().unwrap();
        let task = Task::new("test_task_2".to_string(), "Test Task 2".to_string());
        store.save_task(&task).await.unwrap();

        let span1 = Span::new(
            "span_1".to_string(),
            task.id.clone(),
            1,
            "test-provider".to_string(),
            "test-model".to_string(),
            "sha256:abc".to_string(),
        );
        store.save_span(&span1).await.unwrap();

        let retrieved = store.get_span(&span1.id).await.unwrap().unwrap();
        assert_eq!(retrieved.id, span1.id);
        assert_eq!(retrieved.span_num, 1);
        assert_eq!(retrieved.state, SpanState::Started);

        let spans = store.list_spans(&task.id).await.unwrap();
        assert_eq!(spans.len(), 1);
    }

    #[tokio::test]
    async fn test_continuation_packets() {
        let store = SqliteTaskStore::new_in_memory().unwrap();
        let task = Task::new("test_task_3".to_string(), "Test Task 3".to_string());
        store.save_task(&task).await.unwrap();

        let packet = ContinuationPacket::create(
            task.id.clone(),
            1,
            2,
            "test-provider".to_string(),
            "test-model".to_string(),
            "Summary of task 3".to_string(),
            serde_json::json!({"step": 2}),
        )
        .unwrap();
        store.save_continuation(&packet).await.unwrap();

        let retrieved = store
            .get_latest_continuation(&task.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.task_id, task.id);
        assert_eq!(retrieved.to_span, 2);
        assert_eq!(retrieved.task_summary, "Summary of task 3");
        assert!(retrieved.verify().is_ok());
    }

    #[tokio::test]
    async fn test_foreign_key_enforcement() {
        let store = SqliteTaskStore::new_in_memory().unwrap();
        // span referring to non-existent task must fail due to foreign keys = ON
        let span = Span::new(
            "span_orphan".to_string(),
            "non_existent_task".to_string(),
            1,
            "test-provider".to_string(),
            "test-model".to_string(),
            "sha256:abc".to_string(),
        );
        let result = store.save_span(&span).await;
        assert!(
            result.is_err(),
            "Foreign key constraint must prevent span creation for missing task"
        );
    }

    #[tokio::test]
    async fn test_session_store_crud_and_journal() {
        use custos_domain::{Session, SessionId, SessionJournalEntry, SessionMode, SessionStatus};

        let store = SqliteTaskStore::new_in_memory().unwrap();
        let session_id = SessionId("ses_test_1".into());
        let session = Session::new(session_id.clone(), SessionMode::Bare);

        // 1. Save session
        store.save_session(&session).await.unwrap();

        // 2. Retrieve session
        let loaded = store.get_session(&session_id).await.unwrap().unwrap();
        assert_eq!(loaded.id, session_id);
        assert_eq!(loaded.status, SessionStatus::Active);

        // 3. Append journal entries
        let entry1 = SessionJournalEntry {
            entry_id: None,
            session_id: session_id.clone(),
            entry_type: "user_message".into(),
            entry_data: "Hello Custos".into(),
            occurred_at: chrono::Utc::now().to_rfc3339(),
        };
        let row_id = store.append_journal(&entry1).await.unwrap();
        assert!(row_id > 0);

        let entry2 = SessionJournalEntry {
            entry_id: None,
            session_id: session_id.clone(),
            entry_type: "assistant_message".into(),
            entry_data: "Hello! How can I assist you?".into(),
            occurred_at: chrono::Utc::now().to_rfc3339(),
        };
        store.append_journal(&entry2).await.unwrap();

        // 4. Retrieve journal
        let journal = store.get_journal(&session_id).await.unwrap();
        assert_eq!(journal.len(), 2);
        assert_eq!(journal[0].entry_type, "user_message");
        assert_eq!(journal[0].entry_data, "Hello Custos");
        assert_eq!(journal[1].entry_type, "assistant_message");

        // 5. Update session status and goal
        let mut updated_session = loaded;
        updated_session.current_goal = Some("Refactor codebase".into());
        updated_session.promotion_score = 0.45;
        updated_session.status = SessionStatus::Paused;
        store.save_session(&updated_session).await.unwrap();

        let reloaded = store.get_session(&session_id).await.unwrap().unwrap();
        assert_eq!(reloaded.current_goal.as_deref(), Some("Refactor codebase"));
        assert_eq!(reloaded.promotion_score, 0.45);
        assert_eq!(reloaded.status, SessionStatus::Paused);

        // 6. List sessions
        let list = store.list_sessions().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, session_id);

        // 7. Delete session and canonical journal without touching other stores
        let deleted = store.delete_session(&session_id).await.unwrap();
        assert!(deleted);
        assert!(store.get_session(&session_id).await.unwrap().is_none());
        assert!(store.get_journal(&session_id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_run_and_worker_run_persistence() {
        use custos_domain::{Run, RunStatus, WorkerRun};

        let store = SqliteTaskStore::new_in_memory().unwrap();
        let task = Task::new("task_run_1".into(), "Run Test Task".into());
        store.save_task(&task).await.unwrap();

        // 1. Create and save Run
        let mut run = Run::new(task.id.clone(), 1);
        run.workflow_revision = Some("rev-1".into());
        run.transition(RunStatus::Active).unwrap();
        store.save_run(&mut run).await.unwrap();

        // 2. Query Run
        let loaded = store.get_run(&run.id).await.unwrap().unwrap();
        assert_eq!(loaded.id, run.id);
        assert_eq!(loaded.task_id, task.id);
        assert_eq!(loaded.status, RunStatus::Active);
        assert_eq!(loaded.workflow_revision.as_deref(), Some("rev-1"));

        // 3. Create and save WorkerRun
        let mut wrun = WorkerRun::new(task.id.clone(), "worker_alpha".into(), 3);
        wrun.run_id = Some(run.id.clone());
        wrun.transition(RunStatus::Active).unwrap();
        store.save_worker_run(&mut wrun).await.unwrap();

        // 4. Query WorkerRun
        let loaded_wrun = store.get_worker_run(&wrun.id).await.unwrap().unwrap();
        assert_eq!(loaded_wrun.id, wrun.id);
        assert_eq!(loaded_wrun.run_id.as_deref(), Some(run.id.as_str()));
        assert_eq!(loaded_wrun.status, RunStatus::Active);
        assert_eq!(loaded_wrun.worker_id, "worker_alpha");

        // 5. Update Run status to Completed
        run.transition(RunStatus::Completed).unwrap();
        store.save_run(&mut run).await.unwrap();
        let updated_run = store.get_run(&run.id).await.unwrap().unwrap();
        assert_eq!(updated_run.status, RunStatus::Completed);
        assert!(updated_run.ended_at.is_some());

        // 6. List runs and worker runs
        let runs = store.list_runs_for_task(&task.id).await.unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].id, run.id);

        let wruns = store.list_worker_runs_for_task(&task.id).await.unwrap();
        assert_eq!(wruns.len(), 1);
        assert_eq!(wruns[0].id, wrun.id);

        let wruns_by_run = store.list_worker_runs_for_run(&run.id).await.unwrap();
        assert_eq!(wruns_by_run.len(), 1);
        assert_eq!(wruns_by_run[0].id, wrun.id);
    }

    #[tokio::test]
    async fn test_workflow_revision_and_replan_ports() {
        use custos_domain::{ReplanBrief, ReplanTrigger, RevisionNode};

        let store = SqliteTaskStore::new_in_memory().unwrap();
        let task = Task::new("task_store_rev_1".to_string(), "Rev Test".to_string());
        store.save_task(&task).await.unwrap();

        // 1. Save and query WorkflowRevision
        let mut revision = WorkflowRevision::new(&task.id, "prop_oi_01", 1);
        revision.nodes.push(RevisionNode {
            node_id: "node_1".into(),
            step_name: "Inspect".into(),
            role: "worker".into(),
            harness_id: "claude_code".into(),
            allocated_budget_tokens: 3000,
            read_set: vec!["file.txt".into()],
            write_set: vec![],
            required_capabilities: vec!["fs_read".into()],
        });

        let placements = vec![NodePlacement::new("node_1", "worker", "claude_code", 3000)];

        store.save_revision(&revision, &placements).await.unwrap();

        let loaded_rev = store
            .get_revision(&revision.revision_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded_rev.revision_id, revision.revision_id);
        assert_eq!(loaded_rev.nodes.len(), 1);

        let loaded_placements = store
            .get_placements_for_revision(&revision.revision_id)
            .await
            .unwrap();
        assert_eq!(loaded_placements.len(), 1);
        assert_eq!(loaded_placements[0].budget_tokens_slice, 3000);

        let revs = store.list_revisions_for_task(&task.id).await.unwrap();
        assert_eq!(revs.len(), 1);

        // 2. Save and query ReplanRecord
        let brief = ReplanBrief::new(
            &task.id,
            ReplanTrigger::SourceDrift,
            None,
            "Files changed on disk",
        );
        let replan = ReplanRecord::new(&task.id, brief, "prop_oi_02");
        store.record_replan(&replan).await.unwrap();

        let loaded_replan = store.get_replan(&replan.id).await.unwrap().unwrap();
        assert_eq!(loaded_replan.id, replan.id);
        assert_eq!(loaded_replan.brief.trigger, ReplanTrigger::SourceDrift);

        let replans = store.list_replans_for_task(&task.id).await.unwrap();
        assert_eq!(replans.len(), 1);
        assert_eq!(replans[0].id, replan.id);
    }

    #[tokio::test]
    async fn test_execution_workspace_persistence() {
        use custos_core::contracts::workspace::WorkspaceRepository as _;
        use custos_domain::{ExecutionWorkspace, WorkspaceId, WorkspaceKind, WorkspaceStatus};

        let store = SqliteTaskStore::new_in_memory().unwrap();
        let ws_id = WorkspaceId::generate();

        let ws = ExecutionWorkspace::new(
            ws_id.clone(),
            "orca-worktree-feat",
            WorkspaceKind::Git {
                repo_path: "/workspace/custos".into(),
                branch: "feat/orca".into(),
                base_commit: Some("c0ffee".into()),
            },
            "/workspace/custos/.worktrees/orca",
        )
        .with_metadata(serde_json::json!({
            "domain": "engineering",
            "workbench": "coding"
        }));

        // 1. Save workspace
        store.save_workspace(&ws).await.unwrap();

        // 2. Get workspace
        let loaded = store.get_workspace(&ws_id).await.unwrap().unwrap();
        assert_eq!(loaded.id, ws_id);
        assert_eq!(loaded.name, "orca-worktree-feat");
        assert_eq!(loaded.status, WorkspaceStatus::Initializing);
        assert_eq!(loaded.metadata["domain"], "engineering");

        // 3. Update status
        store
            .update_workspace_status(&ws_id, WorkspaceStatus::Ready)
            .await
            .unwrap();

        let updated = store.get_workspace(&ws_id).await.unwrap().unwrap();
        assert_eq!(updated.status, WorkspaceStatus::Ready);

        // 4. List workspaces
        let list = store.list_workspaces().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, ws_id);

        // 5. Delete workspace
        store.delete_workspace(&ws_id).await.unwrap();
        let deleted = store.get_workspace(&ws_id).await.unwrap();
        assert!(deleted.is_none());
    }
