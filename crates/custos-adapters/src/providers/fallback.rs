//! Provider Fallback and Cooldown Router
//!
//! Implements multi-tier provider failover and rate-limit backoff.
//! Absorbs 9Router's `accountFallback.js` routing logic into native Custos Rust.

use async_trait::async_trait;
use custos_domain::DomainError;
use custos_provider::port::ModelProvider;
use custos_provider::request::{ModelResponse, ProviderRequest};
use custos_provider::turn::{ModelTurnEvent, ModelTurnRequest};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn};

#[derive(Clone)]
pub struct ProviderCandidate {
    pub id: String,
    pub priority: u32,
    pub provider: Arc<dyn ModelProvider>,
}

#[derive(Clone, Default)]
pub struct CooldownTracker {
    cooldowns: Arc<RwLock<HashMap<String, Instant>>>,
}

impl CooldownTracker {
    pub fn new() -> Self {
        Self {
            cooldowns: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Mark a route as cooled down until now + duration
    pub async fn mark_cooldown(&self, route_id: &str, duration: Duration) {
        let until = Instant::now() + duration;
        let mut guard = self.cooldowns.write().await;
        guard.insert(route_id.to_string(), until);
        warn!("Provider route '{}' put in cooldown for {:?}", route_id, duration);
    }

    /// Check if a route is currently available (not in cooldown)
    pub async fn is_available(&self, route_id: &str) -> bool {
        let guard = self.cooldowns.read().await;
        if let Some(until) = guard.get(route_id) {
            Instant::now() >= *until
        } else {
            true
        }
    }

    /// Reset cooldown for a route
    pub async fn clear(&self, route_id: &str) {
        let mut guard = self.cooldowns.write().await;
        guard.remove(route_id);
    }
}

pub struct FallbackRouter {
    candidates: Vec<ProviderCandidate>,
    tracker: CooldownTracker,
}

impl FallbackRouter {
    pub fn new(mut candidates: Vec<ProviderCandidate>) -> Self {
        candidates.sort_by_key(|c| c.priority);
        Self {
            candidates,
            tracker: CooldownTracker::new(),
        }
    }

    pub fn tracker(&self) -> CooldownTracker {
        self.tracker.clone()
    }
}

#[async_trait]
impl ModelProvider for FallbackRouter {
    fn provider_id(&self) -> &str {
        "fallback_router"
    }

    async fn generate(&self, req: &ProviderRequest) -> Result<ModelResponse, DomainError> {
        let mut last_err = None;

        for candidate in &self.candidates {
            if !self.tracker.is_available(&candidate.id).await {
                info!("Skipping candidate '{}' due to active cooldown", candidate.id);
                continue;
            }

            match candidate.provider.generate(req).await {
                Ok(resp) => {
                    self.tracker.clear(&candidate.id).await;
                    return Ok(resp);
                }
                Err(err) => {
                    warn!("Candidate '{}' failed with error: {}", candidate.id, err);
                    self.tracker
                        .mark_cooldown(&candidate.id, Duration::from_secs(30))
                        .await;
                    last_err = Some(err);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| {
            DomainError::Validation("All provider candidates exhausted or in cooldown".to_string())
        }))
    }

    async fn execute_turn(
        &self,
        req: ModelTurnRequest,
    ) -> Result<mpsc::Receiver<ModelTurnEvent>, DomainError> {
        let mut last_err = None;

        for candidate in &self.candidates {
            if !self.tracker.is_available(&candidate.id).await {
                info!("Skipping candidate '{}' due to active cooldown", candidate.id);
                continue;
            }

            match candidate.provider.execute_turn(req.clone()).await {
                Ok(mut rx) => {
                    info!("Successfully routed turn to candidate '{}'", candidate.id);
                    self.tracker.clear(&candidate.id).await;

                    // Ensure events are stamped with the actual executing model and provider (Zero Silent Fallback)
                    let (tx, proxy_rx) = mpsc::channel(16);
                    let candidate_id = candidate.id.clone();
                    let candidate_provider = candidate.provider.provider_id().to_string();

                    tokio::spawn(async move {
                        while let Some(mut event) = rx.recv().await {
                            if event.actual_model.is_none() {
                                event.actual_model = Some(candidate_id.clone());
                            }
                            if event.actual_provider.is_none() {
                                event.actual_provider = Some(candidate_provider.clone());
                            }
                            if tx.send(event).await.is_err() {
                                break;
                            }
                        }
                    });

                    return Ok(proxy_rx);
                }
                Err(err) => {
                    warn!("Candidate '{}' execute_turn failed: {}", candidate.id, err);
                    self.tracker
                        .mark_cooldown(&candidate.id, Duration::from_secs(30))
                        .await;
                    last_err = Some(err);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| {
            DomainError::Validation("All provider candidates exhausted or in cooldown".to_string())
        }))
    }
}
