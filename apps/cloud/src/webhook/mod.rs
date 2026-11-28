//! Zero-Knowledge Webhook Scrubber & Content-Free Wake Router.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

const MAX_JSON_DEPTH: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookWakeSignal {
    pub host_id: String,
    pub workbench_id: String,
    pub source: String,
    pub event_key: String,
}

/// RFC 2104 HMAC-SHA256 without extra dependencies.
pub fn hmac_sha256(secret: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut key_block = [0u8; 64];
    if secret.len() > 64 {
        let hashed = Sha256::digest(secret);
        key_block[..32].copy_from_slice(&hashed);
    } else {
        key_block[..secret.len()].copy_from_slice(secret);
    }
    let mut o_pad = [0x5cu8; 64];
    let mut i_pad = [0x36u8; 64];
    for i in 0..64 {
        o_pad[i] ^= key_block[i];
        i_pad[i] ^= key_block[i];
    }
    let mut inner = Sha256::new();
    inner.update(i_pad);
    inner.update(msg);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(o_pad);
    outer.update(inner_hash);
    let mut out = [0u8; 32];
    out.copy_from_slice(&outer.finalize());
    out
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn hex_encode_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

pub struct WebhookEnvelopeScrubber;

impl WebhookEnvelopeScrubber {
    pub fn verify_github_signature(secret: &[u8], body: &[u8], header_sig: &str) -> bool {
        let Some(hex_sig) = header_sig.trim().strip_prefix("sha256=") else {
            return false;
        };
        let expected = hex_encode_lower(&hmac_sha256(secret, body));
        constant_time_eq(expected.as_bytes(), hex_sig.to_ascii_lowercase().as_bytes())
    }

    pub fn verify_slack_signature(
        signing_secret: &[u8],
        timestamp: &str,
        body: &[u8],
        header_sig: &str,
    ) -> bool {
        let Some(hex_sig) = header_sig.trim().strip_prefix("v0=") else {
            return false;
        };
        let mut signed = Vec::with_capacity(4 + timestamp.len() + body.len());
        signed.extend_from_slice(b"v0:");
        signed.extend_from_slice(timestamp.as_bytes());
        signed.push(b':');
        signed.extend_from_slice(body);
        let expected = hex_encode_lower(&hmac_sha256(signing_secret, &signed));
        constant_time_eq(expected.as_bytes(), hex_sig.to_ascii_lowercase().as_bytes())
    }

    /// Strips credential and signature headers before any envelope storage.
    pub fn scrub_headers(headers: &[(String, String)]) -> Vec<(String, String)> {
        headers
            .iter()
            .filter(|(k, _)| {
                let lower = k.to_ascii_lowercase();
                !matches!(
                    lower.as_str(),
                    "authorization"
                        | "cookie"
                        | "x-hub-signature-256"
                        | "x-slack-signature"
                        | "linear-signature"
                )
            })
            .cloned()
            .collect()
    }
}

#[derive(Default)]
pub struct WebhookWakeRouter {
    seen_keys: Mutex<HashSet<String>>,
    pending_by_host: Mutex<HashMap<String, Vec<WebhookWakeSignal>>>,
}

impl WebhookWakeRouter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Iterative depth check to reject deeply nested JSON bombs before matching.
    pub fn validate_json_depth(val: &serde_json::Value) -> bool {
        let mut stack = vec![(val, 1usize)];
        while let Some((node, depth)) = stack.pop() {
            if depth > MAX_JSON_DEPTH {
                return false;
            }
            match node {
                serde_json::Value::Array(items) => {
                    for item in items {
                        stack.push((item, depth + 1));
                    }
                }
                serde_json::Value::Object(map) => {
                    for item in map.values() {
                        stack.push((item, depth + 1));
                    }
                }
                _ => {}
            }
        }
        true
    }

    pub fn enqueue(&self, signal: WebhookWakeSignal) -> Result<bool, String> {
        {
            let mut seen = self.seen_keys.lock().map_err(|_| "lock poisoned".to_string())?;
            if !seen.insert(signal.event_key.clone()) {
                return Ok(false);
            }
        }
        let mut pending = self.pending_by_host.lock().map_err(|_| "lock poisoned".to_string())?;
        pending.entry(signal.host_id.clone()).or_default().push(signal);
        Ok(true)
    }

    pub fn drain_host(&self, host_id: &str) -> Vec<WebhookWakeSignal> {
        self.pending_by_host
            .lock()
            .ok()
            .and_then(|mut map| map.remove(host_id))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webhook_scrubber_verifies_github_and_slack_hmac_and_strips_creds() {
        let secret = b"whsec_test_123";
        let body = br#"{"action":"opened"}"#;
        let gh_sig = format!("sha256={}", hex_encode_lower(&hmac_sha256(secret, body)));
        assert!(WebhookEnvelopeScrubber::verify_github_signature(secret, body, &gh_sig));
        assert!(!WebhookEnvelopeScrubber::verify_github_signature(secret, body, "sha256=deadbeef"));

        let ts = "1700000000";
        let slack_msg = format!("v0:{ts}:{}", std::str::from_utf8(body).unwrap());
        let slack_sig = format!("v0={}", hex_encode_lower(&hmac_sha256(secret, slack_msg.as_bytes())));
        assert!(WebhookEnvelopeScrubber::verify_slack_signature(secret, ts, body, &slack_sig));

        let scrubbed = WebhookEnvelopeScrubber::scrub_headers(&[
            ("Authorization".into(), "Bearer secret".into()),
            ("X-GitHub-Event".into(), "pull_request".into()),
        ]);
        assert_eq!(scrubbed.len(), 1);
        assert_eq!(scrubbed[0].0, "X-GitHub-Event");
    }
}
