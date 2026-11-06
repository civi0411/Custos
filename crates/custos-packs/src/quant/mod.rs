//! Custos Quant Finance Domain Pack
//! SADE Master Specification Part 14 — Quant Finance Agent Workbench.

pub mod jobs;
pub mod roles;
pub mod skills;
pub mod verifiers;

pub use jobs::*;
pub use roles::*;
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

pub struct QuantPack;

impl QuantPack {
    pub fn new() -> Self {
        Self
    }
}

impl Default for QuantPack {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DomainPack for QuantPack {
    fn manifest(&self) -> DomainPackManifest {
        DomainPackManifest {
            id: "quant".into(),
            name: "Quant Finance Pack".into(),
            version: "0.1.0".into(),
            lens: WorkbenchLens::Quant,
            description: "Alpha signal discovery, backtesting, risk auditing, \
                          and execution planning for algo finance agents"
                .into(),
            supported_jobs: self.jobs().into_iter().map(|j| j.job_id).collect(),
            supported_roles: self.roles().into_iter().map(|r| r.role_id).collect(),
        }
    }

    fn profile(&self) -> PackProfile {
        PackProfile::new(
            "quant",
            "Quant Finance Pack",
            ExecutionTopology::T2StagedPipeline,
            vec![
                ExecutionTopology::T0Direct,
                ExecutionTopology::T1SingleWorker,
                ExecutionTopology::T2StagedPipeline,
            ],
            "deflated_sharpe_verifier",
            40_000,
            false, // Blind retry on financial execution is strictly prohibited
        )
    }

    fn roles(&self) -> Vec<WorkerRole> {
        roles::quant_roles()
    }

    fn jobs(&self) -> Vec<WorkbenchJob> {
        jobs::quant_jobs()
    }

    fn verifiers(&self) -> Vec<Arc<dyn PackVerifier>> {
        vec![
            Arc::new(verifiers::deflated_sharpe::DeflatedSharpeVerifier),
            Arc::new(verifiers::lookahead_bias::LookaheadBiasChecker),
            Arc::new(verifiers::pbo_cscv::PboOverfittingVerifier),
        ]
    }

    async fn initialize(&self) -> Result<(), DomainError> {
        tracing::info!(pack = "quant", "Quant Finance Pack initialized");
        Ok(())
    }

    fn tool_declarations(&self) -> Vec<serde_json::Value> {
        vec![serde_json::json!({
            "type": "function",
            "function": {
                "name": "run_backtest",
                "description": "Execute deterministic backtest simulation against tick/candle datasets",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "strategy_id": { "type": "string", "description": "Strategy identifier" },
                        "start_date": { "type": "string", "description": "ISO date YYYY-MM-DD" },
                        "end_date": { "type": "string", "description": "ISO date YYYY-MM-DD" }
                    },
                    "required": ["strategy_id", "start_date", "end_date"]
                }
            }
        })]
    }
}
