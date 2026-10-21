use crate::memory_service::traits::{MemoryStore, MemoryTier};
use async_trait::async_trait;
use custos_domain::{DomainError, Fact};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Thread-safe in-memory memory store partitioned by MemoryTier.
#[derive(Clone, Default)]
pub struct InMemoryMemoryStore {
    facts: Arc<RwLock<HashMap<MemoryTier, Vec<Fact>>>>,
}

impl InMemoryMemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl MemoryStore for InMemoryMemoryStore {
    async fn remember(&self, tier: MemoryTier, fact: Fact) -> Result<(), DomainError> {
        let mut map = self.facts.write().await;
        let tier_facts = map.entry(tier).or_default();
        tier_facts.retain(|f| f.id != fact.id);
        tier_facts.push(fact);
        Ok(())
    }

    async fn recall(&self, tier: MemoryTier, query: &str) -> Result<Vec<Fact>, DomainError> {
        let map = self.facts.read().await;
        let query_lower = query.to_lowercase();
        let results = match map.get(&tier) {
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
}
