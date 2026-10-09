//! Scoped Browser Domain Models
//!
//! Provides browser session, tab, and snapshot contracts scoped to execution
//! workspaces. Network isolation and DNS-aware SSRF enforcement remain adapter duties.

use serde::{Deserialize, Serialize};
use crate::error::DomainError;
use crate::ids::new_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserTabStatus {
    Idle,
    Loading,
    Ready,
    Error,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowserConsoleEntry {
    pub level: String,
    pub message: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowserNetworkRequest {
    pub url: String,
    pub method: String,
    pub status: Option<u16>,
    pub content_type: Option<String>,
    pub duration_ms: u64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowserPageSnapshot {
    pub tab_id: String,
    pub url: String,
    pub title: String,
    pub dom_tree_summary: String,
    pub text_content: String,
    pub links: Vec<String>,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub screenshot_uri: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowserTab {
    pub id: String,
    pub session_id: String,
    pub url: String,
    pub title: String,
    pub status: BrowserTabStatus,
    pub active: bool,
    pub last_snapshot: Option<BrowserPageSnapshot>,
    pub console_logs: Vec<BrowserConsoleEntry>,
    pub network_requests: Vec<BrowserNetworkRequest>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl BrowserTab {
    pub fn new(session_id: impl Into<String>, initial_url: impl Into<String>) -> Result<Self, DomainError> {
        let url = initial_url.into();
        validate_browser_url(&url)?;
        let now = chrono::Utc::now().timestamp_millis();
        Ok(Self {
            id: new_id("tab"),
            session_id: session_id.into(),
            url: url.clone(),
            title: if url.is_empty() { "New Tab".to_string() } else { url },
            status: BrowserTabStatus::Idle,
            active: true,
            last_snapshot: None,
            console_logs: Vec::new(),
            network_requests: Vec::new(),
            created_at: now,
            updated_at: now,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrowserSession {
    pub id: String,
    pub name: String,
    pub workspace_id: Option<String>,
    pub tabs: Vec<BrowserTab>,
    pub active_tab_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl BrowserSession {
    pub fn new(name: impl Into<String>, workspace_id: Option<String>) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            id: new_id("brw"),
            name: name.into(),
            workspace_id,
            tabs: Vec::new(),
            active_tab_id: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Request parameters to navigate a browser tab
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigateTabParams {
    pub tab_id: String,
    pub url: String,
}

/// Request parameters to capture a page snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureSnapshotParams {
    pub tab_id: String,
    #[serde(default)]
    pub include_screenshot: bool,
}

/// Request parameters to create a new tab
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTabParams {
    pub session_id: Option<String>,
    pub url: Option<String>,
    pub workspace_id: Option<String>,
}

/// Rejects unsupported schemes and obvious local/private-network destinations.
/// A browser adapter must additionally resolve DNS and enforce egress policy at dispatch time.
pub fn validate_browser_url(raw_url: &str) -> Result<(), DomainError> {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    let (scheme, rest) = if let Some(stripped) = trimmed.strip_prefix("https://") {
        ("https", stripped)
    } else if let Some(stripped) = trimmed.strip_prefix("http://") {
        ("http", stripped)
    } else {
        return Err(DomainError::Validation(format!(
            "Protocol is rejected by Custos URL sandbox; only http:// and https:// are permitted (got '{}').",
            trimmed
        )));
    };

    // Extract host (up to next '/', ':', '?', or '#')
    let host = rest
        .split(['/', ':', '?', '#'])
        .next()
        .unwrap_or("")
        .trim();

    if host.is_empty() {
        return Err(DomainError::Validation("URL must contain a valid hostname.".to_string()));
    }

    let lower = host.to_lowercase();
    let private_ipv4 = lower == "0.0.0.0"
        || lower.starts_with("127.")
        || lower.starts_with("10.")
        || lower.starts_with("192.168.")
        || lower.starts_with("169.254.")
        || lower
            .strip_prefix("172.")
            .and_then(|tail| tail.split('.').next())
            .and_then(|octet| octet.parse::<u8>().ok())
            .is_some_and(|octet| (16..=31).contains(&octet));
    let private_name = lower == "localhost"
        || lower.ends_with(".localhost")
        || lower.ends_with(".local")
        || lower == "metadata.google.internal"
        || lower == "metadata";
    let private_ipv6 = lower == "[::1]"
        || lower.starts_with("[fc")
        || lower.starts_with("[fd")
        || lower.starts_with("[fe80:");

    if private_ipv4 || private_name || private_ipv6 {
        return Err(DomainError::Validation(format!(
            "Access to local or private destination '{}' is rejected by the browser contract.",
            lower
        )));
    }

    let _ = scheme;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_url_validation() {
        assert!(validate_browser_url("https://example.com").is_ok());
        assert!(validate_browser_url("http://arxiv.org/abs/2301.00001").is_ok());
        assert!(validate_browser_url("file:///etc/passwd").is_err());
        assert!(validate_browser_url("javascript:alert(1)").is_err());
        assert!(validate_browser_url("http://169.254.169.254/latest/meta-data").is_err());
        assert!(validate_browser_url("http://localhost:8080").is_err());
        assert!(validate_browser_url("http://127.0.0.1").is_err());
        assert!(validate_browser_url("http://10.0.0.1").is_err());
        assert!(validate_browser_url("http://172.16.1.2").is_err());
        assert!(validate_browser_url("http://192.168.1.2").is_err());
    }

    #[test]
    fn test_browser_tab_lifecycle() {
        let session = BrowserSession::new("research-session", Some("ws_default".to_string()));
        assert!(session.id.starts_with("brw_"));

        let tab = BrowserTab::new(&session.id, "https://docs.rs").unwrap();
        assert!(tab.id.starts_with("tab_"));
        assert_eq!(tab.status, BrowserTabStatus::Idle);
        assert_eq!(tab.url, "https://docs.rs");
    }
}
