use anyhow::{anyhow, Result};
use chrono::Utc;
use custos_domain::{SessionId, TaskId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{oneshot, Mutex, OwnedMutexGuard, RwLock};
use tokio::time::timeout;
use tracing::warn;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HumanGateOutcome {
    Approved(Value),
    Rejected { reason: String },
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanGateRequest {
    pub id: String,
    pub session_id: SessionId,
    pub task_id: Option<TaskId>,
    pub message: String,
    pub schema: Option<Value>,
    pub created_at: String,
}

struct PendingRequestInternal {
    request: HumanGateRequest,
    response_tx: Option<oneshot::Sender<HumanGateOutcome>>,
}

pub struct PendingClaim {
    pub request_id: String,
    pending: OwnedMutexGuard<PendingRequestInternal>,
}

impl PendingClaim {
    pub fn submit(mut self, response: HumanGateOutcome) -> Result<()> {
        let tx = self
            .pending
            .response_tx
            .take()
            .ok_or_else(|| anyhow!("Request already resolved: {}", self.request_id))?;
        drop(self.pending);

        if tx.send(response).is_err() {
            return Err(anyhow!("Response receiver dropped"));
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct HumanGate {
    pending: Arc<RwLock<HashMap<String, Arc<Mutex<PendingRequestInternal>>>>>,
}

impl Default for HumanGate {
    fn default() -> Self {
        Self::new()
    }
}

impl HumanGate {
    pub fn new() -> Self {
        Self {
            pending: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Creates an elicitation or approval request and awaits human response or timeout.
    pub async fn request_and_wait(
        &self,
        session_id: SessionId,
        task_id: Option<TaskId>,
        message: String,
        schema: Option<Value>,
        timeout_duration: Duration,
    ) -> Result<HumanGateOutcome> {
        let id = Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();

        let req = HumanGateRequest {
            id: id.clone(),
            session_id,
            task_id,
            message,
            schema,
            created_at: Utc::now().to_rfc3339(),
        };

        let pending_internal = PendingRequestInternal {
            request: req,
            response_tx: Some(tx),
        };
        let internal_arc = Arc::new(Mutex::new(pending_internal));

        self.pending
            .write()
            .await
            .insert(id.clone(), Arc::clone(&internal_arc));

        let res = match timeout(timeout_duration, rx).await {
            Ok(Ok(outcome)) => Ok(outcome),
            Ok(Err(_)) => Err(anyhow!("Human gate channel closed unexpectedly")),
            Err(_) => {
                warn!(request_id = %id, "Human gate request timed out");
                Err(anyhow!("Human gate request timed out"))
            }
        };

        self.pending.write().await.remove(&id);
        res
    }

    /// Claims an active request for submitting a response.
    pub async fn claim_request(&self, request_id: &str) -> Result<PendingClaim> {
        let pending_arc = self
            .pending
            .read()
            .await
            .get(request_id)
            .cloned()
            .ok_or_else(|| anyhow!("Human gate request not found: {}", request_id))?;

        let guard = pending_arc
            .try_lock_owned()
            .map_err(|_| anyhow!("Request is already being processed"))?;

        Ok(PendingClaim {
            request_id: request_id.to_string(),
            pending: guard,
        })
    }

    /// Lists all pending human gate requests.
    pub async fn list_pending(&self) -> Vec<HumanGateRequest> {
        let mut list = Vec::new();
        let pending = self.pending.read().await;
        for item in pending.values() {
            if let Ok(guard) = item.try_lock() {
                list.push(guard.request.clone());
            }
        }
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_request_and_approval_flow() {
        let gate = HumanGate::new();
        let gate_clone = gate.clone();
        let session_id = SessionId::generate();

        let handle = tokio::spawn(async move {
            gate_clone
                .request_and_wait(
                    session_id,
                    None,
                    "Do you approve executing rm -rf?".into(),
                    None,
                    Duration::from_secs(5),
                )
                .await
        });

        // Give the task a moment to register
        tokio::time::sleep(Duration::from_millis(50)).await;

        let pending = gate.list_pending().await;
        assert_eq!(pending.len(), 1);
        let req_id = &pending[0].id;

        let claim = gate.claim_request(req_id).await.unwrap();
        claim
            .submit(HumanGateOutcome::Approved(Value::Bool(true)))
            .unwrap();

        let outcome = handle.await.unwrap().unwrap();
        assert_eq!(outcome, HumanGateOutcome::Approved(Value::Bool(true)));

        // Verify cleared
        assert_eq!(gate.list_pending().await.len(), 0);
    }

    #[tokio::test]
    async fn test_request_timeout() {
        let gate = HumanGate::new();
        let session_id = SessionId::generate();

        let res = gate
            .request_and_wait(
                session_id,
                None,
                "Slow user prompt".into(),
                None,
                Duration::from_millis(100),
            )
            .await;

        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("timed out"));
    }
}
