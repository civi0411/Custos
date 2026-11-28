//! Daemon Zero-PII Push Adapter bridging `KernelServices` to `ZeroPiiPushGateway`.

use std::sync::Arc;

use async_trait::async_trait;
use custos_protocol::relay::{
    PushCategory, ZeroPiiPushAdapter, ZeroPiiPushCommand, ZeroPiiPushGateway,
};

/// HTTP relay adapter sending 0-PII push alerts (`device_id`, `category`, `badge_count`).
pub struct RelayZeroPiiPushAdapter {
    relay_push_url: Option<String>,
    client: reqwest::Client,
}

impl RelayZeroPiiPushAdapter {
    pub fn from_env() -> Self {
        Self {
            relay_push_url: std::env::var("CUSTOS_RELAY_PUSH_URL")
                .ok()
                .filter(|v| !v.trim().is_empty()),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_url(url: impl Into<String>) -> Self {
        Self {
            relay_push_url: Some(url.into()),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl ZeroPiiPushAdapter for RelayZeroPiiPushAdapter {
    async fn send(&self, cmd: &ZeroPiiPushCommand) -> Result<(), String> {
        let Some(ref url) = self.relay_push_url else {
            tracing::debug!(
                device_id = %cmd.device_id,
                category = ?cmd.category,
                badge_count = cmd.badge_count,
                "Zero-PII push suppressed (CUSTOS_RELAY_PUSH_URL unset)"
            );
            return Ok(());
        };
        let resp = self
            .client
            .post(url)
            .json(cmd)
            .send()
            .await
            .map_err(|e| format!("Relay push request failed: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("Relay push returned HTTP {}", resp.status()));
        }
        Ok(())
    }
}

pub async fn notify_persona_needs_you(
    gateway: Option<&Arc<ZeroPiiPushGateway>>,
    device_id: &str,
    category: PushCategory,
    badge_count: u32,
) {
    let Some(gw) = gateway else {
        return;
    };
    let cmd = ZeroPiiPushCommand {
        device_id: device_id.to_string(),
        category,
        badge_count,
    };
    if let Err(err) = gw.dispatch(&cmd).await {
        tracing::warn!(device_id = %device_id, error = %err, "Zero-PII push dispatch failed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn zero_pii_gateway_dispatches_and_records_counter() {
        let adapter = Arc::new(RelayZeroPiiPushAdapter::from_env());
        let gateway = Arc::new(ZeroPiiPushGateway::new(adapter));
        notify_persona_needs_you(
            Some(&gateway),
            "dev-iphone-1",
            PushCategory::AutomationAlert,
            3,
        )
        .await;
        let (sent, failed, _) = gateway.counters.snapshot();
        assert_eq!(sent, 1);
        assert_eq!(failed, 0);
    }
}

