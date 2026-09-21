//! Span Orchestration Service
//!
//! Manages atomic execution steps (spans) and continuation state packets.

use crate::ports::TaskStore;
use custos_core_domain::{new_id, ContinuationPacket, DomainError, Span};
use std::sync::Arc;

pub struct SpanService {
    store: Arc<dyn TaskStore>,
}

impl SpanService {
    pub fn new(store: Arc<dyn TaskStore>) -> Self {
        Self { store }
    }

    pub async fn start_span(
        &self,
        task_id: &str,
        span_num: u32,
        provider: String,
        model: String,
        input_digest: String,
    ) -> Result<Span, DomainError> {
        let span_id = new_id("span");
        let span = Span::new(
            span_id,
            task_id.to_string(),
            span_num,
            provider,
            model,
            input_digest,
        );
        self.store.save_span(&span).await?;
        Ok(span)
    }

    pub async fn complete_span(
        &self,
        span_id: &str,
        output_digest: String,
    ) -> Result<Span, DomainError> {
        let mut span = self
            .store
            .get_span(span_id)
            .await?
            .ok_or_else(|| DomainError::Validation(format!("Span {span_id} not found")))?;

        span.complete(output_digest)?;
        self.store.save_span(&span).await?;
        Ok(span)
    }

    pub async fn record_continuation(&self, packet: ContinuationPacket) -> Result<(), DomainError> {
        packet.verify()?;
        self.store.save_continuation(&packet).await?;
        Ok(())
    }
}
