//! Artifact References & Storage Metadata
//!
//! Immutable references to content-addressed or file-system artifacts
//! generated throughout the task execution lifecycle.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::new_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    FileDiff,
    DiagnosticLog,
    VerificationReport,
    ExecutionTrace,
    Snapshot,
    ContinuationState,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRef {
    pub id: String,
    pub kind: ArtifactKind,
    pub hash: String,
    pub path: String,
    pub size_bytes: u64,
    pub mime: Option<String>,
}

impl ArtifactRef {
    pub fn new(
        id: String,
        kind: ArtifactKind,
        hash: String,
        path: String,
        size_bytes: u64,
    ) -> Self {
        Self {
            id,
            kind,
            hash,
            path,
            size_bytes,
            mime: None,
        }
    }

    /// Converts this ArtifactRef to a canonical Universal Resource Registry URI
    pub fn to_urr(&self, component: impl Into<String>) -> UniversalResourceRef {
        UniversalResourceRef {
            component: component.into(),
            kind: format!("{:?}", self.kind).to_lowercase(),
            id: self.id.clone(),
            sha256: self.hash.clone(),
        }
    }
}

/// Canonical Universal Resource Registry (URR) Reference
/// Format: `urr://[component]/[kind]/[id]#[sha256]`
///
/// Serves as the immutable, content-addressed substrate for zero-copy
/// artifact sharing across heterogeneous agents in the SADE Agent Lake.
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

impl std::fmt::Display for UniversalResourceRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_uri())
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffConsent {
    Pending,
    Granted,
    Denied,
}

/// Selected artifact transfer between pack obligations.
/// Authority never follows this value: grants, permits, and secrets are intentionally absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactHandoff {
    pub id: String,
    pub parent_task_id: String,
    pub source_node_id: Option<String>,
    pub producer_pack: String,
    pub consumer_pack: String,
    pub artifact_ref: String,
    pub artifact_type: String,
    pub artifact_version: String,
    pub artifact_digest: String,
    pub source_dependencies: Vec<String>,
    pub selected_content_refs: Vec<String>,
    pub redacted_refs: Vec<String>,
    pub privacy_class: String,
    pub intent: String,
    pub consent: HandoffConsent,
    pub caveats: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl ArtifactHandoff {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        parent_task_id: impl Into<String>,
        producer_pack: impl Into<String>,
        consumer_pack: impl Into<String>,
        artifact_ref: impl Into<String>,
        artifact_type: impl Into<String>,
        artifact_version: impl Into<String>,
        artifact_digest: impl Into<String>,
        privacy_class: impl Into<String>,
        intent: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let parent_task_id = parent_task_id.into();
        let producer_pack = producer_pack.into();
        let consumer_pack = consumer_pack.into();
        let artifact_ref = artifact_ref.into();
        let artifact_type = artifact_type.into();
        let artifact_version = artifact_version.into();
        let artifact_digest = artifact_digest.into();
        let privacy_class = privacy_class.into();
        let intent = intent.into();
        if [
            parent_task_id.as_str(),
            producer_pack.as_str(),
            consumer_pack.as_str(),
            artifact_ref.as_str(),
            artifact_type.as_str(),
            artifact_version.as_str(),
            artifact_digest.as_str(),
            privacy_class.as_str(),
            intent.as_str(),
        ]
        .iter()
        .any(|value| value.trim().is_empty())
        {
            return Err(DomainError::Validation(
                "Artifact handoff identity, provenance, digest, privacy, and intent are required"
                    .into(),
            ));
        }
        if producer_pack == consumer_pack {
            return Err(DomainError::Validation(
                "Cross-pack artifact handoff requires distinct producer and consumer packs".into(),
            ));
        }

        Ok(Self {
            id: new_id("handoff"),
            parent_task_id,
            source_node_id: None,
            producer_pack,
            consumer_pack,
            artifact_ref,
            artifact_type,
            artifact_version,
            artifact_digest,
            source_dependencies: Vec::new(),
            selected_content_refs: Vec::new(),
            redacted_refs: Vec::new(),
            privacy_class,
            intent,
            consent: HandoffConsent::Pending,
            caveats: Vec::new(),
            created_at: Utc::now(),
        })
    }

    pub fn grant_consent(&mut self) -> Result<(), DomainError> {
        if self.consent != HandoffConsent::Pending {
            return Err(DomainError::Conflict(format!(
                "Artifact handoff {} consent is already resolved",
                self.id
            )));
        }
        self.consent = HandoffConsent::Granted;
        Ok(())
    }

    pub fn deny_consent(&mut self) -> Result<(), DomainError> {
        if self.consent != HandoffConsent::Pending {
            return Err(DomainError::Conflict(format!(
                "Artifact handoff {} consent is already resolved",
                self.id
            )));
        }
        self.consent = HandoffConsent::Denied;
        Ok(())
    }
}

/// Content-addressed lineage node of generated artifacts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactLineageNode {
    pub artifact_path: String,
    pub version: u32,
    pub content_hash: String,
    pub produced_by_run_id: Option<String>,
    pub parent_version_hash: Option<String>,
    pub timestamp: i64,
}

/// Summary descriptor of a tracked artifact with its latest version and lineage
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactSummary {
    pub artifact_path: String,
    pub latest_version: u32,
    pub latest_content_hash: String,
    pub versions_count: u32,
    pub produced_by_run_id: Option<String>,
    pub updated_at: i64,
}

/// A node in the artifact provenance DAG
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineageGraphNode {
    pub id: String,
    pub label: String,
    pub node_type: String, // "artifact", "run", "recipe", "note"
    pub version: Option<u32>,
    pub content_hash: Option<String>,
    pub timestamp: i64,
    pub metadata: serde_json::Value,
}

/// A directed edge in the artifact provenance DAG
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineageGraphEdge {
    pub from: String,
    pub to: String,
    pub edge_type: String, // "derived_from_version", "produced_by_run", "used_as_input"
}

/// Complete artifact lineage and provenance DAG
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ArtifactLineageGraph {
    pub nodes: Vec<LineageGraphNode>,
    pub edges: Vec<LineageGraphEdge>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handoff_requires_cross_pack_provenance_and_single_consent_decision() {
        let invalid = ArtifactHandoff::new(
            "task_1",
            "research",
            "research",
            "artifact_1",
            "engineering_brief",
            "1",
            "digest",
            "user_private",
            "Implement the selected finding",
        );
        assert!(matches!(invalid, Err(DomainError::Validation(_))));

        let mut handoff = ArtifactHandoff::new(
            "task_1",
            "research",
            "engineering",
            "artifact_1",
            "engineering_brief",
            "1",
            "digest",
            "user_private",
            "Implement the selected finding",
        );
        assert!(handoff.is_ok());
        if let Ok(ref mut value) = handoff {
            assert!(value.grant_consent().is_ok());
            assert!(value.deny_consent().is_err());
        }
    }

    #[test]
    fn test_universal_resource_ref_parse_and_format() {
        let urr = UniversalResourceRef::new(
            "science",
            "claim",
            "claim_8921",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        );
        let uri = urr.to_uri();
        assert_eq!(
            uri,
            "urr://science/claim/claim_8921#e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(format!("{}", urr), uri);

        let parsed = UniversalResourceRef::parse(&uri).expect("parsing valid URI should succeed");
        assert_eq!(parsed, urr);

        // Invalid URI checks
        assert!(UniversalResourceRef::parse("https://science/claim/123#hash").is_err());
        assert!(UniversalResourceRef::parse("urr://science/claim").is_err());
        assert!(UniversalResourceRef::parse("urr://science/claim/123").is_err());
        assert!(UniversalResourceRef::parse("urr://science/claim/123#").is_err());

        // Test ArtifactRef to_urr conversion
        let artifact = ArtifactRef::new(
            "diff_42".to_string(),
            ArtifactKind::FileDiff,
            "abc123hash".to_string(),
            "platform/core/src/lib.rs".to_string(),
            1024,
        );
        let art_urr = artifact.to_urr("engineering");
        assert_eq!(art_urr.component, "engineering");
        assert_eq!(art_urr.kind, "filediff");
        assert_eq!(art_urr.id, "diff_42");
        assert_eq!(art_urr.sha256, "abc123hash");
        assert_eq!(art_urr.to_uri(), "urr://engineering/filediff/diff_42#abc123hash");
    }
}

