//! Custos Research Domain Pack
//!
//! Authoritative mapping to Custos SADE Master Specification Part 11.
//! Covers academic literature integration, citation verification, dataset auditing,
//! and experiment execution across the Research Workbench Lens.

use std::sync::Arc;
use async_trait::async_trait;
use custos_domain::session::WorkbenchLens;
use custos_domain::DomainError;
use custos_core::contracts::pack::{
    DomainPack, DomainPackManifest, PackProfile, PackVerifier, WorkbenchJob, WorkerRole,
};

use crate::jobs;
use crate::profile;
use crate::roles;
use crate::verifiers;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResearchPackDescriptor {
    pub name: String,
    pub version: String,
    pub features: Vec<String>,
}

pub fn get_descriptor() -> ResearchPackDescriptor {
    ResearchPackDescriptor {
        name: "custos-pack-research".to_string(),
        version: "0.1.0".to_string(),
        features: vec![
            "Academic Literature Search & Fetch".to_string(),
            "Claim & Fact Extraction".to_string(),
            "Cross-Reference Citation Verifier".to_string(),
            "ReadingCard Synthesis with Evidence Anchors".to_string(),
            "Dataset & Experiment Provenance".to_string(),
        ],
    }
}

pub struct ResearchPack;

impl ResearchPack {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ResearchPack {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainPack for ResearchPack {
    fn manifest(&self) -> DomainPackManifest {
        let roles = self.roles().into_iter().map(|r| r.role_id).collect();
        let jobs = self.jobs().into_iter().map(|j| j.job_id).collect();
        DomainPackManifest {
            id: "research".to_string(),
            name: "Research Pack".to_string(),
            version: "0.1.0".to_string(),
            lens: WorkbenchLens::Research,
            description: "Academic literature search, fact extraction, citation verification, and reproducible experimentation".to_string(),
            supported_jobs: jobs,
            supported_roles: roles,
            required_capabilities: vec!["notebook".to_string(), "documents".to_string()],
        }
    }

    fn profile(&self) -> PackProfile {
        profile::research_profile()
    }

    fn roles(&self) -> Vec<WorkerRole> {
        roles::research_roles()
    }

    fn jobs(&self) -> Vec<WorkbenchJob> {
        jobs::research_jobs()
    }

    fn verifiers(&self) -> Vec<Arc<dyn PackVerifier>> {
        vec![Arc::new(verifiers::CitationCoverageOracle::new())]
    }

    async fn initialize(&self) -> Result<(), DomainError> {
        tracing::info!("Initialized Research Domain Pack with Literature & Citation Verifiers");
        Ok(())
    }

    fn lane_policy(&self) -> custos_domain::LanePolicy {
        custos_domain::LanePolicy::research_fanout(8)
    }

    fn supervision_profile(&self) -> Option<custos_domain::supervision::SupervisionProfile> {
        Some(crate::supervision::research_supervision_profile())
    }

    fn tool_declarations(&self) -> Vec<serde_json::Value> {
        vec![serde_json::json!({
            "type": "function",
            "function": {
                "name": "search_literature",
                "description": "Search scientific papers, ArXiv, and research document corpuses",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Research search query" },
                        "limit": { "type": "integer", "description": "Max number of citations to return" }
                    },
                    "required": ["query"]
                }
            }
        })]
    }

    async fn execute_tool(
        &self,
        tool_name: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, DomainError> {
        match tool_name {
            "search_literature" => {
                let query = params
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Ok(serde_json::json!({
                    "status": "success",
                    "query": query,
                    "results_count": 0,
                    "citations": [],
                }))
            }
            _ => Err(DomainError::Validation(format!(
                "Unsupported research tool: '{tool_name}'"
            ))),
        }
    }
}
