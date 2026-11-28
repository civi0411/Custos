//! Custos Zero-Knowledge Cloud (`custos-cloud`).
//!
//! 1-binary, 4-role OOP architecture (`relay`, `push`, `webhook`, `policy`)
//! serving NAT traversal, Zero-PII push notifications, and webhook wake fanout.

pub mod policy;
pub mod push;
pub mod relay;
pub mod webhook;

use custos_protocol::relay::{PushDeliveryCounters, SpliceState, ZeroPiiPushAdapter, ZeroPiiPushCommand};
use policy::CloudConfig;
use push::PushService;
use relay::RelayService;
use std::sync::Arc;
use webhook::{WebhookWakeRouter, WebhookWakeSignal};

pub use push::NoopPushAdapter;

pub struct CloudRuntime {
    pub config: CloudConfig,
    pub relay: RelayService,
    pub push: PushService,
    pub webhook: WebhookWakeRouter,
}

impl CloudRuntime {
    pub fn new(config: CloudConfig, push_adapter: Arc<dyn ZeroPiiPushAdapter>) -> Self {
        Self {
            config,
            relay: RelayService::new(),
            push: PushService::new(push_adapter),
            webhook: WebhookWakeRouter::new(),
        }
    }

    pub fn admit_session(&self, session_id: impl Into<String>) -> Result<SpliceState, String> {
        if !self.config.role.enables_relay() {
            return Err(format!("Role {:?} does not enable relay", self.config.role));
        }
        self.relay.admit_session(session_id)
    }

    pub fn advance_session(&self, session_id: &str, next: SpliceState) -> Result<SpliceState, String> {
        if !self.config.role.enables_relay() {
            return Err(format!("Role {:?} does not enable relay", self.config.role));
        }
        self.relay.advance_session(session_id, next)
    }

    pub async fn notify_zero_pii(&self, cmd: ZeroPiiPushCommand) -> Result<(), String> {
        if !self.config.role.enables_push() {
            return Err(format!("Role {:?} does not enable push", self.config.role));
        }
        self.push.notify(cmd).await
    }

    pub fn push_counters(&self) -> &PushDeliveryCounters {
        self.push.counters()
    }

    pub fn enqueue_wake(&self, signal: WebhookWakeSignal) -> Result<bool, String> {
        if !self.config.role.enables_webhook() {
            return Err(format!("Role {:?} does not enable webhook", self.config.role));
        }
        self.webhook.enqueue(signal)
    }

    pub fn drain_host_wakes(&self, host_id: &str) -> Vec<WebhookWakeSignal> {
        if !self.config.role.enables_webhook() {
            return Vec::new();
        }
        self.webhook.drain_host(host_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::CloudRole;
    use custos_protocol::relay::PushCategory;

    #[tokio::test]
    async fn cloud_runtime_dispatches_relay_push_and_webhook_wake() {
        let rt = CloudRuntime::new(CloudConfig::default(), Arc::new(NoopPushAdapter));
        assert_eq!(rt.admit_session("rel-1").unwrap(), SpliceState::PreAuthAdmitted);
        assert_eq!(
            rt.advance_session("rel-1", SpliceState::CredentialLeaseReserved).unwrap(),
            SpliceState::CredentialLeaseReserved
        );
        rt.notify_zero_pii(ZeroPiiPushCommand {
            device_id: "ios-1".into(),
            category: PushCategory::PermitApproval,
            badge_count: 1,
        })
        .await
        .unwrap();
        assert_eq!(rt.push_counters().snapshot(), (1, 0, 0));

        let sig = WebhookWakeSignal {
            host_id: "host-1".into(),
            workbench_id: "coding".into(),
            source: "github".into(),
            event_key: "gh-ev-1".into(),
        };
        assert_eq!(rt.enqueue_wake(sig.clone()).unwrap(), true);
        assert_eq!(rt.enqueue_wake(sig).unwrap(), false); // deduplicated
        assert_eq!(rt.drain_host_wakes("host-1").len(), 1);
    }

    #[test]
    fn cloud_role_enforces_isolation() {
        assert!(CloudRole::parse("relay").unwrap().enables_relay());
        assert!(!CloudRole::parse("relay").unwrap().enables_push());
        assert!(CloudRole::parse("webhook").unwrap().enables_webhook());
    }
}

