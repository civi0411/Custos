use super::*;
use std::sync::Arc;

mod research;
mod tasks_sessions;
mod workspaces;

#[derive(Clone)]
pub struct LocalApiClient {
    pub(crate) transport: Arc<dyn ApiTransport>,
}

impl LocalApiClient {
    pub fn new(transport: Box<dyn ApiTransport>) -> Self {
        Self {
            transport: Arc::from(transport),
        }
    }

    pub fn with_arc_transport(transport: Arc<dyn ApiTransport>) -> Self {
        Self { transport }
    }

    pub fn from_tcp(addr: impl Into<String>) -> Self {
        Self::with_arc_transport(Arc::new(TcpTransport::new(addr)))
    }

    pub fn from_port(port: u16) -> Self {
        Self::from_tcp(format!("127.0.0.1:{port}"))
    }

    pub async fn ping(&self) -> Result<(), String> {
        let req = ApiRequest::new("ping", "v1.ping", serde_json::json!({}));
        let resp = self.transport.send_request(req).await?;
        if let Some(err) = resp.error {
            Err(err)
        } else {
            Ok(())
        }
    }

    pub async fn send_request(&self, req: ApiRequest) -> Result<ApiResponse, String> {
        self.transport.send_request(req).await
    }

    pub async fn dispatch_raw(&self, raw: &str) -> String {
        let req: ApiRequest = match serde_json::from_str(raw) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = ApiResponse::error("", format!("Invalid JSON request: {e}"));
                return serde_json::to_string(&err_resp).unwrap_or_else(|_| "{}".to_string());
            }
        };
        match self.transport.send_request(req).await {
            Ok(resp) => {
                serde_json::to_string(&resp).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
            }
            Err(err) => {
                let err_resp = ApiResponse::error("", err);
                serde_json::to_string(&err_resp).unwrap_or_else(|_| "{}".to_string())
            }
        }
    }
}
