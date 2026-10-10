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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackErrorClass {
    /// Credential, rate-limit, or capacity limits (HTTP 429, 503, 401, 402, 403, timeouts)
    /// Triggers cooldown and fallback to the next candidate.
    CredentialOrCapacity,
    /// Request-scoped client errors (HTTP 400, validation, bad params)
    /// Must NOT cooldown healthy credentials to avoid false lockout.
    RequestScoped,
}

/// Classify domain and provider errors to determine if route cooldown is appropriate
pub fn classify_fallback_error(err: &DomainError) -> FallbackErrorClass {
    let msg = err.to_string().to_lowercase();
    if msg.contains("429")
        || msg.contains("rate limit")
        || msg.contains("quota")
        || msg.contains("overloaded")
        || msg.contains("503")
        || msg.contains("timeout")
        || msg.contains("401")
        || msg.contains("unauthorized")
    {
        FallbackErrorClass::CredentialOrCapacity
    } else if msg.contains("validation")
        || msg.contains("invalid parameter")
        || msg.contains("context length exceeded")
        || msg.contains("400")
        || msg.contains("bad request")
    {
        FallbackErrorClass::RequestScoped
    } else {
        FallbackErrorClass::CredentialOrCapacity
    }
}

#[derive(Debug, Clone)]
struct RouteCooldownEntry {
    until: Instant,
    backoff_level: u32,
}

#[derive(Clone, Default)]
pub struct CooldownTracker {
    cooldowns: Arc<RwLock<HashMap<String, RouteCooldownEntry>>>,
}

impl CooldownTracker {
    pub fn new() -> Self {
        Self {
            cooldowns: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record a failure with exponential backoff calculation (1s, 2s, 4s, 8s, up to 240s)
    pub async fn record_failure(&self, route_id: &str) -> Duration {
        let mut guard = self.cooldowns.write().await;
        let entry = guard.entry(route_id.to_string()).or_insert(RouteCooldownEntry {
            until: Instant::now(),
            backoff_level: 0,
        });

        entry.backoff_level = (entry.backoff_level + 1).min(8);
        let base_secs = 2u64.pow(entry.backoff_level.saturating_sub(1));
        let duration = Duration::from_secs(base_secs.min(240));

        entry.until = Instant::now() + duration;
        warn!(
            "Provider route '{}' penalized (level {}), cooldown for {:?}",
            route_id, entry.backoff_level, duration
        );
        duration
    }

    /// Mark a route as cooled down until now + duration
    pub async fn mark_cooldown(&self, route_id: &str, duration: Duration) {
        let mut guard = self.cooldowns.write().await;
        let entry = guard.entry(route_id.to_string()).or_insert(RouteCooldownEntry {
            until: Instant::now(),
            backoff_level: 1,
        });
        entry.until = Instant::now() + duration;
        warn!("Provider route '{}' put in cooldown for {:?}", route_id, duration);
    }

    /// Check if a route is currently available (not in cooldown)
    pub async fn is_available(&self, route_id: &str) -> bool {
        let guard = self.cooldowns.read().await;
        if let Some(entry) = guard.get(route_id) {
            Instant::now() >= entry.until
        } else {
            true
        }
    }

    /// Reset cooldown and backoff level for a route on successful request
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
                    match classify_fallback_error(&err) {
                        FallbackErrorClass::CredentialOrCapacity => {
                            self.tracker.record_failure(&candidate.id).await;
                            last_err = Some(err);
                        }
                        FallbackErrorClass::RequestScoped => {
                            // Request-scoped error must NOT lock out healthy candidate
                            return Err(err);
                        }
                    }
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
                    match classify_fallback_error(&err) {
                        FallbackErrorClass::CredentialOrCapacity => {
                            self.tracker.record_failure(&candidate.id).await;
                            last_err = Some(err);
                        }
                        FallbackErrorClass::RequestScoped => {
                            return Err(err);
                        }
                    }
                }
            }
        }

        Err(last_err.unwrap_or_else(|| {
            DomainError::Validation("All provider candidates exhausted or in cooldown".to_string())
        }))
    }
}

