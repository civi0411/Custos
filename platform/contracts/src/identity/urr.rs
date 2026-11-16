//! Canonical Universal Resource Reference (URR)
//!
//! Strongly-typed message passing address for Alan Kay OOP architecture:
//! `urr://[component]/[kind]/[id]#[sha256]`

use crate::identity::error::DomainError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UniversalResourceRef {
    pub component: String,
    pub kind: String,
    pub id: String,
    pub sha256: String,
}

impl UniversalResourceRef {
    pub fn new(
        component: impl Into<String>,
        kind: impl Into<String>,
        id: impl Into<String>,
        sha256: impl Into<String>,
    ) -> Self {
        Self {
            component: component.into(),
            kind: kind.into(),
            id: id.into(),
            sha256: sha256.into(),
        }
    }

    /// Returns the canonical URI string: `urr://[component]/[kind]/[id]#[sha256]`
    pub fn to_uri(&self) -> String {
        format!(
            "urr://{}/{}/{}#{}",
            self.component, self.kind, self.id, self.sha256
        )
    }

    /// Parses a canonical URR URI string into a strongly-typed UniversalResourceRef
    pub fn parse(s: &str) -> Result<Self, DomainError> {
        if !s.starts_with("urr://") {
            return Err(DomainError::Validation(format!(
                "Invalid URR URI scheme (expected urr://): {}",
                s
            )));
        }
        let rest = &s[6..];
        let (path_part, sha256) = match rest.split_once('#') {
            Some((p, h)) => (p, h),
            None => {
                return Err(DomainError::Validation(format!(
                    "URR URI missing sha256 fragment (#): {}",
                    s
                )))
            }
        };
        let parts: Vec<&str> = path_part.split('/').collect();
        if parts.len() < 3 {
            return Err(DomainError::Validation(format!(
                "URR URI path must have [component]/[kind]/[id]: {}",
                s
            )));
        }
        if sha256.trim().is_empty() {
            return Err(DomainError::Validation(format!(
                "URR URI contains empty sha256 hash: {}",
                s
            )));
        }
        Ok(Self {
            component: parts[0].to_string(),
            kind: parts[1].to_string(),
            id: parts[2..].join("/"),
            sha256: sha256.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_universal_resource_ref_parse_and_format() {
        let urr = UniversalResourceRef::new("research", "artifact", "art-1234", "abcdef0123456789");
        assert_eq!(
            urr.to_uri(),
            "urr://research/artifact/art-1234#abcdef0123456789"
        );

        let parsed =
            UniversalResourceRef::parse("urr://research/artifact/art-1234#abcdef0123456789")
                .unwrap();
        assert_eq!(parsed, urr);
        assert_eq!(parsed.component, "research");
        assert_eq!(parsed.kind, "artifact");
        assert_eq!(parsed.id, "art-1234");
        assert_eq!(parsed.sha256, "abcdef0123456789");

        // Edge case: ID containing slashes
        let complex =
            UniversalResourceRef::parse("urr://coding/file/src/lib.rs#11223344").unwrap();
        assert_eq!(complex.component, "coding");
        assert_eq!(complex.kind, "file");
        assert_eq!(complex.id, "src/lib.rs");
        assert_eq!(complex.sha256, "11223344");

        // Failure cases
        assert!(UniversalResourceRef::parse("https://research/artifact#123").is_err());
        assert!(UniversalResourceRef::parse("urr://research/artifact").is_err());
        assert!(UniversalResourceRef::parse("urr://research#123").is_err());
        assert!(UniversalResourceRef::parse("urr://research/artifact/123#").is_err());
    }
}

