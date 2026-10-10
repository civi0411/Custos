//! MCP Local Loopback and DNS Rebinding Security Defense
//!
//! Synthesized from AgentGateway (`crates/agentgateway/src/mcp/dns_rebinding.rs`).
//! Protects loopback MCP servers and local IPC endpoints against browser-based
//! DNS rebinding attacks and unauthorized cross-origin requests.

use url::Url;

/// Check if a hostname string represents a safe local loopback host
pub fn is_localhost_host(host: &str) -> bool {
    let h = host.trim();
    let host_only = if h.starts_with('[') {
        if let Some(end_bracket) = h.find(']') {
            &h[1..end_bracket]
        } else {
            h
        }
    } else if let Some((h_prefix, _port)) = h.rsplit_once(':') {
        if !h_prefix.contains(':') {
            h_prefix
        } else {
            h
        }
    } else {
        h
    };

    let cleaned = host_only.trim_matches(['[', ']']);
    cleaned.eq_ignore_ascii_case("localhost") || cleaned == "127.0.0.1" || cleaned == "::1"
}

/// Check if an HTTP Origin header string represents a safe localhost origin
pub fn is_localhost_origin(origin: &str) -> bool {
    let Ok(parsed) = Url::parse(origin.trim()) else {
        return false;
    };

    if !matches!(parsed.scheme(), "http" | "https") {
        return false;
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return false;
    }
    if parsed.path() != "/" && !parsed.path().is_empty() {
        return false;
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        return false;
    }

    parsed
        .host_str()
        .map(is_localhost_host)
        .unwrap_or(false)
}

/// Validate whether an incoming MCP HTTP request satisfies DNS rebinding protection invariants
pub fn is_localhost_request(host: Option<&str>, origin: Option<&str>) -> bool {
    // 1. Host header MUST be localhost
    let Some(host) = host else {
        return false;
    };
    if !is_localhost_host(host) {
        return false;
    }

    // 2. If Origin header is present (browser context), it MUST be a localhost origin
    if let Some(origin) = origin {
        if !is_localhost_origin(origin) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accepts_localhost_hosts() {
        for host in [
            "localhost",
            "localhost:8080",
            "127.0.0.1",
            "127.0.0.1:3000",
            "[::1]",
            "[::1]:8080",
            "::1",
        ] {
            assert!(is_localhost_host(host), "failed for host: {}", host);
        }
    }

    #[test]
    fn test_rejects_external_hosts() {
        for host in ["evil.com", "evil.com:8080", "192.168.1.1", "10.0.0.1"] {
            assert!(!is_localhost_host(host), "should reject: {}", host);
        }
    }

    #[test]
    fn test_accepts_localhost_origins() {
        for origin in [
            "http://localhost",
            "http://localhost:8080",
            "http://127.0.0.1:3000",
            "http://[::1]:8080",
            "https://localhost",
        ] {
            assert!(is_localhost_origin(origin), "failed for origin: {}", origin);
        }
    }

    #[test]
    fn test_rejects_malicious_origins() {
        for origin in [
            "null",
            "http://evil.com",
            "http://evil.com:8080",
            "ftp://localhost",
            "http://localhost/path",
            "http://[::1].evil.example",
        ] {
            assert!(!is_localhost_origin(origin), "should reject origin: {}", origin);
        }
    }

    #[test]
    fn test_request_rebinding_gate() {
        assert!(is_localhost_request(Some("localhost:8080"), None));
        assert!(is_localhost_request(
            Some("127.0.0.1:8080"),
            Some("http://localhost:8080")
        ));
        assert!(!is_localhost_request(Some("evil.com"), None));
        assert!(!is_localhost_request(
            Some("127.0.0.1:8080"),
            Some("http://evil.com")
        ));
    }
}
