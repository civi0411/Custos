//! Loopback-only credential-injecting proxy policy for MCP/Bot tool calls.
//!
//! Enforces three mechanical invariants learned from Orca's `BotToolRelay`:
//! 1. Rejects any request with an `Origin` header (prevents browser CSRF).
//! 2. Rejects any non-loopback `Host` header.
//! 3. Disables 3xx redirects (`Policy::none()`) so injected secrets never leak on redirect.

use reqwest::redirect::Policy;

pub struct CredentialInjectingToolProxy {
    client: reqwest::Client,
}

impl CredentialInjectingToolProxy {
    pub fn new() -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            .redirect(Policy::none())
            .build()?;
        Ok(Self { client })
    }

    pub fn validate_local_request(
        origin_header: Option<&str>,
        host_header: Option<&str>,
    ) -> Result<(), String> {
        if origin_header.is_some() {
            return Err("Rejected: Origin header is forbidden on CredentialInjectingToolProxy".into());
        }
        let host = host_header
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        let is_loopback = host == "127.0.0.1"
            || host.starts_with("127.0.0.1:")
            || host == "localhost"
            || host.starts_with("localhost:")
            || host == "[::1]"
            || host.starts_with("[::1]:");
        if !is_loopback {
            return Err(format!("Rejected non-loopback Host header: '{host}'"));
        }
        Ok(())
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_proxy_rejects_origin_and_non_loopback_host() {
        assert!(CredentialInjectingToolProxy::validate_local_request(
            Some("https://evil.test"),
            Some("127.0.0.1:8080")
        )
        .is_err());

        assert!(CredentialInjectingToolProxy::validate_local_request(
            None,
            Some("evil.test:8080")
        )
        .is_err());

        assert!(CredentialInjectingToolProxy::validate_local_request(
            None,
            Some("127.0.0.1:9000")
        )
        .is_ok());

        assert!(CredentialInjectingToolProxy::new().is_ok());
    }
}

