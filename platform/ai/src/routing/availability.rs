//! Model Availability & Transient Admission State
//!
//! Tracks account-scoped transient access, quota, rate-limit, and offline admission.
//! Integrates directly with canonical `ProviderObservationSnapshot` from `custos_domain`.

use custos_domain::provider::observation::{ProviderHealthStatus, ProviderObservationSnapshot};
use custos_domain::DomainError;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Transient account-scoped inference admission; discovery never clears quota exhaustion.
#[derive(Clone, Debug, Serialize)]
pub struct ModelUnavailable {
    pub reason: String,
    pub kind: String,
    pub retry_at: Option<i64>,
}

#[derive(Clone, Default)]
pub struct ModelAvailability {
    entries: Arc<RwLock<HashMap<(String, String), ModelUnavailable>>>,
}

impl ModelAvailability {
    pub fn snapshot(&self, scope: &str) -> Result<HashMap<String, ModelUnavailable>, DomainError> {
        let now = chrono::Utc::now().timestamp_millis();
        let entries = self.entries.read().map_err(|_| {
            DomainError::Validation("Model availability lock unavailable".into())
        })?;
        Ok(entries
            .iter()
            .filter(|((key, _), value)| {
                key == scope && value.retry_at.is_none_or(|time| time > now)
            })
            .map(|((_, model), value)| (model.clone(), value.clone()))
            .collect())
    }

    pub fn check(&self, scope: &str, model: &str) -> Result<(), DomainError> {
        let blocked = self.snapshot(scope)?;
        if let Some(reason) = blocked.get("*").or_else(|| blocked.get(model)) {
            return Err(DomainError::Validation(format!(
                "Model unavailable: {}",
                reason.reason
            )));
        }
        Ok(())
    }

    pub fn refresh(&self, scope: &str) -> Result<(), DomainError> {
        let mut entries = self.entries.write().map_err(|_| {
            DomainError::Validation("Model availability lock unavailable".into())
        })?;
        entries.retain(|(key, _), value| {
            key != scope || matches!(value.kind.as_str(), "quota" | "rate")
        });
        Ok(())
    }

    pub fn quota_recovered(&self, scope: &str) -> Result<(), DomainError> {
        self.entries
            .write()
            .map_err(|_| {
                DomainError::Validation("Model availability lock unavailable".into())
            })?
            .retain(|(key, _), value| key != scope || value.kind != "quota");
        Ok(())
    }

    pub fn observe(
        &self,
        scope: &str,
        model: &str,
        status: u16,
        body: &str,
        retry_at: Option<i64>,
    ) -> Result<(), DomainError> {
        let lower = body.to_ascii_lowercase();
        let value = serde_json::from_str::<serde_json::Value>(body).ok();
        let retry_at = value
            .as_ref()
            .and_then(|value| {
                value
                    .pointer("/error/resets_at")
                    .or_else(|| value.pointer("/error/reset_at"))
            })
            .and_then(|reset| {
                reset
                    .as_i64()
                    .map(|timestamp| {
                        if timestamp < 10_000_000_000 {
                            timestamp.saturating_mul(1000)
                        } else {
                            timestamp
                        }
                    })
                    .or_else(|| {
                        reset
                            .as_str()
                            .and_then(|text| chrono::DateTime::parse_from_rfc3339(text).ok())
                            .map(|time| time.timestamp_millis())
                    })
            })
            .or(retry_at);
        let code = value
            .as_ref()
            .and_then(|v| {
                v.pointer("/error/code")
                    .filter(|value| value.is_string())
                    .or_else(|| v.pointer("/error/type"))
                    .or_else(|| v.get("code"))
            })
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let (target, kind, reason) = if status == 0 {
            (
                "*",
                "offline",
                "Provider connection unavailable; refresh models to reconnect",
            )
        } else if status == 401 {
            (
                "*",
                "credentials",
                "Provider authentication failed; reconnect this account",
            )
        } else if status == 429
            && (matches!(
                code,
                "insufficient_quota"
                    | "credit_balance_exhausted"
                    | "quota_exhausted"
                    | "usage_limit_reached"
            ) || lower.contains("quota exhausted")
                || lower.contains("usage limit")
                || lower.contains("insufficient_quota"))
        {
            (
                "*",
                "quota",
                "Provider quota exhausted; refresh quota or wait for reset",
            )
        } else if status == 429 {
            (model, "rate", "Model temporarily rate limited")
        } else if matches!(status, 403 | 404)
            && (matches!(
                code,
                "model_not_found" | "model_not_supported" | "model_access_denied"
            ) || lower.contains("model not found")
                || lower.contains("model is not supported")
                || lower.contains("do not have access to model"))
        {
            (
                model,
                "access",
                "This account cannot access the model; refresh models",
            )
        } else if status == 403 {
            (
                "*",
                "access",
                "Provider denied access; refresh models or reconnect",
            )
        } else {
            return Ok(());
        };
        let retry_at = if kind == "rate" {
            Some(retry_at.unwrap_or_else(|| chrono::Utc::now().timestamp_millis() + 30_000))
        } else {
            retry_at
        };
        self.entries
            .write()
            .map_err(|_| {
                DomainError::Validation("Model availability lock unavailable".into())
            })?
            .insert(
                (scope.to_string(), target.to_string()),
                ModelUnavailable {
                    reason: reason.into(),
                    kind: kind.into(),
                    retry_at,
                },
            );
        Ok(())
    }

    /// Returns an atomic observation snapshot for the given scope and model.
    pub fn observation_snapshot(&self, scope: &str, model: &str) -> ProviderObservationSnapshot {
        let blocked = self.snapshot(scope).unwrap_or_default();
        let unavail = blocked.get("*").or_else(|| blocked.get(model));
        let status = match unavail {
            Some(u) => ProviderHealthStatus::Unavailable {
                kind: u.kind.clone(),
                reason: u.reason.clone(),
                retry_at: u.retry_at,
            },
            None => ProviderHealthStatus::Healthy,
        };
        ProviderObservationSnapshot {
            provider_id: scope.to_string(),
            model_id: Some(model.to_string()),
            status,
            latency_ema_ms: 0.0,
            total_requests: 0,
            total_failures: 0,
            last_attempt_at: None,
        }
    }

    /// Returns atomic observation snapshots for all tracked models in a given scope.
    pub fn all_observation_snapshots(&self, scope: &str) -> Vec<ProviderObservationSnapshot> {
        let blocked = match self.snapshot(scope) {
            Ok(b) => b,
            Err(_) => return Vec::new(),
        };
        blocked
            .into_iter()
            .map(|(model, u)| ProviderObservationSnapshot {
                provider_id: scope.to_string(),
                model_id: Some(model),
                status: ProviderHealthStatus::Unavailable {
                    kind: u.kind,
                    reason: u.reason,
                    retry_at: u.retry_at,
                },
                latency_ema_ms: 0.0,
                total_requests: 0,
                total_failures: 0,
                last_attempt_at: None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transient_blocks_are_scoped_and_prompt_errors_do_not_hide_models() {
        let state = ModelAvailability::default();
        state
            .observe(
                "A",
                "gpt",
                400,
                r#"{"error":{"code":"invalid_reasoning_effort"}}"#,
                None,
            )
            .unwrap();
        assert!(state.check("A", "gpt").is_ok());
        state
            .observe(
                "A",
                "gpt",
                404,
                r#"{"error":{"code":"model_not_found"}}"#,
                None,
            )
            .unwrap();
        assert!(state.check("A", "gpt").is_err());
        assert!(state.check("B", "gpt").is_ok());
        assert!(state.check("A", "another").is_ok());
        state.refresh("A").unwrap();
        assert!(state.check("A", "gpt").is_ok());
        state.observe("A", "gpt", 401, "", None).unwrap();
        assert!(state.check("A", "another").is_err());
        state.refresh("A").unwrap();
        state.observe("A", "gpt", 0, "", None).unwrap();
        assert!(state.check("A", "gpt").is_err());
        state.refresh("A").unwrap();
        assert!(state.check("A", "gpt").is_ok());
    }

    #[test]
    fn catalog_refresh_does_not_reset_quota_or_retry_window() {
        let state = ModelAvailability::default();
        state
            .observe(
                "A",
                "gpt",
                429,
                r#"{"error":{"code":"insufficient_quota"}}"#,
                None,
            )
            .unwrap();
        state.refresh("A").unwrap();
        assert!(state.check("A", "gpt").is_err());
        state.quota_recovered("A").unwrap();
        assert!(state.check("A", "gpt").is_ok());
        state
            .observe(
                "A",
                "gpt",
                429,
                r#"{"error":{"code":"rate_limit_exceeded"}}"#,
                None,
            )
            .unwrap();
        state.refresh("A").unwrap();
        assert_eq!(state.snapshot("A").unwrap()["gpt"].kind, "rate");
        assert!(state.check("A", "another").is_ok());
        state
            .observe(
                "A",
                "gpt",
                429,
                "rate",
                Some(chrono::Utc::now().timestamp_millis() - 1),
            )
            .unwrap();
        assert!(state.check("A", "gpt").is_ok());
    }

    #[test]
    fn test_model_availability_observation_snapshot() {
        let state = ModelAvailability::default();
        let obs = state.observation_snapshot("scope-1", "gpt-4o");
        assert!(obs.is_operable());
        assert_eq!(obs.provider_id, "scope-1");
        assert_eq!(obs.model_id.as_deref(), Some("gpt-4o"));

        state
            .observe(
                "scope-1",
                "gpt-4o",
                429,
                r#"{"error":{"code":"insufficient_quota"}}"#,
                None,
            )
            .unwrap();
        let blocked = state.observation_snapshot("scope-1", "gpt-4o");
        assert!(!blocked.is_operable());
    }
}

