use super::*;

#[tokio::test]
async fn test_mcp_client_call_mock() {
    let client = CustosMcpClient::new("test-server");
    let ctx = ToolCallContext::new("sess-1", "req-1");

    let res = client
        .call_tool(
            &ctx,
            "read_file",
            Some(serde_json::json!({"path": "Cargo.toml"})),
        )
        .await
        .unwrap();

    assert!(!res.content.is_empty());
    let text = res.content[0].as_text().unwrap();
    assert!(text.text.contains("read_file"));
}

#[tokio::test]
async fn test_http_client_creation() {
    let mut headers = HashMap::new();
    headers.insert("X-Custos-Trace".into(), "test-trace".into());
    let client = CustosMcpClient::new_http(
        "remote-server",
        "http://127.0.0.1:9099/mcp",
        Some("bearer-token-123".into()),
        headers,
    );
    assert_eq!(client.server_name, "remote-server");
}

#[tokio::test]
async fn test_dangerous_tool_pattern_requires_permit() {
    let client = CustosMcpClient::new("test-server");

    // 1. Calling dangerous tool without permit must fail
    let unpermitted_ctx = ToolCallContext::new("sess-1", "req-1");
    let res_err = client
        .call_tool(&unpermitted_ctx, "bash_execute", None)
        .await;
    assert!(res_err.is_err());
    assert!(res_err
        .unwrap_err()
        .to_string()
        .contains("requires explicit grant"));

    // 2. Calling dangerous tool with permit succeeds on mock
    let permitted_ctx = ToolCallContext::new("sess-1", "req-2").with_sade_scope(
        "task-1",
        "run-1",
        Some("pmt-1".into()),
    );
    let res_ok = client.call_tool(&permitted_ctx, "bash_execute", None).await;
    assert!(res_ok.is_ok());
}
