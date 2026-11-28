//! Proactive Look-Around Mechanical Read-Only Sandbox & 14d/30d Suggestion Deduplication.

use std::collections::HashMap;

use custos_contracts::authority::action::RiskLevel;
use custos_contracts::oi::{PersonaItem, PersonaItemBucket, PersonaItemKind};
use custos_contracts::{new_id, DomainError};

const READ_ONLY_CAPABILITIES: &[&str] = &["fs_read", "read_only", "git_read"];
pub const ACCEPTED_RECALL_DAYS: i64 = 14;
pub const DISMISSED_BLOCK_DAYS: i64 = 30;

/// Mechanically rejects any mutating capability or `RiskLevel::Medium+` during Look-Around / Tidy-Up runs.
pub struct LookAroundSandboxPolicy;

impl LookAroundSandboxPolicy {
    pub fn enforce_read_only(
        requested_capabilities: &[String],
        requested_risk: RiskLevel,
    ) -> Result<Vec<String>, DomainError> {
        if !matches!(requested_risk, RiskLevel::Low) {
            return Err(DomainError::Conflict(format!(
                "Look-Around sandbox blocks non-low risk intent: {requested_risk:?}"
            )));
        }
        for cap in requested_capabilities {
            if !READ_ONLY_CAPABILITIES.contains(&cap.as_str()) {
                return Err(DomainError::Conflict(format!(
                    "Look-Around sandbox blocks non-read-only capability '{cap}'"
                )));
            }
        }
        if requested_capabilities.is_empty() {
            Ok(vec!["fs_read".to_string()])
        } else {
            Ok(requested_capabilities.to_vec())
        }
    }
}

#[derive(Debug, Clone)]
pub struct LookAroundSuggestion {
    pub dedup_key: String,
    pub title: String,
    pub summary: String,
    pub prompt_ask: String,
    pub priority: i32,
}

/// Prevents Up-Next suggestion spam using 14-day accepted recall and 30-day dismissal blocklist.
#[derive(Debug, Default)]
pub struct LookAroundDedupFilter {
    pub accepted_at_epoch_ms: HashMap<String, i64>,
    pub dismissed_at_epoch_ms: HashMap<String, i64>,
}

impl LookAroundDedupFilter {
    pub fn is_suppressed(&self, dedup_key: &str, now_ms: i64) -> bool {
        let day_ms = 86_400_000_i64;
        if let Some(&ts) = self.accepted_at_epoch_ms.get(dedup_key) {
            if now_ms.saturating_sub(ts) < ACCEPTED_RECALL_DAYS * day_ms {
                return true;
            }
        }
        if let Some(&ts) = self.dismissed_at_epoch_ms.get(dedup_key) {
            if now_ms.saturating_sub(ts) < DISMISSED_BLOCK_DAYS * day_ms {
                return true;
            }
        }
        false
    }

    pub fn filter_suggestions(
        &self,
        suggestions: Vec<LookAroundSuggestion>,
        now_ms: i64,
        max_items: usize,
    ) -> Vec<LookAroundSuggestion> {
        suggestions
            .into_iter()
            .filter(|s| !self.is_suppressed(&s.dedup_key, now_ms))
            .take(max_items)
            .collect()
    }
}

pub fn build_up_next_item(
    persona_id: &str,
    suggestion: &LookAroundSuggestion,
    now_rfc3339: &str,
) -> PersonaItem {
    PersonaItem {
        id: new_id("pitem"),
        persona_id: persona_id.to_string(),
        bucket: PersonaItemBucket::UpNext,
        kind: PersonaItemKind::LookAroundFinding,
        title: suggestion.title.clone(),
        summary: suggestion.summary.clone(),
        ask_questions: Vec::new(),
        source_ref: None,
        task_id: None,
        run_id: None,
        priority: suggestion.priority,
        payload_json: serde_json::json!({
            "dedup_key": suggestion.dedup_key,
            "ask": suggestion.prompt_ask,
        }),
        created_at: now_rfc3339.to_string(),
        updated_at: now_rfc3339.to_string(),
        resolved_at: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sandbox_rejects_mutations_and_allows_read_only() {
        assert!(LookAroundSandboxPolicy::enforce_read_only(
            &["fs_read".into()],
            RiskLevel::Low
        )
        .is_ok());

        let write_err = LookAroundSandboxPolicy::enforce_read_only(
            &["fs_write".into()],
            RiskLevel::Low,
        )
        .unwrap_err();
        assert!(matches!(write_err, DomainError::Conflict(_)));

        let risk_err = LookAroundSandboxPolicy::enforce_read_only(
            &["fs_read".into()],
            RiskLevel::Medium,
        )
        .unwrap_err();
        assert!(matches!(risk_err, DomainError::Conflict(_)));
    }

    #[test]
    fn dedup_filter_enforces_14d_accepted_and_30d_dismissed() {
        let mut filter = LookAroundDedupFilter::default();
        let day_ms = 86_400_000_i64;
        let now = 100 * day_ms;
        filter.accepted_at_epoch_ms.insert("k_acc".into(), now - 5 * day_ms);
        filter.dismissed_at_epoch_ms.insert("k_dis".into(), now - 20 * day_ms);
        filter.dismissed_at_epoch_ms.insert("k_old".into(), now - 31 * day_ms);

        assert!(filter.is_suppressed("k_acc", now));
        assert!(filter.is_suppressed("k_dis", now));
        assert!(!filter.is_suppressed("k_old", now));
    }
}

