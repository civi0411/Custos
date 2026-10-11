//! Custos Research Domain Pack
//!
//! Authoritative mapping to Custos SADE Master Specification Part 11.
//! Covers academic literature integration, citation verification, dataset auditing,
//! and experiment execution across the Research Workbench Lens.

pub mod fixtures;
pub mod profile;
pub mod sdk;
pub mod skills;
pub mod verifiers;

pub use fixtures::*;
pub use profile::*;
pub use skills::*;
pub use verifiers::*;

use async_trait::async_trait;
use custos_domain::oi::ExecutionTopology;
use custos_domain::session::WorkbenchLens;
use custos_domain::DomainError;
use std::sync::Arc;

use crate::pack::{DomainPack, DomainPackManifest, WorkbenchJob, WorkerRole};
use crate::profile::PackProfile;
use crate::verifier::PackVerifier;

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
        }
    }

    fn profile(&self) -> PackProfile {
        profile::research_profile()
    }

    fn roles(&self) -> Vec<WorkerRole> {
        vec![
            WorkerRole::new(
                "literature_scout",
                "research",
                "Literature Scout",
                "Academic paper retriever, PDF parser, and metadata extractor across ArXiv, CrossRef, and PubMed",
                "You are an academic literature researcher. Your objective is to discover relevant publications, fetch papers, extract abstracts, and map authors and publication dates without hallucinating non-existent papers.",
                10_000,
                &["http_fetch", "doi_verify", "file_read"],
                true,
            ),
            WorkerRole::new(
                "citation_oracle",
                "research",
                "Citation Oracle",
                "Factual and atomic claim extractor that verifies text support against ground-truth paper passages",
                "You are a rigorous scientific fact-checker. Deconstruct research findings into atomic proposition claims, anchor them to exact page/paragraph passages, and detect contradictory claims.",
                8_000,
                &["doi_verify", "claim_extract", "file_read"],
                true,
            ),
            WorkerRole::new(
                "dataset_auditor",
                "research",
                "Dataset Auditor",
                "Data volume inspector that validates splits, schema integrity, and checks for test-set leakage",
                "You are a data integrity and evaluation auditor. Inspect datasets, verify split distributions, check for sample leakage between train and test, and produce canonical DatasetCards.",
                15_000,
                &["file_read", "lexical_search"],
                true,
            ),
            WorkerRole::new(
                "experiment_worker",
                "research",
                "Experiment Worker",
                "Jupyter notebook and Python execution coordinator for training and benchmark reproduction",
                "You are an experimental machine learning engineer. Execute reproducible benchmarks, log metric curves, save artifact checkpoints, and generate figures with traceable seeds and configs.",
                25_000,
                &["file_read"],
                false,
            ),
        ]
    }

    fn jobs(&self) -> Vec<WorkbenchJob> {
        vec![
            WorkbenchJob::new(
                "research__source_qa",
                WorkbenchLens::Research,
                "Source QA & Paper Reading",
                "Read academic paper, extract core contributions, and synthesize a verified ReadingCard",
                &["source_locator", "question"],
                &["PaperReadingCard", "ExtractedPassages"],
                ExecutionTopology::T1SingleWorker,
                "literature_scout",
                &["citation_coverage_oracle"],
                "Accurate passage anchors, valid DOI locator, extraction fidelity with parse gaps acknowledged",
            ),
            WorkbenchJob::new(
                "research__compare",
                WorkbenchLens::Research,
                "Comparative Literature Matrix",
                "Compare multiple papers or baseline methods across identical criteria and benchmark metrics",
                &["sources", "comparison_criteria"],
                &["ComparisonMatrix", "TradeoffReport"],
                ExecutionTopology::T2StagedPipeline,
                "citation_oracle",
                &["citation_coverage_oracle"],
                "Consistent measurement units, identical baseline versions, explicit caveats on contradictions",
            ),
            WorkbenchJob::new(
                "research__deep_research",
                WorkbenchLens::Research,
                "Deep Literature Synthesis",
                "Comprehensive survey across scientific literature with inclusion/exclusion ledger",
                &["topic", "search_scope"],
                &["LiteratureLedger", "ClaimSynthesisBrief"],
                ExecutionTopology::T2StagedPipeline,
                "literature_scout",
                &["citation_coverage_oracle"],
                "Documented query history, claim coverage, explicit separation of supported vs disputed claims",
            ),
            WorkbenchJob::new(
                "research__dataset_audit",
                WorkbenchLens::Research,
                "Dataset & Benchmark Audit",
                "Audit dataset splits, statistics, license terms, and leakage risk",
                &["dataset_path", "schema_spec"],
                &["DatasetCard", "LeakageAuditReport"],
                ExecutionTopology::T1SingleWorker,
                "dataset_auditor",
                &[],
                "Exact sample counts, verified split separation, zero data leakage between train and test",
            ),
            WorkbenchJob::new(
                "research__experiment_design",
                WorkbenchLens::Research,
                "Experiment Design Protocol",
                "Formulate falsifiable hypothesis, baseline controls, metrics, and resource plan",
                &["research_question", "available_compute"],
                &["ExperimentProtocol", "ResourceBudgetPlan"],
                ExecutionTopology::T1SingleWorker,
                "experiment_worker",
                &[],
                "Falsifiable protocol, pre-registered evaluation metrics, absence of test-data tuning",
            ),
            WorkbenchJob::new(
                "research__reproduce",
                WorkbenchLens::Research,
                "Reproducibility Execution",
                "Execute reproducible experiment run capturing code, seeds, configs, and logs",
                &["code_repo", "config_yaml", "seeds"],
                &["ReproducibilityBundle", "ExecutionMetrics", "Figures"],
                ExecutionTopology::T4Worktree,
                "experiment_worker",
                &[],
                "Traceable run outputs, recorded random seeds, comparable metrics against published baseline",
            ),
            WorkbenchJob::new(
                "research__result_analysis",
                WorkbenchLens::Research,
                "Experimental Result Analysis",
                "Statistically compare measured experimental results against literature claims",
                &["run_logs", "baseline_claims"],
                &["ResultAnalysisReport", "SignificanceLedger"],
                ExecutionTopology::T1SingleWorker,
                "citation_oracle",
                &["citation_coverage_oracle"],
                "Evidence-based difference calculation, caveats documented, divergence separated from paper refutation",
            ),
            WorkbenchJob::new(
                "research__research_to_code",
                WorkbenchLens::Research,
                "Research to Code Handoff",
                "Convert validated paper algorithm and claims into implementable engineering specification",
                &["verified_claims", "target_system"],
                &["EngineeringBrief", "AlgorithmSpec", "AcceptanceCriteria"],
                ExecutionTopology::T2StagedPipeline,
                "citation_oracle",
                &[],
                "Typed requirement definitions, provenance to source paper passages, ready for Engineering Pack",
            ),
        ]
    }

    fn verifiers(&self) -> Vec<Arc<dyn PackVerifier>> {
        vec![Arc::new(verifiers::CitationCoverageOracle::new())]
    }

    async fn initialize(&self) -> Result<(), DomainError> {
        tracing::info!("Initialized Research Domain Pack with Literature & Citation Verifiers");
        Ok(())
    }
}
