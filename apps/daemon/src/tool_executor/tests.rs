use super::*;
use custos_domain::DomainError;

#[test]
fn test_parse_tool_invocation_valid_standard_tools() {
    let params = serde_json::json!({ "path": "platform/core/src/lib.rs" });
    let (act, risk, target) =
        DaemonGovernedToolExecutor::parse_tool_invocation("read_file", &params).unwrap();
    assert_eq!(act, "read_file");
    assert_eq!(risk, custos_domain::RiskLevel::Low);
    assert_eq!(target, "platform/core/src/lib.rs");

    let write_params = serde_json::json!({ "path": "output.txt", "content": "hello" });
    let (act, risk, _) =
        DaemonGovernedToolExecutor::parse_tool_invocation("write_file", &write_params).unwrap();
    assert_eq!(act, "write_file");
    assert_eq!(risk, custos_domain::RiskLevel::Medium);
}

#[test]
fn test_parse_tool_invocation_rejects_path_traversal() {
    let params = serde_json::json!({ "path": "../../etc/passwd" });
    let res = DaemonGovernedToolExecutor::parse_tool_invocation("read_file", &params);
    assert!(res.is_err());
    assert!(
        matches!(res.unwrap_err(), DomainError::Validation(msg) if msg.contains("Path traversal"))
    );
}

#[test]
fn test_parse_tool_invocation_rejects_unknown_tools() {
    let params = serde_json::json!({});
    let res = DaemonGovernedToolExecutor::parse_tool_invocation("unregistered_tool", &params);
    assert!(res.is_err());
    assert!(
        matches!(res.unwrap_err(), DomainError::Validation(msg) if msg.contains("Unknown or unmapped"))
    );
}

#[test]
fn test_parse_tool_invocation_classifies_critical_bash_commands() {
    let safe_cmd = serde_json::json!({ "command": "cargo check" });
    let (_, risk, _) =
        DaemonGovernedToolExecutor::parse_tool_invocation("bash", &safe_cmd).unwrap();
    assert_eq!(risk, custos_domain::RiskLevel::High);

    let destructive_cmd = serde_json::json!({ "command": "mkfs.ext4 /dev/sda1" });
    let (_, risk, _) =
        DaemonGovernedToolExecutor::parse_tool_invocation("bash", &destructive_cmd).unwrap();
    assert_eq!(risk, custos_domain::RiskLevel::Critical);
}

#[test]
fn test_parse_tool_invocation_mcp_tools() {
    let mcp_params = serde_json::json!({ "target": "https://example.com" });
    let (act, risk, target) =
        DaemonGovernedToolExecutor::parse_tool_invocation("web__fetch", &mcp_params).unwrap();
    assert_eq!(act, "web__fetch");
    assert_eq!(risk, custos_domain::RiskLevel::Medium);
    assert_eq!(target, "https://example.com");
}

#[cfg(feature = "custos-workbench-assistant")]
#[test]
fn test_parse_tool_with_registry_allows_registered_domain_tool() {
    custos_packs::ModuleRegistry::global()
        .register(std::sync::Arc::new(custos_workbench_assistant::AssistantModule::new()));

    let registry = custos_packs::create_pack_registry_from_inventory(&["assistant"]);
    let params = serde_json::json!({ "title": "Check logs", "due_at": "2026-11-09T00:00:00Z" });
    let (act, risk, target) = DaemonGovernedToolExecutor::parse_tool_with_registry(
        "schedule_task",
        &params,
        Some(&registry),
    )
    .expect("schedule_task should be accepted when assistant pack is installed");

    assert_eq!(act, "schedule_task");
    assert_eq!(risk, custos_domain::RiskLevel::Medium);
    assert_eq!(target, "Check logs");
}

#[test]
fn test_parse_tool_with_registry_blocks_uninstalled_workbench_tool() {
    let registry = custos_packs::create_pack_registry_from_inventory(&["assistant"]);
    let params = serde_json::json!({ "strategy_id": "momentum_1" });
    let res = DaemonGovernedToolExecutor::parse_tool_with_registry(
        "run_backtest",
        &params,
        Some(&registry),
    );

    assert!(res.is_err());
    match res.unwrap_err() {
        DomainError::Unauthorized(msg) => {
            assert!(msg.contains("not installed or enabled"));
        }
        other => panic!("Expected DomainError::Unauthorized, got {other:?}"),
    }
}

