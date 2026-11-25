//! Custos Quant Finance Domain Pack (On-Demand Installable Workbench)
//!
//! Zero-Mock Pack: all tools execute against the live injected `QuantPort` (SQLite WAL).
//! When the 'quant' workbench is not installed/enabled, this pack is not registered
//! and zero quant tools are disclosed to any agent.

use crate::contracts::QuantPort;
use crate::domain::{calculate_dsr, calculate_pbo_cscv, detect_lookahead_bias};
use crate::jobs;
use crate::roles;
use crate::services::dto::*;
use crate::services::{BacktestService, MarketService, OrderExecutionService};
use crate::verifiers;
use async_trait::async_trait;
use custos_contracts::oi::ExecutionTopology;
use custos_contracts::session::WorkbenchLens;
use custos_contracts::DomainError;
use custos_engine::contracts::pack::{
    DomainPack, DomainPackManifest, PackProfile, PackVerifier, WorkbenchJob, WorkerRole,
};
use serde_json::json;
use std::sync::Arc;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QuantPackDescriptor {
    pub name: String,
    pub version: String,
    pub features: Vec<String>,
}

pub fn get_descriptor() -> QuantPackDescriptor {
    QuantPackDescriptor {
        name: "custos-pack-quant".to_string(),
        version: "0.2.0".to_string(),
        features: vec![
            "Alpha Signal Discovery & Factor DSL (BH-FDR)".to_string(),
            "Zero-Lookahead Next-Bar Backtesting Engine".to_string(),
            "Deflated Sharpe Ratio, PBO CSCV & Evidence Quality Gate".to_string(),
            "Governed Order Routing, Exit Immunity & FIFO Review".to_string(),
        ],
    }
}

pub struct QuantPack {
    port: Option<Arc<dyn QuantPort>>,
    outbox: Option<Arc<dyn custos_engine::contracts::storage::OutboxPort>>,
}

impl QuantPack {
    pub fn new() -> Self {
        Self {
            port: None,
            outbox: None,
        }
    }

    /// Construct a live QuantPack bound to the daemon's lazily activated `QuantPort`.
    pub fn with_port(port: Arc<dyn QuantPort>) -> Self {
        Self {
            port: Some(port),
            outbox: None,
        }
    }

    /// Construct a live QuantPack bound to both `QuantPort` and `OutboxPort`.
    pub fn with_ports(
        port: Arc<dyn QuantPort>,
        outbox: Option<Arc<dyn custos_engine::contracts::storage::OutboxPort>>,
    ) -> Self {
        Self {
            port: Some(port),
            outbox,
        }
    }

    fn require_port(&self) -> Result<&dyn QuantPort, DomainError> {
        self.port.as_deref().ok_or_else(|| {
            DomainError::Validation(
                "Quant persistence port is not attached; enable the 'quant' workbench package first"
                    .into(),
            )
        })
    }
}

impl Default for QuantPack {
    fn default() -> Self {
        Self::new()
    }
}

fn tool_schema(
    name: &str,
    description: &str,
    properties: serde_json::Value,
    required: &[&str],
) -> serde_json::Value {
    json!({
        "type": "function",
        "function": {
            "name": name,
            "description": description,
            "parameters": {
                "type": "object",
                "properties": properties,
                "required": required
            }
        }
    })
}

#[async_trait]
impl DomainPack for QuantPack {
    fn manifest(&self) -> DomainPackManifest {
        DomainPackManifest {
            id: "quant".into(),
            name: "Quant Finance Pack".into(),
            version: "0.2.0".into(),
            lens: WorkbenchLens::Quant,
            description: "Alpha signal discovery, zero-lookahead backtesting, risk auditing, \
                          and governed execution planning for quantitative agents"
                .into(),
            supported_jobs: self.jobs().into_iter().map(|j| j.job_id).collect(),
            supported_roles: self.roles().into_iter().map(|r| r.role_id).collect(),
            required_capabilities: vec![],
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
            false,
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
        tracing::info!(
            pack = "quant",
            live_port = self.port.is_some(),
            "Quant Finance Pack initialized"
        );
        Ok(())
    }

    fn lane_policy(&self) -> custos_contracts::LanePolicy {
        custos_contracts::LanePolicy::quant_batch(2)
    }

    fn supervision_profile(&self) -> Option<custos_contracts::supervision::SupervisionProfile> {
        Some(crate::supervision::quant_supervision_profile())
    }

    fn tool_declarations(&self) -> Vec<serde_json::Value> {
        vec![
            tool_schema(
                "quant_bars_read",
                "Read closed historical OHLCV bars from the local Quant SQLite store",
                json!({
                    "symbol": { "type": "string" },
                    "timeframe": { "type": "string", "default": "1d" },
                    "limit": { "type": "integer", "default": 200 }
                }),
                &["symbol"],
            ),
            tool_schema(
                "quant_indicators_compute",
                "Compute zero-lookahead SMA, EMA, Wilder RSI(14), Wilder ATR(14), MACD, Bollinger Bands, and VWAP on ingested bars",
                json!({
                    "symbol": { "type": "string" },
                    "timeframe": { "type": "string", "default": "1d" },
                    "limit": { "type": "integer", "default": 200 }
                }),
                &["symbol"],
            ),
            tool_schema(
                "quant_lookahead_audit",
                "Audit an alpha factor or strategy formula for lookahead bias / future-data leakage",
                json!({ "formula": { "type": "string" } }),
                &["formula"],
            ),
            tool_schema(
                "quant_dsr_calculate",
                "Calculate Deflated Sharpe Ratio (DSR) correcting for multiple testing",
                json!({
                    "sharpe": { "type": "number" },
                    "trials_count": { "type": "integer", "default": 1 },
                    "sample_length": { "type": "integer", "default": 252 }
                }),
                &["sharpe"],
            ),
            tool_schema(
                "quant_pbo_evaluate",
                "Compute CSCV Probability of Backtest Overfitting (PBO) from a trial returns matrix",
                json!({
                    "score_matrix": { "type": "array" },
                    "max_combinations": { "type": "integer", "default": 500 }
                }),
                &["score_matrix"],
            ),
            tool_schema(
                "quant_evidence_assess",
                "Assess market evidence Coverage, Freshness, and Cross-Source Conflict (<=2%) before trade decisions",
                json!({
                    "symbol": { "type": "string" },
                    "market": { "type": "string", "default": "USStock" },
                    "required_metrics": { "type": "array" },
                    "observations": { "type": "array" }
                }),
                &["symbol", "observations"],
            ),
            tool_schema(
                "quant_factors_discover",
                "Evaluate factor DSL expressions on real ingested OHLCV bars with Benjamini-Hochberg FDR control",
                json!({
                    "symbol": { "type": "string" },
                    "timeframe": { "type": "string", "default": "1d" },
                    "formulas": { "type": "array" }
                }),
                &["symbol"],
            ),
            tool_schema(
                "run_backtest",
                "Run deterministic next-bar-open backtest simulation on real ingested OHLCV bars and persist tearsheet & fills",
                json!({
                    "strategy_id": { "type": "string" },
                    "symbol": { "type": "string" },
                    "timeframe": { "type": "string", "default": "1d" },
                    "formula": { "type": "string" },
                    "task_id": { "type": "string" }
                }),
                &["strategy_id", "symbol"],
            ),
            tool_schema(
                "quant_positions_read",
                "List active persisted positions and historical fills for a strategy",
                json!({ "strategy_id": { "type": "string" } }),
                &[],
            ),
            tool_schema(
                "quant_review_analyze",
                "Compute post-trade FIFO net PnL, holding cohorts, and exit reason diagnostics from real persisted fills",
                json!({
                    "strategy_id": { "type": "string" },
                    "symbol": { "type": "string", "default": "ALL" }
                }),
                &["strategy_id"],
            ),
            tool_schema(
                "quant_order_propose",
                "Evaluate Pre-Trade Decision Gate and draft a governed OrderTicket awaiting human permit approval (never routes directly)",
                json!({
                    "strategy_id": { "type": "string" },
                    "task_id": { "type": "string" },
                    "symbol": { "type": "string" },
                    "direction": { "type": "string" },
                    "quantity": { "type": "number" },
                    "limit_price": { "type": "number" },
                    "target_weight": { "type": "number", "default": 0.05 }
                }),
                &["strategy_id", "symbol", "direction", "quantity"],
            ),
            tool_schema(
                "quant_emergency_kill_switch",
                "Cancel all draft, awaiting-permit, approved, and routed order tickets immediately",
                json!({ "reason": { "type": "string" } }),
                &["reason"],
            ),
        ]
    }

    async fn execute_tool(
        &self,
        tool_name: &str,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, DomainError> {
        match tool_name {
            "quant_lookahead_audit" => {
                let formula = params
                    .get("formula")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| DomainError::Validation("Missing 'formula'".into()))?;
                match detect_lookahead_bias(formula) {
                    Ok(()) => Ok(json!({ "formula": formula, "lookahead_clean": true })),
                    Err(e) => Ok(
                        json!({ "formula": formula, "lookahead_clean": false, "reason": e.to_string() }),
                    ),
                }
            }
            "quant_dsr_calculate" => {
                let sharpe = params
                    .get("sharpe")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| DomainError::Validation("Missing 'sharpe'".into()))?;
                let trials = params
                    .get("trials_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1) as usize;
                let sample = params
                    .get("sample_length")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(252) as usize;
                let dsr = calculate_dsr(sharpe, trials, sample, 0.0, 3.0, 1.0);
                Ok(
                    json!({ "annualized_sharpe": sharpe, "deflated_sharpe_ratio": dsr, "trials_evaluated": trials }),
                )
            }
            "quant_pbo_evaluate" => {
                let req: ComputePboRequest = serde_json::from_value(params.clone())
                    .map_err(|e| DomainError::Validation(format!("Invalid PBO params: {e}")))?;
                let res = calculate_pbo_cscv(&req.score_matrix, req.max_combinations)?;
                serde_json::to_value(res).map_err(|e| DomainError::Validation(e.to_string()))
            }
            "quant_bars_read" => {
                let port = self.require_port()?;
                let req: GetMarketBarsRequest = serde_json::from_value(params.clone())
                    .map_err(|e| DomainError::Validation(format!("Invalid bars query: {e}")))?;
                let bars = MarketService::get_market_bars(req.clone(), port).await?;
                Ok(
                    json!({ "symbol": req.symbol, "timeframe": req.timeframe, "count": bars.len(), "bars": bars }),
                )
            }
            "quant_indicators_compute" => {
                let port = self.require_port()?;
                let req: ComputeIndicatorsRequest = serde_json::from_value(params.clone())
                    .map_err(|e| {
                        DomainError::Validation(format!("Invalid indicators params: {e}"))
                    })?;
                BacktestService::compute_indicators(req, port).await
            }
            "quant_evidence_assess" => {
                let port = self.require_port()?;
                let req: AssessEvidenceRequest =
                    serde_json::from_value(params.clone()).map_err(|e| {
                        DomainError::Validation(format!("Invalid evidence params: {e}"))
                    })?;
                let summary = MarketService::assess_evidence(req, port).await?;
                serde_json::to_value(summary).map_err(|e| DomainError::Validation(e.to_string()))
            }
            "quant_factors_discover" => {
                let port = self.require_port()?;
                let req: DiscoverFactorsRequest =
                    serde_json::from_value(params.clone()).map_err(|e| {
                        DomainError::Validation(format!("Invalid factor discover params: {e}"))
                    })?;
                MarketService::discover_factors(req, port).await
            }
            "run_backtest" => {
                let port = self.require_port()?;
                let mut req_val = params.clone();
                if let Some(obj) = req_val.as_object_mut() {
                    obj.entry("task_id".to_string())
                        .or_insert_with(|| json!("agent_backtest_task"));
                }
                let req: SubmitBacktestRequest = serde_json::from_value(req_val).map_err(|e| {
                    DomainError::Validation(format!("Invalid backtest params: {e}"))
                })?;
                if req.bars.is_none() && req.symbol.is_none() {
                    return Err(DomainError::Validation(
                        "run_backtest requires 'symbol' (with ingested SQLite bars) or explicit 'bars'; synthetic data generation is prohibited (Zero-Mock INV-11)".into(),
                    ));
                }
                if let Some(ref sym) = req.symbol {
                    let stored = port.get_market_bars(sym, &req.timeframe, 10).await?;
                    if stored.len() < 5 && req.bars.is_none() {
                        return Err(DomainError::Validation(format!(
                            "Insufficient real market bars for '{}' ({}); ingest verified OHLCV bars first",
                            sym, req.timeframe
                        )));
                    }
                }
                let sheet = BacktestService::submit_backtest(req, port).await?;
                serde_json::to_value(sheet).map_err(|e| DomainError::Validation(e.to_string()))
            }
            "quant_positions_read" => {
                let port = self.require_port()?;
                let strategy_id = params
                    .get("strategy_id")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
                let positions = MarketService::list_positions(
                    ListPositionsRequest {
                        strategy_id: strategy_id.clone(),
                    },
                    port,
                )
                .await?;
                let fills = MarketService::list_fills(
                    ListFillsRequest {
                        strategy_id,
                        symbol: None,
                        limit: 100,
                    },
                    port,
                )
                .await?;
                Ok(json!({ "positions": positions, "recent_fills": fills }))
            }
            "quant_review_analyze" => {
                let port = self.require_port()?;
                let req: ComputeReviewRequest = serde_json::from_value(params.clone())
                    .map_err(|e| DomainError::Validation(format!("Invalid review params: {e}")))?;
                let report = MarketService::compute_review_with_store(req, port).await?;
                serde_json::to_value(report).map_err(|e| DomainError::Validation(e.to_string()))
            }
            "quant_order_propose" => {
                let port = self.require_port()?;
                let symbol = params
                    .get("symbol")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let direction = params
                    .get("direction")
                    .and_then(|v| v.as_str())
                    .unwrap_or("buy")
                    .to_string();
                let quantity = params
                    .get("quantity")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let limit_price = params.get("limit_price").and_then(|v| v.as_f64());
                let strategy_id = params
                    .get("strategy_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("agent_strategy")
                    .to_string();
                let task_id = params
                    .get("task_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("agent_order_task")
                    .to_string();
                let target_weight = params
                    .get("target_weight")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.05);

                let ref_price = match limit_price {
                    Some(p) if p > 0.0 => p,
                    _ => port
                        .get_market_bars(&symbol, "1d", 1)
                        .await?
                        .last()
                        .map(|b| b.close)
                        .ok_or_else(|| {
                            DomainError::Validation(format!(
                                "No market price available for '{symbol}'"
                            ))
                        })?,
                };
                let latest_dd = port
                    .get_latest_tearsheet(&strategy_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|t| t.max_drawdown)
                    .unwrap_or(0.0);
                let gate = MarketService::evaluate_gate_with_store(
                    EvaluateGateRequest {
                        symbol: symbol.clone(),
                        direction: direction.clone(),
                        quantity,
                        estimated_price: ref_price,
                        current_drawdown: latest_dd,
                        consecutive_losses: 0,
                        daily_executed_notional: 0.0,
                        is_risk_reducing: false,
                        max_order_notional: 50_000.0,
                        max_daily_notional: 250_000.0,
                        max_drawdown_limit: 0.20,
                        max_consecutive_losses: 5,
                    },
                    port,
                )
                .await?;
                if !gate["is_allowed"].as_bool().unwrap_or(false) {
                    return Err(DomainError::Validation(format!(
                        "Pre-trade risk gate rejected order proposal: {}",
                        gate["outcome"]
                    )));
                }
                let created = OrderExecutionService::create_orders(
                    CreateOrdersRequest {
                        strategy_id,
                        task_id,
                        orders: vec![OrderSpecDto {
                            symbol,
                            direction,
                            quantity,
                            target_weight,
                            limit_price: Some(ref_price),
                            estimated_slippage_bps: Some(2.0),
                        }],
                    },
                    port,
                )
                .await?;
                Ok(json!({
                    "status": "awaiting_human_permit",
                    "gate": gate,
                    "created_orders": created
                }))
            }
            "quant_emergency_kill_switch" => {
                let port = self.require_port()?;
                let req: KillSwitchRequest =
                    serde_json::from_value(params.clone()).unwrap_or(KillSwitchRequest {
                        reason: "Agent emergency kill-switch".into(),
                    });
                OrderExecutionService::kill_switch(req, port, self.outbox.as_deref()).await
            }
            _ => Err(DomainError::Validation(format!(
                "Unsupported quant tool: '{tool_name}'"
            ))),
        }
    }
}
