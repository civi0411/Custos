use super::*;
use async_trait::async_trait;
use custos_domain::StartRunCommand;
use custos_provider::request::ProviderRequest;

struct SlowStreamModel;
#[async_trait]
impl ModelPort for SlowStreamModel {
    fn provider_id(&self) -> &str {
        "slow"
    }
    async fn generate(
        &self,
        _: &ProviderRequest,
    ) -> Result<custos_provider::ModelResponse, DomainError> {
        Err(DomainError::Validation("Streaming only".into()))
    }
    async fn stream(
        &self,
        req: &ProviderRequest,
    ) -> Result<tokio::sync::mpsc::Receiver<custos_provider::ProviderEvent>, DomainError> {
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        let id = req.request_id.clone();
        tokio::spawn(async move {
            tx.send(custos_provider::ProviderEvent::chunk(&id, "partial"))
                .await
                .unwrap();
            tokio::select! {_=tx.closed()=>{},_=tokio::time::sleep(std::time::Duration::from_secs(30))=>{}}
        });
        Ok(rx)
    }
}
#[tokio::test]
async fn background_returns_early_and_stop_preserves_partial() {
    let runtime = TaskRuntime::new().with_model(Arc::new(SlowStreamModel));
    let cmd = StartRunCommand::new("background-task", "chat")
        .with_preferred_mode("model")
        .with_prompt("hello")
        .with_session_id("session")
        .with_turn_id("turn");
    let handle = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        runtime.start_run_background(cmd),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(handle.status, RunStatus::Active);
    for _ in 0..100 {
        if runtime
            .live_run("session", &handle.run_id, 0)
            .await
            .unwrap()
            .snapshot
            .text
            == "partial"
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    assert_eq!(
        runtime
            .live_run("session", &handle.run_id, 0)
            .await
            .unwrap()
            .snapshot
            .text,
        "partial"
    );
    runtime
        .request_cancel(&handle.run_id, "test")
        .await
        .unwrap();
    for _ in 0..100 {
        if runtime
            .live_run("session", &handle.run_id, 0)
            .await
            .unwrap()
            .snapshot
            .status
            == "cancelled"
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    let snapshot = runtime
        .live_run("session", &handle.run_id, 0)
        .await
        .unwrap()
        .snapshot;
    assert_eq!(snapshot.status, "cancelled");
    assert_eq!(snapshot.text, "partial");
}

#[tokio::test]
async fn durable_background_result_restores_without_duplicate_journal() {
    use custos_core::contracts::storage::RunPort;
    use custos_core::TaskStore;
    let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
    let task = custos_domain::Task::new("durable-live-task".into(), "Durable live".into());
    store.save_task(&task).await.unwrap();
    let sessions = Arc::new(crate::session::SessionManager::with_store(store.clone()));
    let session = sessions
        .create_session(custos_domain::SessionMode::Assisted)
        .await;
    let runtime = TaskRuntime::new()
        .with_run_store(store.clone())
        .with_model(Arc::new(TestMockModel))
        .with_journal_manager(sessions.clone());
    let cmd = StartRunCommand::new(&task.id, "chat")
        .with_preferred_mode("model")
        .with_prompt("hello")
        .with_session_id(session.id.to_string())
        .with_turn_id("durable-turn");
    let handle = runtime.start_run_background(cmd).await.unwrap();
    for _ in 0..100 {
        if runtime
            .live_run(&session.id.to_string(), &handle.run_id, 0)
            .await
            .unwrap()
            .snapshot
            .status
            == "completed"
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    let run = store.get_run(&handle.run_id).await.unwrap().unwrap();
    assert_eq!(run.status, RunStatus::Completed);
    assert_eq!(run.metadata["background"], true);
    assert!(run.metadata["journal_occurred_at"].is_string());
    let restarted = TaskRuntime::new().with_run_store(store.clone());
    let snapshot = restarted
        .live_run(&session.id.to_string(), &handle.run_id, 99)
        .await
        .unwrap();
    assert!(snapshot.gap);
    assert_eq!(snapshot.snapshot.status, "completed");
    assert!(snapshot.snapshot.text.contains("Completed turn response"));
    assert_eq!(
        sessions
            .get_journal(&session.id)
            .await
            .unwrap()
            .entries
            .iter()
            .filter(|e| e.entry_type == "assistant_message")
            .count(),
        1
    );
    let mut interrupted = custos_domain::Run::new(task.id.clone(), 2);
    interrupted.transition(RunStatus::Active).unwrap();
    interrupted.metadata = serde_json::json!({"session_id":session.id,"background":true});
    store.save_run(&mut interrupted).await.unwrap();
    assert_eq!(
        restarted
            .live_run(&session.id.to_string(), &interrupted.id, 0)
            .await
            .unwrap()
            .snapshot
            .status,
        "interrupted"
    );
}

#[tokio::test]
async fn live_capacity_rejection_releases_dispatch_claim() {
    let runtime = TaskRuntime::new().with_model(Arc::new(TestMockModel));
    for _ in 0..256 {
        let run = Run::new("retained".into(), 1);
        runtime
            .live
            .register(&run, &StartRunCommand::new("retained", "chat"))
            .unwrap();
    }
    assert!(runtime
        .start_run_background(
            StartRunCommand::new("capacity-task", "chat")
                .with_preferred_mode("model")
                .with_prompt("hello")
        )
        .await
        .is_err());
    assert!(runtime
        .dispatcher
        .get_active_claim("capacity-task", None)
        .await
        .is_none());
}

