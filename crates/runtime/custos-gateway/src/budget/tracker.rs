use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Default, Clone)]
pub struct UsageRecord {
    pub total_tokens: u64,
    pub total_cost_usd: f32,
    pub call_count: u32,
}

#[derive(Clone, Default)]
pub struct BudgetTracker {
    task_usage: Arc<RwLock<HashMap<String, UsageRecord>>>,
    session_usage: Arc<RwLock<HashMap<String, UsageRecord>>>,
}

impl BudgetTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn record(
        &self,
        task_id: Option<&str>,
        session_id: &str,
        tokens_used: u64,
        cost_usd: f32,
    ) {
        if let Some(tid) = task_id {
            let mut tasks = self.task_usage.write().await;
            let entry = tasks.entry(tid.to_string()).or_default();
            entry.total_tokens = entry.total_tokens.saturating_add(tokens_used);
            entry.total_cost_usd += cost_usd;
            entry.call_count += 1;
        }

        let mut sessions = self.session_usage.write().await;
        let entry = sessions.entry(session_id.to_string()).or_default();
        entry.total_tokens = entry.total_tokens.saturating_add(tokens_used);
        entry.total_cost_usd += cost_usd;
        entry.call_count += 1;
    }

    pub async fn get_task_usage(&self, task_id: &str) -> UsageRecord {
        let tasks = self.task_usage.read().await;
        tasks.get(task_id).cloned().unwrap_or_default()
    }

    pub async fn get_session_usage(&self, session_id: &str) -> UsageRecord {
        let sessions = self.session_usage.read().await;
        sessions.get(session_id).cloned().unwrap_or_default()
    }
}
