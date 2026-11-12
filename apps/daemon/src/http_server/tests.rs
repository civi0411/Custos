use super::security::{is_allowed_origin, is_authorized_ipc_client};
use super::server::start_http_server;
use super::sse::write_live;
use crate::api::LocalApiDispatcher;
use custos_core::{contracts::workflow::WorkflowPort, TaskService};
use custos_domain::{SessionMode, StartRunCommand};
use custos_runtime::{workflow::TaskRuntime, SessionManager};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn slow_subscriber_writer_times_out() {
    let (mut server, client) = tokio::io::duplex(64);
    let data = vec![b'x'; 1024];
    let error = tokio::time::timeout(Duration::from_secs(7), write_live(&mut server, &data))
        .await
        .unwrap()
        .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
    drop(client);
}

#[tokio::test]
async fn sse_replays_after_start_and_checks_session_scope() {
    let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
    let tasks = Arc::new(TaskService::new(store.clone()));
    let sessions = Arc::new(SessionManager::with_store(store.clone()));
    let task = tasks.create_task("SSE fixture".into()).await.unwrap();
    let session = sessions.create_session(SessionMode::Assisted).await;
    let other = sessions.create_session(SessionMode::Assisted).await;
    let bridge = Arc::new(custos_runtime::bridge::BridgeService::new(
        sessions.clone(),
        tasks.clone(),
    ));
    let runtime = Arc::new(TaskRuntime::new().with_run_store(store.clone()).with_model(
        Arc::new(
            custos_provider::cloud::fake::FakeProvider::with_default_text("test", "answer"),
        ),
    ));
    let run = runtime
        .start_run_background(
            StartRunCommand::new(&task.id, "chat")
                .with_prompt("prompt")
                .with_session_id(session.id.to_string())
                .with_preferred_mode("model"),
        )
        .await
        .unwrap();
    let api = Arc::new(LocalApiDispatcher::new(tasks, sessions, bridge).with_workflow(runtime));
    let (port, server) = start_http_server("127.0.0.1:0", api).await.unwrap();
    let url = format!(
        "http://127.0.0.1:{port}/api/events?session_id={}&run_id={}",
        session.id, run.run_id
    );
    let response = reqwest::get(&url).await.unwrap();
    assert_eq!(response.status(), 200);
    assert!(response.headers()["content-type"]
        .to_str()
        .unwrap()
        .starts_with("text/event-stream"));
    let events = response.text().await.unwrap();
    assert!(events.contains("event: run"));
    assert!(events.contains("queued"));
    assert!(events.contains("completed"));
    assert!(events.contains("answer"));
    let cursor = events
        .lines()
        .filter_map(|line| line.strip_prefix("id: "))
        .next_back()
        .unwrap();
    let response = reqwest::Client::new()
        .get(&url)
        .header("Last-Event-ID", cursor)
        .send()
        .await
        .unwrap();
    let replay = response.text().await.unwrap();
    assert!(replay.contains("event: snapshot"));
    assert!(!replay.contains("text_delta"));
    let wrong = format!(
        "http://127.0.0.1:{port}/api/events?session_id={}&run_id={}",
        other.id, run.run_id
    );
    assert_eq!(reqwest::get(wrong).await.unwrap().status(), 403);
    server.abort();
}

#[test]
fn origin_policy_accepts_only_exact_loopback_authorities() {
    for allowed in [
        "tauri://localhost",
        "http://localhost",
        "http://localhost:1420",
        "https://127.0.0.1:443",
        "http://[::1]:3000",
    ] {
        assert!(is_allowed_origin(allowed), "expected allowed: {allowed}");
    }

    for rejected in [
        "http://localhost.evil.test",
        "http://localhost@evil.test",
        "https://127.0.0.1.attacker.test",
        "http://127.0.0.1/path",
        "http://localhost:not-a-port",
        "https://example.com",
        "null",
    ] {
        assert!(
            !is_allowed_origin(rejected),
            "expected rejected: {rejected}"
        );
    }
}

#[test]
fn ipc_token_enforcement_cannot_be_bypassed_by_origin() {
    std::env::set_var("CUSTOS_IPC_TOKEN", "secret-token-123");

    // Allowed origin without token MUST be rejected
    assert!(!is_authorized_ipc_client(
        Some("http://localhost:1420"),
        None,
        None,
        None
    ));

    // Allowed origin with wrong token MUST be rejected
    assert!(!is_authorized_ipc_client(
        Some("http://localhost:1420"),
        Some("Bearer wrong"),
        None,
        None
    ));

    // Untrusted origin even with valid token MUST be rejected
    assert!(!is_authorized_ipc_client(
        Some("http://evil.com"),
        Some("Bearer secret-token-123"),
        None,
        None
    ));

    // Allowed origin with valid Bearer token MUST be accepted
    assert!(is_authorized_ipc_client(
        Some("http://localhost:1420"),
        Some("Bearer secret-token-123"),
        None,
        None
    ));

    // Allowed origin with valid X-Custos-Token header MUST be accepted
    assert!(is_authorized_ipc_client(
        Some("http://localhost:1420"),
        None,
        Some("secret-token-123"),
        None
    ));

    // Allowed origin with valid query token MUST be accepted
    assert!(is_authorized_ipc_client(
        Some("http://localhost:1420"),
        None,
        None,
        Some("secret-token-123")
    ));

    // Direct client without origin with valid token MUST be accepted
    assert!(is_authorized_ipc_client(
        None,
        Some("Bearer secret-token-123"),
        None,
        None
    ));

    std::env::remove_var("CUSTOS_IPC_TOKEN");

    // Without CUSTOS_IPC_TOKEN, untrusted origin MUST still be rejected
    assert!(!is_authorized_ipc_client(
        Some("http://evil.com"),
        None,
        None,
        None
    ));
    // Without CUSTOS_IPC_TOKEN, allowed origin is accepted
    assert!(is_authorized_ipc_client(
        Some("http://localhost:1420"),
        None,
        None,
        None
    ));
}
