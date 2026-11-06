//! Kernel FSM Observability and Transition Tracing

/// Hook for tracing and auditing FSM state transitions across kernel execution
pub struct KernelObservability;

impl KernelObservability {
    pub fn on_state_change(
        entity_id: &str,
        from_state: &str,
        to_state: &str,
        reason: Option<&str>,
    ) {
        tracing::info!(
            target: "custos::kernel::fsm",
            entity_id = %entity_id,
            from = %from_state,
            to = %to_state,
            reason = ?reason,
            "Kernel FSM state transition"
        );
    }
}

