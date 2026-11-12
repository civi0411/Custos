use super::envelope::{ApiRequest, ApiResponse};
use async_trait::async_trait;

/// Abstract transport for communicating with the Custos Daemon.
#[async_trait]
pub trait ApiTransport: Send + Sync {
    async fn send_request(&self, req: ApiRequest) -> Result<ApiResponse, String>;
}

