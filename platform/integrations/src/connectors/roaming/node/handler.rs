use std::sync::Arc;

use futures::io::{AsyncRead, AsyncWrite};
use iroh::{
    endpoint::Connection,
    protocol::{AcceptError, ProtocolHandler},
    EndpointId,
};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use super::{AcpStreamServer, RoamingNode, HANDSHAKE_TIMEOUT};
use crate::roaming::directory::Direction;
use crate::roaming::error::RoamingError;
use crate::roaming::frame::{read_frame, write_frame};
use crate::roaming::handshake::{ClientHello, HostAck};
use crate::roaming::trust::TrustBook;

#[derive(Clone)]
pub(crate) struct RoamingAcpHandler {
    pub(crate) node: Arc<RoamingNode>,
    pub(crate) server: Arc<dyn AcpStreamServer>,
}

impl std::fmt::Debug for RoamingAcpHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RoamingAcpHandler")
    }
}

impl ProtocolHandler for RoamingAcpHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let client = connection.remote_id();

        // Bound the whole handshake phase so a peer that connects and then
        // stalls (never opening its stream or never sending a hello) is dropped
        // instead of parking this task forever (Slowloris guard).
        let handshake = async {
            let (send, mut recv) = connection.accept_bi().await?;
            // Authorize the key *before* acking so "Accepted" is truthful.
            let decision = self.authorize(client, &mut recv).await;
            Ok::<_, AcceptError>((send, recv, decision))
        };
        let (mut send, recv, decision) =
            match tokio::time::timeout(HANDSHAKE_TIMEOUT, handshake).await {
                Ok(Ok(parts)) => parts,
                Ok(Err(e)) => return Err(e),
                Err(_) => {
                    tracing::info!(%client, "roaming: handshake timed out; dropping connection");
                    return Ok(());
                }
            };
        match decision {
            Ok(label) => {
                let agent_id = self.server.agent_id();
                let ack = HostAck::Accepted {
                    agent_id: agent_id.clone(),
                };
                if let Err(e) = send_ack(&mut send, &ack).await {
                    tracing::warn!("roaming: failed to send accept ack: {e}");
                    return Ok(());
                }
                self.node.register_live(client, connection.clone()).await;

                // Close the accept/register TOCTOU: a revocation that lands
                // after `authorize` read the allowlist but before the line
                // above can be processed by the revocation watcher while this
                // connection is still absent from `live`, so it would never be
                // force-closed. Re-read trust now that we are registered; if
                // the key is no longer allowed, drop the connection here. A
                // failed reload fails closed, matching `authorize`.
                let still_allowed = match &self.node.trust_path {
                    Some(path) => match TrustBook::load(path) {
                        Ok(book) => book.is_allowed(&client),
                        Err(e) => {
                            tracing::warn!(%client, "roaming: trust recheck failed, closing: {e}");
                            false
                        }
                    },
                    None => self.node.trust.lock().await.is_allowed(&client),
                };
                if !still_allowed {
                    self.node.unregister_live(client, &connection).await;
                    connection.close(0u32.into(), b"revoked");
                    tracing::info!(%client, "roaming: revoked during accept; closed connection");
                    return Ok(());
                }

                self.node
                    .directory
                    .record_connect(client, label, Direction::Inbound, Some(agent_id), now_ms())
                    .await;
                let recv_box: Box<dyn AsyncRead + Send + Unpin> = Box::new(recv.compat());
                let send_box: Box<dyn AsyncWrite + Send + Unpin> = Box::new(send.compat_write());
                if let Err(e) = self.server.serve_stream(client, recv_box, send_box).await {
                    tracing::warn!("roaming: ACP session ended with error: {e}");
                }
                self.node.unregister_live(client, &connection).await;
                self.node
                    .directory
                    .record_disconnect(client, now_ms())
                    .await;
            }
            Err(reason) => {
                let ack = HostAck::Rejected {
                    code: reason.to_string(),
                };
                let _ = send_ack(&mut send, &ack).await;
                // Returning drops the connection, and a QUIC close can beat
                // the ack to the peer — which would turn a precise "rejected:
                // not_allowlisted" into a generic "connection lost" on the
                // client. Finish the stream and give the client a moment to
                // read the ack and close first.
                let _ = send.finish();
                let _ =
                    tokio::time::timeout(std::time::Duration::from_secs(3), connection.closed())
                        .await;
                tracing::info!(%client, reason = %reason, "roaming: rejected connection");
            }
        }
        Ok(())
    }
}

impl RoamingAcpHandler {
    /// Authorize a connection purely by the transport-authenticated peer key.
    /// The peer's identity is already proven by QUIC-TLS; this only checks the
    /// allowlist. The [`ClientHello`] carries just a display label — nothing
    /// trusted for authorization. Returns the sanitized label on success.
    async fn authorize(
        &self,
        client: EndpointId,
        recv: &mut iroh::endpoint::RecvStream,
    ) -> Result<Option<String>, String> {
        let hello_bytes = read_frame(recv).await.map_err(|e| e.to_string())?;
        let hello: ClientHello =
            serde_json::from_slice(&hello_bytes).map_err(|e| format!("bad hello: {e}"))?;

        // Re-read the persisted allowlist so `peers accept`/`revoke` from a
        // separate process take effect against this running share without a
        // restart. Reads are atomic (writers rename into place), so we never see
        // a half-written file. If a path is set but the read genuinely fails we
        // fail *closed* rather than fall back to a stale in-memory book — a
        // just-revoked peer must not slip through. We snapshot under the lock and
        // drop it before any decision so the mutex isn't held across I/O.
        let refreshed = match &self.node.trust_path {
            Some(path) => match TrustBook::load(path) {
                Ok(book) => Some(book),
                Err(e) => {
                    tracing::warn!(%client, "roaming: trust reload failed, refusing: {e}");
                    return Err("unavailable".to_string());
                }
            },
            None => None,
        };
        let trust = match &refreshed {
            Some(book) => book,
            None => &*self.node.trust.lock().await,
        };
        if trust.is_key_revoked(&client) {
            return Err("revoked".to_string());
        }
        if !trust.is_allowed(&client) {
            return Err("not_allowlisted".to_string());
        }

        Ok(hello.label.and_then(sanitize_label))
    }
}

async fn send_ack(
    send: &mut iroh::endpoint::SendStream,
    ack: &HostAck,
) -> Result<(), RoamingError> {
    let bytes =
        serde_json::to_vec(ack).map_err(|e| RoamingError::Transport(format!("encode ack: {e}")))?;
    write_frame(send, &bytes).await
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// The label is attacker-controlled display text (it comes from the connecting
/// peer's hello) and is surfaced in `roam connections`. Strip control
/// characters and cap the length so it can't corrupt terminal output; drop it
/// entirely if nothing printable remains.
pub(crate) fn sanitize_label(label: String) -> Option<String> {
    const MAX_LABEL_CHARS: usize = 64;
    let cleaned: String = label
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_LABEL_CHARS)
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_label;

    #[test]
    fn sanitize_label_strips_control_chars_and_caps_length() {
        assert_eq!(sanitize_label("laptop".into()), Some("laptop".into()));
        // Control chars (incl. ANSI escape) are removed.
        assert_eq!(
            sanitize_label("lap\x1b[31mtop\n".into()),
            Some("lap[31mtop".into())
        );
        // Whitespace-only / empty collapses to None.
        assert_eq!(sanitize_label("   ".into()), None);
        assert_eq!(sanitize_label("\n\t".into()), None);
        // Length is capped.
        let long = "x".repeat(200);
        assert_eq!(sanitize_label(long).unwrap().chars().count(), 64);
    }
}
