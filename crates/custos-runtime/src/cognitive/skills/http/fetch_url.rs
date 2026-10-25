//! HTTP Fetch Skill
//!
//! Fetches content from external web endpoints.
//! Enforces SSRF defenses (blocking RFC 1918 / localhost / cloud metadata)
//! and marks all outputs with `TaintLevel::Untrusted` (THR-02 / THR-07).

use async_trait::async_trait;
use serde_json::json;
use std::net::IpAddr;
use std::time::Duration;

use crate::cognitive::skills::{
    Skill, SkillCapabilityProfile, SkillContext, SkillError, SkillOutput, TaintLevel,
};

pub struct FetchUrlSkill {
    client: reqwest::Client,
}

const MAX_BODY_BYTES: usize = 128 * 1024;

fn is_safe_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            if ipv4.is_loopback()
                || ipv4.is_private()
                || ipv4.is_link_local()
                || ipv4.is_broadcast()
                || ipv4.is_multicast()
                || ipv4.is_unspecified()
            {
                return false;
            }
            let octets = ipv4.octets();
            // Carrier-grade NAT (100.64.0.0/10)
            if octets[0] == 100 && (octets[1] & 0xC0) == 64 {
                return false;
            }
            // Documentation and benchmarking test networks (RFC 5737 / RFC 2544)
            if octets[0] == 192 && octets[1] == 0 && octets[2] == 2 {
                return false;
            }
            if octets[0] == 198 && (octets[1] == 18 || octets[1] == 19) {
                return false;
            }
            if octets[0] == 198 && octets[1] == 51 && octets[2] == 100 {
                return false;
            }
            if octets[0] == 203 && octets[1] == 0 && octets[2] == 113 {
                return false;
            }
            // Cloud metadata (169.254.169.254)
            if octets == [169, 254, 169, 254] {
                return false;
            }
            true
        }
        IpAddr::V6(ipv6) => {
            if ipv6.is_loopback() || ipv6.is_multicast() || ipv6.is_unspecified() {
                return false;
            }
            // Check IPv4-mapped IPv6 (e.g. ::ffff:127.0.0.1)
            let segments = ipv6.segments();
            if segments[0] == 0 && segments[1] == 0 && segments[2] == 0 && segments[3] == 0 && segments[4] == 0 && segments[5] == 0xffff {
                let v4 = std::net::Ipv4Addr::new(
                    (segments[6] >> 8) as u8,
                    segments[6] as u8,
                    (segments[7] >> 8) as u8,
                    segments[7] as u8,
                );
                return is_safe_ip(IpAddr::V4(v4));
            }
            // Unique local addresses (fc00::/7)
            if (segments[0] & 0xfe00) == 0xfc00 {
                return false;
            }
            // Link-local unicast (fe80::/10)
            if (segments[0] & 0xffc0) == 0xfe80 {
                return false;
            }
            true
        }
    }
}

impl Default for FetchUrlSkill {
    fn default() -> Self {
        let redirect_policy = reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 {
                return attempt.error("Too many redirects (max 5)");
            }
            if let Err(e) = FetchUrlSkill::validate_ssrf_url(attempt.url()) {
                return attempt.error(format!("SSRF violation on redirect target: {}", e));
            }
            attempt.follow()
        });

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .redirect(redirect_policy)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { client }
    }
}

impl FetchUrlSkill {
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate URL against SSRF attacks (private IPs, localhost, cloud metadata)
    pub fn validate_ssrf_url(url: &reqwest::Url) -> Result<(), SkillError> {
        let scheme = url.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(SkillError::SandboxViolation(format!(
                "forbidden URL scheme: '{}'. Only http/https allowed.",
                scheme
            )));
        }

        let host_str = url
            .host_str()
            .ok_or_else(|| SkillError::InvalidArg("URL missing host".into()))?;
        let lower_host = host_str.to_lowercase();

        if lower_host == "localhost"
            || lower_host.ends_with(".localhost")
            || lower_host.ends_with(".local")
            || lower_host.ends_with(".internal")
        {
            return Err(SkillError::SandboxViolation(format!(
                "SSRF protection: access to host '{}' is blocked",
                host_str
            )));
        }

        // Parse host if it's an IP literal
        let unbracketed = lower_host.trim_start_matches('[').trim_end_matches(']');
        if let Ok(ip) = unbracketed.parse::<IpAddr>() {
            if !is_safe_ip(ip) {
                return Err(SkillError::SandboxViolation(format!(
                    "SSRF protection: access to private/reserved IP '{}' is blocked",
                    ip
                )));
            }
        }

        Ok(())
    }

    pub fn validate_ssrf_safety(url_str: &str) -> Result<reqwest::Url, SkillError> {
        let url = reqwest::Url::parse(url_str)
            .map_err(|e| SkillError::InvalidArg(format!("invalid URL: {}", e)))?;
        Self::validate_ssrf_url(&url)?;
        Ok(url)
    }
}

#[async_trait]
impl Skill for FetchUrlSkill {
    fn skill_id(&self) -> &'static str {
        "http_fetch"
    }

    fn description(&self) -> &'static str {
        "Fetch external HTTP content with SSRF protection and untrusted taint marking"
    }

    fn capability_profile(&self) -> SkillCapabilityProfile {
        SkillCapabilityProfile::external_read(15_000)
    }

    async fn run(&self, ctx: &SkillContext) -> Result<SkillOutput, SkillError> {
        let url_str = ctx
            .args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| SkillError::MissingArg("url".to_string()))?;

        let safe_url = Self::validate_ssrf_safety(url_str)?;

        let mut response = self
            .client
            .get(safe_url.clone())
            .send()
            .await
            .map_err(|e| SkillError::FetchError(format!("request failed: {}", e)))?;

        let status = response.status().as_u16();

        // Bounded stream consumption up to 128 KB to prevent unbounded RAM consumption
        let mut bytes = Vec::new();
        let mut truncated = false;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| SkillError::FetchError(format!("failed to read body stream: {}", e)))?
        {
            let needed = MAX_BODY_BYTES.saturating_sub(bytes.len());
            if chunk.len() > needed {
                bytes.extend_from_slice(&chunk[..needed]);
                truncated = true;
                break;
            } else {
                bytes.extend_from_slice(&chunk);
                if bytes.len() >= MAX_BODY_BYTES {
                    truncated = true;
                    break;
                }
            }
        }

        // Safe lossy UTF-8 conversion prevents panics on multi-byte boundaries
        let mut body_preview = String::from_utf8_lossy(&bytes).into_owned();
        if truncated {
            body_preview.push_str("\n...[truncated: exceeds 128KB limit]");
        }

        Ok(SkillOutput {
            skill_id: self.skill_id().to_string(),
            result: json!({
                "url": safe_url.to_string(),
                "status": status,
                "body": body_preview,
            }),
            artifacts: vec![],
            evidence_hint: Some(format!("http_fetch {} -> status {}", safe_url, status)),
            taint: TaintLevel::Untrusted, // CRITICAL: external web content is always Untrusted
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssrf_blocks_localhost_and_metadata() {
        assert!(FetchUrlSkill::validate_ssrf_safety("http://localhost:8080").is_err());
        assert!(FetchUrlSkill::validate_ssrf_safety("http://127.0.0.1/admin").is_err());
        assert!(
            FetchUrlSkill::validate_ssrf_safety("http://169.254.169.254/latest/meta-data").is_err()
        );
        assert!(FetchUrlSkill::validate_ssrf_safety("ftp://example.com").is_err());
        assert!(FetchUrlSkill::validate_ssrf_safety("https://api.crossref.org/works").is_ok());
    }

    #[test]
    fn ssrf_blocks_private_subnets_and_ipv6() {
        // RFC 1918 private IPv4
        assert!(FetchUrlSkill::validate_ssrf_safety("http://10.0.0.1/").is_err());
        assert!(FetchUrlSkill::validate_ssrf_safety("http://172.16.0.5/status").is_err());
        assert!(FetchUrlSkill::validate_ssrf_safety("http://192.168.1.1/router").is_err());
        // Carrier-grade NAT (100.64.0.0/10)
        assert!(FetchUrlSkill::validate_ssrf_safety("http://100.64.0.1/").is_err());
        // IPv6 loopback and ULA
        assert!(FetchUrlSkill::validate_ssrf_safety("http://[::1]/").is_err());
        assert!(FetchUrlSkill::validate_ssrf_safety("http://[fc00::1]/").is_err());
        assert!(FetchUrlSkill::validate_ssrf_safety("http://[fe80::1]/").is_err());
        // IPv4-mapped IPv6 loopback
        assert!(FetchUrlSkill::validate_ssrf_safety("http://[::ffff:127.0.0.1]/").is_err());
        // Internal domain names
        assert!(FetchUrlSkill::validate_ssrf_safety("http://app.internal/").is_err());
        assert!(FetchUrlSkill::validate_ssrf_safety("http://device.local/").is_err());
    }
}
