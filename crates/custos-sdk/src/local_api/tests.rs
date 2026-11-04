use super::*;

    struct MockTransport;

    #[async_trait]
    impl ApiTransport for MockTransport {
        async fn send_request(&self, req: ApiRequest) -> Result<ApiResponse, String> {
            match req.method.as_str() {
                METHOD_TASKS_CREATE => {
                    let task = Task::new(custos_domain::new_id("task"), "Mock Task".to_string());
                    Ok(ApiResponse::ok(
                        req.id,
                        serde_json::to_value(&task).unwrap(),
                    ))
                }
                _ => Ok(ApiResponse::error(req.id, "Unsupported method")),
            }
        }
    }

    #[tokio::test]
    async fn test_client_create_task() {
        let client = LocalApiClient::new(Box::new(MockTransport));
        let task = client
            .create_task("1", "Mock Task", None, None)
            .await
            .unwrap();
        assert_eq!(task.title, "Mock Task");
        assert_eq!(task.status, TaskStatus::Draft);
    }

    #[tokio::test]
    async fn test_tcp_transport_roundtrip() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let (reader, mut writer) = socket.split();
                let mut lines = BufReader::new(reader).lines();
                if let Ok(Some(line)) = lines.next_line().await {
                    let req: ApiRequest = serde_json::from_str(&line).unwrap();
                    assert_eq!(req.method, "v1.ping");
                    let resp =
                        ApiResponse::success(req.id, serde_json::json!({ "status": "pong" }));
                    let mut data = serde_json::to_vec(&resp).unwrap();
                    data.push(b'\n');
                    let _ = writer.write_all(&data).await;
                    let _ = writer.flush().await;
                }
            }
        });

        let client = LocalApiClient::from_tcp(format!("127.0.0.1:{port}"));
        let req = ApiRequest::new("ping_1", "v1.ping", serde_json::json!({}));
        let resp = client.send_request(req).await.unwrap();
        assert!(resp.is_success());
        assert_eq!(
            resp.result.unwrap(),
            serde_json::json!({ "status": "pong" })
        );
    }
