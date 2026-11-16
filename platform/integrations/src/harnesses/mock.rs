//! Mock execution fixtures and responses for harness testing.

use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Default)]
pub struct MockHarnessState {
    pub enabled: bool,
    pub responses: Arc<Mutex<Vec<String>>>,
}

impl MockHarnessState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enabled() -> Self {
        Self {
            enabled: true,
            responses: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn push_response(&self, response: impl Into<String>) {
        self.responses.lock().await.push(response.into());
    }

    pub async fn next_response(&self, default_output: &str) -> String {
        let mut resps = self.responses.lock().await;
        if resps.is_empty() {
            default_output.to_string()
        } else {
            resps.remove(0)
        }
    }
}
