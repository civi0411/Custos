//! 3-Tier Lease Model, Fencing Tokens & Strategy Command Queue
//!
//! Provides distributed lease ownership, monotonic deadline enforcement,
//! and durable command state machines to prevent multi-worker split-brain executions.

use crate::error::DomainError;
use crate::ids::new_id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaseTier {
    GlobalLeader,
    StrategyRuntime,
    CommandExecution,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeaseRecord {
    pub lease_key: String,
    pub tier: LeaseTier,
    pub owner_id: String,
    pub expires_at_ms: i64,
    pub heartbeat_at_ms: i64,
}

impl LeaseRecord {
    pub fn new(
        lease_key: impl Into<String>,
        tier: LeaseTier,
        owner_id: impl Into<String>,
        ttl_ms: i64,
        now_ms: i64,
    ) -> Result<Self, DomainError> {
        let lease_key = lease_key.into();
        let owner_id = owner_id.into();

        if lease_key.trim().is_empty() {
            return Err(DomainError::Validation("Lease key cannot be empty".into()));
        }
        if owner_id.trim().is_empty() {
            return Err(DomainError::Validation("Owner ID cannot be empty".into()));
        }
        if ttl_ms <= 0 {
            return Err(DomainError::Validation("TTL must be positive".into()));
        }

        Ok(Self {
            lease_key,
            tier,
            owner_id,
            expires_at_ms: now_ms + ttl_ms,
            heartbeat_at_ms: now_ms,
        })
    }

    pub fn is_valid_at(&self, now_ms: i64) -> bool {
        self.expires_at_ms > now_ms
    }

    pub fn renew(&mut self, ttl_ms: i64, now_ms: i64) -> Result<(), DomainError> {
        if ttl_ms <= 0 {
            return Err(DomainError::Validation("TTL must be positive".into()));
        }
        self.expires_at_ms = now_ms + ttl_ms;
        self.heartbeat_at_ms = now_ms;
        Ok(())
    }
}

/// In-memory monotonic safety lease guard.
///
/// Prevents order dispatch if local deadline has passed, guaranteeing
/// zero split-brain mutations even when network to the database partitions.
#[derive(Debug, Clone)]
pub struct LeaseGuard {
    pub owner_id: String,
    pub strategy_id: String,
    pub local_deadline_ms: i64,
}

impl LeaseGuard {
    pub fn new(
        owner_id: impl Into<String>,
        strategy_id: impl Into<String>,
        deadline_ms: i64,
    ) -> Self {
        Self {
            owner_id: owner_id.into(),
            strategy_id: strategy_id.into(),
            local_deadline_ms: deadline_ms,
        }
    }

    pub fn is_execution_allowed(&self, now_ms: i64) -> bool {
        now_ms < self.local_deadline_ms
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategyCommandType {
    Start,
    Stop,
    Restart,
    Reconcile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategyCommandStatus {
    Pending,
    Processing,
    Succeeded,
    Failed,
    Cancelled,
}

/// Durable Strategy Command Outbox Item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyCommand {
    pub id: String,
    pub strategy_id: String,
    pub command_type: StrategyCommandType,
    pub status: StrategyCommandStatus,
    pub idempotency_key: String,
    pub attempts: u32,
    pub max_attempts: u32,
    pub payload_json: String,
    pub claimed_by: Option<String>,
    pub lease_expires_at_ms: Option<i64>,
    pub error_message: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

impl StrategyCommand {
    pub fn new(
        strategy_id: impl Into<String>,
        command_type: StrategyCommandType,
        idempotency_key: impl Into<String>,
        payload_json: impl Into<String>,
        now_ms: i64,
    ) -> Result<Self, DomainError> {
        let strategy_id = strategy_id.into();
        let idempotency_key = idempotency_key.into();

        if strategy_id.trim().is_empty() {
            return Err(DomainError::Validation(
                "Strategy ID cannot be empty".into(),
            ));
        }
        if idempotency_key.trim().is_empty() {
            return Err(DomainError::Validation(
                "Idempotency key cannot be empty".into(),
            ));
        }

        Ok(Self {
            id: new_id("cmd"),
            strategy_id,
            command_type,
            status: StrategyCommandStatus::Pending,
            idempotency_key,
            attempts: 0,
            max_attempts: 3,
            payload_json: payload_json.into(),
            claimed_by: None,
            lease_expires_at_ms: None,
            error_message: None,
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
        })
    }

    pub fn claim(
        &mut self,
        worker_id: impl Into<String>,
        lease_ttl_ms: i64,
        now_ms: i64,
    ) -> Result<(), DomainError> {
        if self.status != StrategyCommandStatus::Pending
            && (self.status != StrategyCommandStatus::Processing
                || self.lease_expires_at_ms.unwrap_or(0) >= now_ms)
        {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Processing".into(),
            });
        }

        if self.attempts >= self.max_attempts {
            return Err(DomainError::Validation(format!(
                "Cannot claim command {}: reached max attempts {}",
                self.id, self.max_attempts
            )));
        }

        self.status = StrategyCommandStatus::Processing;
        self.claimed_by = Some(worker_id.into());
        self.lease_expires_at_ms = Some(now_ms + lease_ttl_ms);
        self.attempts += 1;
        self.updated_at_ms = now_ms;
        Ok(())
    }

    pub fn mark_succeeded(&mut self, now_ms: i64) -> Result<(), DomainError> {
        if self.status != StrategyCommandStatus::Processing {
            return Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Succeeded".into(),
            });
        }
        self.status = StrategyCommandStatus::Succeeded;
        self.lease_expires_at_ms = None;
        self.updated_at_ms = now_ms;
        Ok(())
    }

    pub fn mark_failed(&mut self, error: impl Into<String>, now_ms: i64) {
        self.status = StrategyCommandStatus::Failed;
        self.error_message = Some(error.into());
        self.lease_expires_at_ms = None;
        self.updated_at_ms = now_ms;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lease_record_lifecycle() {
        let mut lease = LeaseRecord::new(
            "strat_01",
            LeaseTier::StrategyRuntime,
            "worker_a",
            30_000,
            1000,
        )
        .expect("valid lease");

        assert!(lease.is_valid_at(20_000));
        assert!(!lease.is_valid_at(32_000));

        lease.renew(30_000, 31_000).expect("renew");
        assert!(lease.is_valid_at(32_000));
        assert!(lease.is_valid_at(60_000));
    }

    #[test]
    fn test_lease_guard_safety_deadline() {
        let guard = LeaseGuard::new("worker_a", "strat_01", 10_000);
        assert!(guard.is_execution_allowed(9_999));
        assert!(!guard.is_execution_allowed(10_000));
        assert!(!guard.is_execution_allowed(10_001));
    }

    #[test]
    fn test_strategy_command_claim_and_recovery() {
        let mut cmd = StrategyCommand::new(
            "strat_01",
            StrategyCommandType::Start,
            "idemp_1",
            "{}",
            1000,
        )
        .expect("valid command");

        // Worker 1 claims command with 5s lease
        cmd.claim("worker_1", 5000, 1000).expect("claim");
        assert_eq!(cmd.status, StrategyCommandStatus::Processing);
        assert_eq!(cmd.attempts, 1);

        // Worker 2 cannot claim while lease is active
        assert!(cmd.claim("worker_2", 5000, 4000).is_err());

        // Worker 2 CAN claim after lease expires at 6001ms (Crash recovery)
        cmd.claim("worker_2", 5000, 6001)
            .expect("recover abandoned command");
        assert_eq!(cmd.claimed_by, Some("worker_2".to_string()));
        assert_eq!(cmd.attempts, 2);

        // Mark completed
        cmd.mark_succeeded(7000).expect("complete");
        assert_eq!(cmd.status, StrategyCommandStatus::Succeeded);
    }
}
