//! Roaming ACP bridge wrapping `LocalApiDispatcher` for `iroh` P2P/Relay streams.
//!
//! Uses `u32` little-endian length-prefixed JSON frames matching `roaming/frame.rs`.

use crate::api::LocalApiDispatcher;
use crate::custos_local_api::ApiRequest;
use custos_integrations::protocols::roaming::{AcpStreamServer, EndpointId};
use futures::future::BoxFuture;
use futures::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use std::sync::Arc;

const MAX_ROAMING_RPC_FRAME_BYTES: usize = 8 * 1024 * 1024;

pub struct DaemonAcpServer {
    agent_id: String,
    api: Arc<LocalApiDispatcher>,
}

impl DaemonAcpServer {
    pub fn new(agent_id: impl Into<String>, api: Arc<LocalApiDispatcher>) -> Self {
        Self {
            agent_id: agent_id.into(),
            api,
        }
    }
}

async fn read_len_frame<R: AsyncRead + Unpin>(recv: &mut R) -> anyhow::Result<Option<Vec<u8>>> {
    let mut len_buf = [0u8; 4];
    match recv.read_exact(&mut len_buf).await {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e.into()),
    }
    let len = u32::from_le_bytes(len_buf) as usize;
    if len == 0 || len > MAX_ROAMING_RPC_FRAME_BYTES {
        anyhow::bail!("roaming frame length out of bounds: {len}");
    }
    let mut payload = vec![0u8; len];
    recv.read_exact(&mut payload).await?;
    Ok(Some(payload))
}

async fn write_len_frame<W: AsyncWrite + Unpin>(send: &mut W, payload: &[u8]) -> anyhow::Result<()> {
    let len: u32 = payload
        .len()
        .try_into()
        .map_err(|_| anyhow::anyhow!("roaming response too large"))?;
    send.write_all(&len.to_le_bytes()).await?;
    send.write_all(payload).await?;
    send.flush().await?;
    Ok(())
}

impl AcpStreamServer for DaemonAcpServer {
    fn agent_id(&self) -> String {
        self.agent_id.clone()
    }

    fn serve_stream(
        &self,
        _client: EndpointId,
        mut recv: Box<dyn AsyncRead + Send + Unpin>,
        mut send: Box<dyn AsyncWrite + Send + Unpin>,
    ) -> BoxFuture<'static, anyhow::Result<()>> {
        let api = self.api.clone();
        Box::pin(async move {
            while let Some(frame) = read_len_frame(&mut recv).await? {
                let resp = match serde_json::from_slice::<ApiRequest>(&frame) {
                    Ok(req) => api.handle_request(req).await,
                    Err(err) => crate::custos_local_api::ApiResponse::error(
                        "",
                        format!("Invalid roaming JSON-RPC frame: {err}"),
                    ),
                };
                let encoded = serde_json::to_vec(&resp)?;
                write_len_frame(&mut send, &encoded).await?;
            }
            Ok(())
        })
    }
}
