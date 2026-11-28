use custos_protocol::relay::{HostProofTranscript, SpliceState, SpliceStateMachine};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Default)]
pub struct RelayService {
    sessions: Mutex<HashMap<String, SpliceStateMachine>>,
}

impl RelayService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn admit_session(&self, session_id: impl Into<String>) -> Result<SpliceState, String> {
        let id = session_id.into();
        let mut guard = self.sessions.lock().map_err(|_| "lock poisoned".to_string())?;
        let sm = SpliceStateMachine::new(id.clone());
        let state = sm.state();
        guard.insert(id, sm);
        Ok(state)
    }

    pub fn advance_session(&self, session_id: &str, next: SpliceState) -> Result<SpliceState, String> {
        let mut guard = self.sessions.lock().map_err(|_| "lock poisoned".to_string())?;
        let sm = guard
            .get_mut(session_id)
            .ok_or_else(|| format!("unknown relay session: {session_id}"))?;
        sm.advance_to(next)
    }

    pub fn verify_host_proof(
        session_id: &str,
        peer_id: &str,
        challenge_secret: [u8; 32],
        expected_digest: &[u8; 32],
    ) -> bool {
        let mut transcript = HostProofTranscript::new(challenge_secret);
        transcript.append_field("session_id", session_id.as_bytes());
        transcript.append_field("peer_id", peer_id.as_bytes());
        transcript.verify_digest(expected_digest)
    }
}

