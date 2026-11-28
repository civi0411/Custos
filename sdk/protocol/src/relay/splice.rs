//! 9-step monotonic splice state machine for E2EE relay sessions.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpliceState {
    PreAuthAdmitted = 0,
    CredentialLeaseReserved = 1,
    HostNotified = 2,
    AttachPending = 3,
    HostAttached = 4,
    ClientAcknowledged = 5,
    Spliced = 6,
    E2eeConfirmable = 7,
    Teardown = 8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpliceStateMachine {
    pub session_id: String,
    state: SpliceState,
    host_attached: bool,
    client_attached: bool,
}

impl SpliceStateMachine {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            state: SpliceState::PreAuthAdmitted,
            host_attached: false,
            client_attached: false,
        }
    }

    pub fn state(&self) -> SpliceState {
        self.state
    }

    pub fn mark_host_attached(&mut self) {
        self.host_attached = true;
    }

    pub fn mark_client_attached(&mut self) {
        self.client_attached = true;
    }

    /// Advances monotonically forward; `ClientAcknowledged` requires both forwarding legs attached.
    pub fn advance_to(&mut self, next: SpliceState) -> Result<SpliceState, String> {
        if next == SpliceState::Teardown {
            self.state = SpliceState::Teardown;
            return Ok(self.state);
        }
        if (next as u8) != (self.state as u8) + 1 {
            return Err(format!(
                "Non-monotonic splice transition from {:?} to {:?}",
                self.state, next
            ));
        }
        if next == SpliceState::ClientAcknowledged && (!self.host_attached || !self.client_attached)
        {
            return Err(
                "Cannot emit ClientAcknowledged before both host and client forwarding legs are attached"
                    .into(),
            );
        }
        self.state = next;
        Ok(self.state)
    }

    pub fn is_ciphertext_only(&self) -> bool {
        matches!(self.state, SpliceState::E2eeConfirmable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splice_enforces_monotonic_9_step_and_bidirectional_attach() {
        let mut sm = SpliceStateMachine::new("rel-1");
        assert_eq!(sm.state(), SpliceState::PreAuthAdmitted);
        assert!(sm.advance_to(SpliceState::HostAttached).is_err());

        sm.advance_to(SpliceState::CredentialLeaseReserved).unwrap();
        sm.advance_to(SpliceState::HostNotified).unwrap();
        sm.advance_to(SpliceState::AttachPending).unwrap();
        sm.mark_host_attached();
        sm.advance_to(SpliceState::HostAttached).unwrap();

        // Client leg not attached yet -> rejected
        assert!(sm.advance_to(SpliceState::ClientAcknowledged).is_err());

        sm.mark_client_attached();
        sm.advance_to(SpliceState::ClientAcknowledged).unwrap();
        sm.advance_to(SpliceState::Spliced).unwrap();
        sm.advance_to(SpliceState::E2eeConfirmable).unwrap();
        assert!(sm.is_ciphertext_only());

        sm.advance_to(SpliceState::Teardown).unwrap();
        assert_eq!(sm.state(), SpliceState::Teardown);
    }
}

