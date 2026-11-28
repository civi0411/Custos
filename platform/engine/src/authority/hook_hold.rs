//! Held Permission Hook Channel for CLI harnesses (e.g. Claude Code `PermissionRequest`).
//!
//! Holds the loopback HTTP request open up to `HookHoldPolicy::budget_ms` (default 122_000ms),
//! converts it into a single-use ExecutionPermit approval prompt, and immediately writes
//! `{ decision: { behavior: "allow" | "deny", message } }` when resolved by Desktop/Mobile.
//! Never returns persistent `updatedPermissions`.

use custos_contracts::{new_id, DomainError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::oneshot;

#[derive(Debug, Clone, Copy)]
pub struct HookHoldPolicy {
    pub budget_ms: u64,
}

impl Default for HookHoldPolicy {
    fn default() -> Self {
        Self { budget_ms: 122_000 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionHookRequest {
    pub session_id: Option<String>,
    pub tool_name: String,
    #[serde(default)]
    pub tool_input: serde_json::Value,
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookDecision {
    pub behavior: HookBehavior,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HookBehavior {
    Allow,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingHookApproval {
    pub request_id: String,
    pub tool_name: String,
    pub session_id: Option<String>,
    pub created_at: i64,
}

struct HeldEntry {
    meta: PendingHookApproval,
    sender: oneshot::Sender<HookDecision>,
}

#[derive(Default)]
pub struct HeldPermissionHookChannel {
    policy: HookHoldPolicy,
    pending: Mutex<HashMap<String, HeldEntry>>,
}

impl HeldPermissionHookChannel {
    pub fn new(policy: HookHoldPolicy) -> Self {
        Self {
            policy,
            pending: Mutex::new(HashMap::new()),
        }
    }

    pub fn policy(&self) -> HookHoldPolicy {
        self.policy
    }

    /// Registers a held permission request and returns `(request_id, receiver)`.
    pub fn register(
        &self,
        req: PermissionHookRequest,
    ) -> Result<(String, oneshot::Receiver<HookDecision>), DomainError> {
        let request_id = new_id("hookperm");
        let (tx, rx) = oneshot::channel();
        let meta = PendingHookApproval {
            request_id: request_id.clone(),
            tool_name: req.tool_name,
            session_id: req.session_id,
            created_at: chrono::Utc::now().timestamp(),
        };
        let mut guard = self
            .pending
            .lock()
            .map_err(|_| DomainError::Validation("held hook channel lock poisoned".into()))?;
        guard.insert(request_id.clone(), HeldEntry { meta, sender: tx });
        Ok((request_id, rx))
    }

    /// Awaits user decision up to `HookHoldPolicy::budget_ms`.
    pub async fn await_decision(&self, request_id: &str, rx: oneshot::Receiver<HookDecision>) -> HookDecision {
        let timeout = Duration::from_millis(self.policy.budget_ms);
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(decision)) => decision,
            _ => {
                if let Ok(mut guard) = self.pending.lock() {
                    guard.remove(request_id);
                }
                HookDecision {
                    behavior: HookBehavior::Deny,
                    message: Some("Custos permission hold timed out; single-use permit denied".into()),
                }
            }
        }
    }

    /// Resolves a held hook request immediately (one-shot allow or deny).
    pub fn resolve(
        &self,
        request_id: &str,
        allow: bool,
        reason: Option<String>,
    ) -> Result<(), DomainError> {
        let entry = {
            let mut guard = self
                .pending
                .lock()
                .map_err(|_| DomainError::Validation("held hook channel lock poisoned".into()))?;
            guard.remove(request_id)
        }
        .ok_or_else(|| DomainError::NotFound {
            kind: "held_permission_hook".into(),
            id: request_id.to_string(),
        })?;

        let decision = HookDecision {
            behavior: if allow {
                HookBehavior::Allow
            } else {
                HookBehavior::Deny
            },
            message: reason,
        };
        let _ = entry.sender.send(decision);
        Ok(())
    }

    pub fn list_pending(&self) -> Vec<PendingHookApproval> {
        self.pending
            .lock()
            .map(|guard| guard.values().map(|e| e.meta.clone()).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn held_hook_resolves_allow_and_deny_and_times_out() {
        let channel = HeldPermissionHookChannel::new(HookHoldPolicy { budget_ms: 50 });

        let (id1, rx1) = channel
            .register(PermissionHookRequest {
                session_id: Some("sess-1".into()),
                tool_name: "Bash".into(),
                tool_input: serde_json::json!({ "command": "ls" }),
                cwd: None,
            })
            .unwrap();
        assert_eq!(channel.list_pending().len(), 1);
        channel.resolve(&id1, true, None).unwrap();
        let d1 = channel.await_decision(&id1, rx1).await;
        assert_eq!(d1.behavior, HookBehavior::Allow);

        let (id2, rx2) = channel
            .register(PermissionHookRequest {
                session_id: None,
                tool_name: "Write".into(),
                tool_input: serde_json::json!({}),
                cwd: None,
            })
            .unwrap();
        let d2 = channel.await_decision(&id2, rx2).await;
        assert_eq!(d2.behavior, HookBehavior::Deny);
        assert!(channel.list_pending().is_empty());
    }
}

