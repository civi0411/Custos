//! Custos Quant Finance Domain Pack
//! SADE Master Specification Part 14 — Quant Finance Agent Workbench.

use std::sync::Arc;
use async_trait::async_trait;
use custos_domain::oi::ExecutionTopology;
use custos_domain::session::WorkbenchLens;
use custos_domain::DomainError;
use custos_core::contracts::pack::{
    DomainPack, DomainPackManifest, PackProfile, PackVerifier, WorkbenchJob, WorkerRole,
};

use crate::jobs;
use crate::roles;
use crate::verifiers;

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
            required_capabilities: vec!["notebook".into()],
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

    fn lane_policy(&self) -> custos_domain::LanePolicy {
        custos_domain::LanePolicy::quant_batch(2)
    }

    fn supervision_profile(&self) -> Option<custos_domain::supervision::SupervisionProfile> {
        Some(crate::supervision::quant_supervision_profile())
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

    async fn execute_tool(
        &self,
        tool_name: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, DomainError> {
        match tool_name {
            "run_backtest" => {
                let strategy_id = params
                    .get("strategy_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("strategy_default");
                let config = crate::domain::engine::BacktestConfig::default();
                let rule = Box::new(crate::domain::engine::DualMovingAverageRule::new(3, 8, 1.0));
                let bars: Vec<crate::domain::engine::OhlcvBar> = if let Some(raw_bars) =
                    params.get("bars").and_then(|b| b.as_array())
                {
                    raw_bars
                        .iter()
                        .filter_map(|b| serde_json::from_value(b.clone()).ok())
                        .collect()
                } else {
                    let base_ts = 1704067200;
                    let mut generated = Vec::with_capacity(30);
                    let mut price = 100.0;
                    for i in 0..30 {
                        let change = ((i as f64 * 1.7).sin() * 2.0) + 0.3;
                        let open = price;
                        let close = price + change;
                        let high = open.max(close) + 0.5;
                        let low = open.min(close) - 0.5;
                        generated.push(crate::domain::engine::OhlcvBar::new(
                            base_ts + i * 86400,
                            open,
                            high,
                            low,
                            close,
                            10_000.0,
                        ));
                        price = close;
                    }
                    generated
                };

                let tearsheet = crate::domain::engine::NativeBacktestEngine::run(
                    strategy_id,
                    &bars,
                    &config,
                    rule,
                )?;
                serde_json::to_value(&tearsheet)
                    .map_err(|e| DomainError::Validation(e.to_string()))
            }
            _ => Err(DomainError::Validation(format!(
                "Unsupported quant tool: '{tool_name}'"
            ))),
        }
    }
}
