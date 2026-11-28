//! Length-prefixed binary HostProofTranscript with 32-byte challenge_secret binding.

use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct HostProofTranscript {
    buf: Vec<u8>,
    challenge_secret: [u8; 32],
}

impl HostProofTranscript {
    pub fn new(challenge_secret: [u8; 32]) -> Self {
        Self {
            buf: Vec::new(),
            challenge_secret,
        }
    }

    pub fn append_field(&mut self, label: &str, value: &[u8]) {
        let label_bytes = label.as_bytes();
        self.buf.extend_from_slice(&(label_bytes.len() as u32).to_be_bytes());
        self.buf.extend_from_slice(label_bytes);
        self.buf.extend_from_slice(&(value.len() as u32).to_be_bytes());
        self.buf.extend_from_slice(value);
    }

    pub fn compute_digest(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.buf);
        hasher.update(self.challenge_secret);
        let result = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&result);
        out
    }

    pub fn verify_digest(&self, expected: &[u8; 32]) -> bool {
        &self.compute_digest() == expected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_proof_transcript_binds_length_prefixed_fields_and_challenge() {
        let secret = [7u8; 32];
        let mut t1 = HostProofTranscript::new(secret);
        t1.append_field("session_id", b"s1");
        t1.append_field("peer_id", b"p1");
        let d1 = t1.compute_digest();
        assert!(t1.verify_digest(&d1));

        let mut t2 = HostProofTranscript::new([8u8; 32]);
        t2.append_field("session_id", b"s1");
        t2.append_field("peer_id", b"p1");
        assert!(!t2.verify_digest(&d1));
    }
}

