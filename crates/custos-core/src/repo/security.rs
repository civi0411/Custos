//! Repository Security & Scope Policy Enforcement
//!
//! Enforces path boundaries, sensitive secret redaction, and token budget
//! egress controls on SourceSpans before they enter LLM context compiler.

use custos_domain::repo::SourceSpan;
use custos_domain::DomainError;
use std::collections::HashSet;

/// Security policy defining which files and spans an execution agent may read.
#[derive(Debug, Clone)]
pub struct RepoSecurityPolicy {
    /// Permitted relative directory prefixes (e.g. `["src/", "tests/"]`). Empty means all.
    pub allowed_paths: Vec<String>,
    /// Blocked relative paths or exact files (e.g. `[".env", "secrets/", "credentials.json"]`).
    pub blocked_patterns: HashSet<String>,
    /// Maximum allowable bytes in a single source span egress.
    pub max_span_bytes: usize,
    /// Maximum cumulative lines allowed in a single context extraction.
    pub max_cumulative_lines: usize,
}

impl Default for RepoSecurityPolicy {
    fn default() -> Self {
        let mut blocked = HashSet::new();
        blocked.insert(".env".into());
        blocked.insert(".git".into());
        blocked.insert("id_rsa".into());
        blocked.insert(".aws".into());
        blocked.insert("credentials.json".into());
        blocked.insert("secret".into());

        Self {
            allowed_paths: Vec::new(),
            blocked_patterns: blocked,
            max_span_bytes: 256 * 1024, // 256 KB per span max
            max_cumulative_lines: 2000,
        }
    }
}

impl RepoSecurityPolicy {
    /// Validates whether a given SourceSpan complies with path scopes and security boundaries.
    pub fn validate_span(&self, span: &SourceSpan) -> Result<(), DomainError> {
        let path = &span.path;

        // 1. Check for directory traversal / escape attempts
        if path.contains("..") || path.starts_with('/') || path.starts_with('\\') {
            return Err(DomainError::Validation(format!(
                "Security violation: path traversal sequence detected in '{}'",
                path
            )));
        }

        // 2. Check blocked sensitive patterns
        for pattern in &self.blocked_patterns {
            if path.contains(pattern) {
                return Err(DomainError::Validation(format!(
                    "Security violation: access to protected secret pattern '{}' is denied in '{}'",
                    pattern, path
                )));
            }
        }

        // 3. Check allowed paths scope if configured
        if !self.allowed_paths.is_empty() {
            let is_allowed = self
                .allowed_paths
                .iter()
                .any(|allowed| path.starts_with(allowed));
            if !is_allowed {
                return Err(DomainError::Validation(format!(
                    "Security violation: path '{}' is outside authorized task scope {:?}",
                    path, self.allowed_paths
                )));
            }
        }

        // 4. Validate span egress size
        if span.end_byte < span.start_byte {
            return Err(DomainError::Validation(format!(
                "Invalid span byte range: start {} > end {}",
                span.start_byte, span.end_byte
            )));
        }
        let byte_len = span.end_byte - span.start_byte;
        if byte_len > self.max_span_bytes {
            return Err(DomainError::Validation(format!(
                "Span egress limit exceeded: {} bytes exceeds max {} bytes",
                byte_len, self.max_span_bytes
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_security_policy_blocks_traversal() {
        let policy = RepoSecurityPolicy::default();
        let span = SourceSpan::new("../../etc/passwd", 0, 10, 1, 2, "hash");
        assert!(policy.validate_span(&span).is_err());
    }

    #[test]
    fn test_repo_security_policy_blocks_secrets() {
        let policy = RepoSecurityPolicy::default();
        let span = SourceSpan::new("config/.env.prod", 0, 10, 1, 2, "hash");
        assert!(policy.validate_span(&span).is_err());
    }

    #[test]
    fn test_repo_security_policy_allows_valid_span() {
        let policy = RepoSecurityPolicy::default();
        let span = SourceSpan::new("src/main.rs", 0, 100, 1, 10, "hash");
        assert!(policy.validate_span(&span).is_ok());
    }
}
