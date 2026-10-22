//! In-Memory Memory Store Implementation
//!
//! Note: in-memory, lost on restart, test/bounded-cache only.
//! Implements both `MemoryStore` and canonical `MemoryPort` with project-scope isolation
//! and strict MAX_FACTS_PER_TIER capacity bounds.

use crate::memory_service::traits::{MemoryStore, MemoryTier};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use custos_core::contracts::memory::MemoryPort;
use custos_domain::{
    new_id, DomainError, Fact, FactProposal, MemoryEntry, MemoryError, MemoryScope, PersonalFact,
    ProposalReceipt, ProposalStatus, RecallQuery, WorkerRunId,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Maximum facts allowed per tier to prevent unbounded memory growth.
pub const MAX_FACTS_PER_TIER: usize = 1000;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TierKey {
    tier: MemoryTier,
    project_id: Option<String>,
}

/// Thread-safe in-memory memory store partitioned by MemoryTier and project scope.
///
/// Note: in-memory, lost on restart, test/bounded-cache only.
#[derive(Clone, Default)]
pub struct InMemoryMemoryStore {
    facts: Arc<RwLock<HashMap<TierKey, Vec<Fact>>>>,
    temporal_facts: Arc<RwLock<Vec<PersonalFact>>>,
}

impl InMemoryMemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores a fact scoped to a specific project.
    pub async fn remember_scoped(
        &self,
        tier: MemoryTier,
        project_id: Option<&str>,
        fact: Fact,
    ) -> Result<(), DomainError> {
        let key = TierKey {
            tier,
            project_id: project_id.map(ToString::to_string),
        };
        let mut map = self.facts.write().await;
        let tier_facts = map.entry(key).or_default();

        if tier_facts.len() >= MAX_FACTS_PER_TIER {
            return Err(DomainError::BudgetExceeded(format!(
                "Memory tier {:?} exceeded MAX_FACTS_PER_TIER limit of {}",
                tier, MAX_FACTS_PER_TIER
            )));
        }

        tier_facts.retain(|f| f.id != fact.id);
        tier_facts.push(fact);
        Ok(())
    }

    /// Recalls facts strictly scoped to a project (no cross-project leakage).
    pub async fn recall_scoped(
        &self,
        tier: MemoryTier,
        project_id: Option<&str>,
        query: &str,
    ) -> Result<Vec<Fact>, DomainError> {
        let key = TierKey {
            tier,
            project_id: project_id.map(ToString::to_string),
        };
        let map = self.facts.read().await;
        let query_lower = query.to_lowercase();
        let results = match map.get(&key) {
            Some(facts) => facts
                .iter()
                .filter(|f| {
                    query.is_empty()
                        || f.subject.to_lowercase().contains(&query_lower)
                        || f.predicate.to_lowercase().contains(&query_lower)
                        || f.object.to_lowercase().contains(&query_lower)
                })
                .cloned()
                .collect(),
            None => Vec::new(),
        };
        Ok(results)
    }
}

#[async_trait]
impl MemoryStore for InMemoryMemoryStore {
    async fn remember(&self, tier: MemoryTier, fact: Fact) -> Result<(), DomainError> {
        self.remember_scoped(tier, None, fact).await
    }

    async fn recall(&self, tier: MemoryTier, query: &str) -> Result<Vec<Fact>, DomainError> {
        self.recall_scoped(tier, None, query).await
    }
}

#[async_trait]
impl MemoryPort for InMemoryMemoryStore {
    async fn recall_context(
        &self,
        query: &RecallQuery,
        scope: &MemoryScope,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        let facts = self
            .recall_scoped(
                MemoryTier::Semantic,
                scope.project.as_deref(),
                &query.text,
            )
            .await
            .map_err(|e| MemoryError::Backend(e.to_string()))?;

        let mut entries: Vec<MemoryEntry> = facts
            .into_iter()
            .take(query.limit)
            .map(|f| MemoryEntry {
                id: f.id,
                content: format!("{} {} {}", f.subject, f.predicate, f.object),
                source: f.source,
                score: f.confidence,
                recorded_at: Utc::now(),
            })
            .collect();

        entries.truncate(query.limit);
        Ok(entries)
    }

    async fn recall_temporal_fact(
        &self,
        subject: &str,
        predicate: &str,
        at_time: DateTime<Utc>,
    ) -> Result<Option<PersonalFact>, MemoryError> {
        let facts = self.temporal_facts.read().await;
        let found = facts
            .iter()
            .find(|f| {
                f.subject == subject
                    && f.predicate == predicate
                    && f.valid_from <= at_time
                    && (f.valid_to.is_none() || at_time < f.valid_to.unwrap())
            })
            .cloned();
        Ok(found)
    }

    async fn propose_fact(
        &self,
        proposal: FactProposal,
        _worker_run_id: WorkerRunId,
    ) -> Result<ProposalReceipt, MemoryError> {
        let fact = Fact {
            id: new_id("fact"),
            subject: proposal.subject,
            predicate: proposal.predicate,
            object: proposal.object,
            confidence: proposal.confidence,
            source: "proposal".to_string(),
        };

        self.remember(MemoryTier::Working, fact)
            .await
            .map_err(|e| MemoryError::Rejected(e.to_string()))?;

        Ok(ProposalReceipt {
            proposal_id: new_id("prop"),
            status: ProposalStatus::Accepted,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_memory_store_lifecycle() {
        let store = InMemoryMemoryStore::new();

        let fact1 = Fact {
            id: "fact-1".into(),
            subject: "architecture".into(),
            predicate: "uses".into(),
            object: "rust".into(),
            confidence: 0.95,
            source: "repo_analysis".into(),
        };

        let fact2 = Fact {
            id: "fact-2".into(),
            subject: "database".into(),
            predicate: "is".into(),
            object: "sqlite".into(),
            confidence: 0.99,
            source: "repo_analysis".into(),
        };

        store.remember(MemoryTier::Semantic, fact1).await.unwrap();
        store.remember(MemoryTier::Semantic, fact2).await.unwrap();

        // Query by subject
        let recalled = store.recall(MemoryTier::Semantic, "arch").await.unwrap();
        assert_eq!(recalled.len(), 1);
        assert_eq!(recalled[0].subject, "architecture");

        // Query empty returns all for that tier
        let all = store.recall(MemoryTier::Semantic, "").await.unwrap();
        assert_eq!(all.len(), 2);

        // Different tier returns empty
        let working = store.recall(MemoryTier::Working, "").await.unwrap();
        assert_eq!(working.len(), 0);
    }

    #[tokio::test]
    async fn test_scope_isolation_prevents_cross_project_leakage() {
        let store = InMemoryMemoryStore::new();

        let fact_proj_a = Fact {
            id: "fact-a".into(),
            subject: "secret_module".into(),
            predicate: "belongs_to".into(),
            object: "Project Alpha".into(),
            confidence: 1.0,
            source: "alpha".into(),
        };

        let fact_proj_b = Fact {
            id: "fact-b".into(),
            subject: "public_module".into(),
            predicate: "belongs_to".into(),
            object: "Project Beta".into(),
            confidence: 1.0,
            source: "beta".into(),
        };

        store
            .remember_scoped(MemoryTier::Semantic, Some("project_a"), fact_proj_a)
            .await
            .unwrap();
        store
            .remember_scoped(MemoryTier::Semantic, Some("project_b"), fact_proj_b)
            .await
            .unwrap();

        // Project A cannot see Project B
        let recalled_a = store
            .recall_scoped(MemoryTier::Semantic, Some("project_a"), "")
            .await
            .unwrap();
        assert_eq!(recalled_a.len(), 1);
        assert_eq!(recalled_a[0].subject, "secret_module");

        // Project B cannot see Project A
        let recalled_b = store
            .recall_scoped(MemoryTier::Semantic, Some("project_b"), "")
            .await
            .unwrap();
        assert_eq!(recalled_b.len(), 1);
        assert_eq!(recalled_b[0].subject, "public_module");

        // MemoryPort interface recall_context respects scope
        let query = RecallQuery {
            text: "".into(),
            limit: 10,
            token_budget: None,
        };
        let scope_b = MemoryScope {
            task_id: None,
            project: Some("project_b".into()),
            include_personal: false,
        };
        let entries = store.recall_context(&query, &scope_b).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].content.contains("Project Beta"));
    }

    #[tokio::test]
    async fn test_over_limit_rejection() {
        let store = InMemoryMemoryStore::new();

        for i in 0..MAX_FACTS_PER_TIER {
            let fact = Fact {
                id: format!("fact-{i}"),
                subject: format!("subject-{i}"),
                predicate: "is".into(),
                object: "valid".into(),
                confidence: 1.0,
                source: "bench".into(),
            };
            store.remember(MemoryTier::Working, fact).await.unwrap();
        }

        // Inserting fact 1001 must be rejected
        let extra_fact = Fact {
            id: "fact-over-limit".into(),
            subject: "subject-over".into(),
            predicate: "is".into(),
            object: "rejected".into(),
            confidence: 1.0,
            source: "bench".into(),
        };

        let err = store.remember(MemoryTier::Working, extra_fact).await.unwrap_err();
        assert!(matches!(err, DomainError::BudgetExceeded(_)));
    }
}
