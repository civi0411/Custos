//! Custos Sovereign Research Workbench (`custos-workbench-research`)
//!
//! Autonomous academic literature analysis, claim verification,
//! note-taking, notebooks, and reproducible experiment execution.

pub mod contracts;
pub mod domain;
pub mod fixtures;
pub mod jobs;
pub mod pack;
pub mod profile;
pub mod repository;
pub mod roles;
pub mod services;
pub mod skills;
pub mod supervision;
pub mod verifiers;

// Contracts re-exports (canonical ownership lives in platform/core & platform/domain)
pub use contracts::research_ingress::{
    claim_draft, experiment_draft, passage_draft, source_draft, source_draft_with_bytes,
};
pub use custos_core::contracts::notebook::{
    NotebookDriverPort, NotebookStoragePort, NotebookWorkerResponse, NotebookWorkerSession,
};

pub use repository::ResearchRepository;
pub use services::{PreparedCodingHandoff, SynthesisService};

// Domain re-exports
pub use custos_domain::{
    CellExecutionStatus, ExecuteCellParams, ExecuteCellResult, FrozenTrainingRecipe,
    KernelStatus, ModelEvaluationReport, ModelEvaluationSpec, NotebookCell, NotebookCellType,
    NotebookDocument, NotebookKernelBinding, NotebookKernelState, TrainingExecutionReceipt,
    TrainingStepTelemetry,
};
pub use domain::claim::*;
pub use domain::note::{NoteRecord, NoteVersionRecord};
pub use domain::pipeline_invariant::EvidenceGroundingInvariant;
pub use domain::synthesis::*;

pub use fixtures::*;
pub use jobs::*;
pub use pack::{get_descriptor, ResearchPack, ResearchPackDescriptor};
pub use profile::research_profile;
pub use roles::research_roles;
pub use skills::*;
pub use verifiers::*;

use custos_core::contracts::pack::{DomainPack, WorkbenchModule};
use custos_core::contracts::skill::SkillSink;
use std::sync::Arc;

/// Sovereign Workbench Module descriptor for Custos Research
#[derive(Default, Clone)]
pub struct ResearchModule;

impl ResearchModule {
    pub fn new() -> Self {
        Self
    }
}

impl WorkbenchModule for ResearchModule {
    fn id(&self) -> &'static str {
        "research"
    }

    fn pack(&self) -> Arc<dyn DomainPack> {
        Arc::new(ResearchPack::new())
    }

    fn register_skills(&self, registry: &mut dyn SkillSink) {
        skills::register_research_skills(registry);
    }

    fn migrations(&self) -> &'static [&'static str] {
        &[include_str!("../migrations/research_schema.sql")]
    }
}

