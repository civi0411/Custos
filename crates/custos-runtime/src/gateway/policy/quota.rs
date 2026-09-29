use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct SessionQuotaLimiter {
    max_requests_per_minute: u32,
    window_history: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
}

impl SessionQuotaLimiter {
    pub fn new(max_requests_per_minute: u32) -> Self {
        Self {
            max_requests_per_minute,
            window_history: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn check_and_record(&self, session_id: &str) -> bool {
        let mut history = self.window_history.lock().await;
        let now = Instant::now();
        let one_minute_ago = now - std::time::Duration::from_secs(60);

        let timestamps = history.entry(session_id.to_string()).or_default();
        timestamps.retain(|&t| t > one_minute_ago);

        if timestamps.len() as u32 >= self.max_requests_per_minute {
            return false;
        }

        timestamps.push(now);
        true
    }
}
