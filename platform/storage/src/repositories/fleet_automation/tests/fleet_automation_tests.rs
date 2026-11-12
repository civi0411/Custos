use custos_domain::{
    BrowserConsoleEntry, BrowserSession, BrowserTab, BrowserTabStatus, FleetExecReceipt,
    HeadlessAutomationJob, HeadlessJobStatus, HeadlessTaskSpec, HeadlessTrigger, RemoteHostNode,
    SshAuthMethod,
};

use crate::connection::DbConnection;
use crate::repositories::browser::BrowserRepository;
use crate::repositories::fleet_automation::FleetAutomationRepository;

fn setup_test_repo() -> FleetAutomationRepository {
    let db = DbConnection::open_in_memory().unwrap();
    FleetAutomationRepository::new(db)
}

#[test]
fn test_browser_sessions_and_tabs_persistence() {
    let db = DbConnection::open_in_memory().unwrap();
    let repo = BrowserRepository::new(db);

    let session = BrowserSession::new("research-browsing", Some("ws_default".to_string()));
    repo.save_browser_session(&session).unwrap();

    let mut tab = BrowserTab::new(&session.id, "https://arxiv.org/abs/2301.00001").unwrap();
    tab.status = BrowserTabStatus::Ready;
    tab.console_logs.push(BrowserConsoleEntry {
        level: "info".to_string(),
        message: "Navigation complete".to_string(),
        timestamp: 1234567,
    });
    repo.save_browser_tab(&tab).unwrap();

    let fetched_tab = repo.get_browser_tab(&tab.id).unwrap().unwrap();
    assert_eq!(fetched_tab.url, "https://arxiv.org/abs/2301.00001");
    assert_eq!(fetched_tab.console_logs.len(), 1);

    let session_with_tabs = repo.get_browser_session(&session.id).unwrap().unwrap();
    assert_eq!(session_with_tabs.tabs.len(), 1);
    assert_eq!(session_with_tabs.tabs[0].id, tab.id);
}

#[test]
fn test_remote_host_and_fleet_execution_persistence() {
    let repo = setup_test_repo();

    let host = RemoteHostNode::new(
        "gpu-node-01",
        "192.168.1.100",
        22,
        "ubuntu",
        SshAuthMethod::KeyPair {
            private_key_path: "~/.ssh/id_rsa".to_string(),
            passphrase: None,
        },
    )
    .unwrap();
    repo.save_remote_host(&host).unwrap();

    let fetched_host = repo.get_remote_host(&host.id).unwrap().unwrap();
    assert_eq!(fetched_host.name, "gpu-node-01");
    assert_eq!(fetched_host.port, 22);

    let receipt = FleetExecReceipt {
        execution_id: "exec_1".to_string(),
        host_id: host.id.clone(),
        command: "nvidia-smi".to_string(),
        exit_code: Some(0),
        stdout: "NVIDIA-SMI 535.129.03 Driver Version: 535.129.03".to_string(),
        stderr: String::new(),
        duration_ms: 45,
        verified: true,
        timestamp: 1234567,
    };
    repo.save_fleet_execution(&receipt).unwrap();

    let receipts = repo.list_fleet_executions(Some(&host.id)).unwrap();
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].command, "nvidia-smi");
    assert_eq!(receipts[0].exit_code, Some(0));
}

#[test]
fn test_headless_automation_jobs_persistence() {
    let repo = setup_test_repo();

    let spec = HeadlessTaskSpec {
        workspace_id: Some("ws_test".to_string()),
        target_type: "local_process".to_string(),
        command_or_script: "cargo test --workspace".to_string(),
        timeout_secs: 180,
        env: std::collections::HashMap::new(),
        required_evidence: vec!["TestResult".to_string()],
    };

    let mut job =
        HeadlessAutomationJob::new("continuous-integration", spec, HeadlessTrigger::Manual)
            .unwrap();
    job.status = HeadlessJobStatus::Completed;
    job.exit_code = Some(0);
    job.output_log = "All tests passed successfully.".to_string();

    repo.save_headless_job(&job).unwrap();

    let fetched = repo.get_headless_job(&job.id).unwrap().unwrap();
    assert_eq!(fetched.name, "continuous-integration");
    assert_eq!(fetched.status, HeadlessJobStatus::Completed);
    assert_eq!(fetched.exit_code, Some(0));
}

