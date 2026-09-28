use custos_domain::{DomainError, SessionStatus};

pub struct SessionStateMachine;

impl SessionStateMachine {
    pub fn transition(
        current: &SessionStatus,
        next: SessionStatus,
    ) -> Result<SessionStatus, DomainError> {
        match (current, &next) {
            // Active transitions
            (SessionStatus::Active, SessionStatus::Paused) => Ok(next),
            (SessionStatus::Active, SessionStatus::Promoted { .. }) => Ok(next),
            (SessionStatus::Active, SessionStatus::Closed) => Ok(next),

            // Paused transitions
            (SessionStatus::Paused, SessionStatus::Active) => Ok(next),
            (SessionStatus::Paused, SessionStatus::Closed) => Ok(next),

            // Promoted transitions (sessions remain independent and can continue active dialogue or close)
            (SessionStatus::Promoted { .. }, SessionStatus::Active) => Ok(next),
            (SessionStatus::Promoted { .. }, SessionStatus::Closed) => Ok(next),
            (SessionStatus::Promoted { task_id }, _) => Err(DomainError::InvalidStateTransition {
                from: format!("Promoted({task_id})"),
                to: format!("{next:?}"),
            }),

            // Terminal state: Closed cannot transition
            (SessionStatus::Closed, _) => Err(DomainError::InvalidStateTransition {
                from: "Closed".to_string(),
                to: format!("{next:?}"),
            }),

            _ => Err(DomainError::InvalidStateTransition {
                from: format!("{current:?}"),
                to: format!("{next:?}"),
            }),
        }
    }
}
