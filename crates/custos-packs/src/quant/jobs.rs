use custos_domain::oi::ExecutionTopology;
use custos_domain::session::WorkbenchLens;
use crate::pack::WorkbenchJob;

pub fn quant_jobs() -> Vec<WorkbenchJob> {
    vec![
        WorkbenchJob::new(
            "quant__alpha_discover",
            WorkbenchLens::Quant,
            "Alpha Signal Discovery",
            "Scan market data for alpha signals; compute IC, IR, and factor decay profile.",
            &["market_data_path", "signal_hypothesis"],
            &["alpha_report_json"],
            ExecutionTopology::T1SingleWorker,
            "alpha_scout",
            &["lookahead_bias_checker"],
            "IC mean > 0.02, IR > 0.5, lookahead_clean = true",
        ),
        WorkbenchJob::new(
            "quant__backtest_full",
            WorkbenchLens::Quant,
            "Full Vectorized Backtest",
            "Execute end-to-end backtest. Produces PnL curve, Sharpe, drawdown metrics.",
            &["alpha_signal_id", "backtest_config_path"],
            &["tearsheet_json", "equity_curve_csv"],
            ExecutionTopology::T2StagedPipeline,
            "backtest_runner",
            &["deflated_sharpe_verifier"],
            "annualized_sharpe > 0.8, max_drawdown < 0.20, DSR > 0",
        ),
        WorkbenchJob::new(
            "quant__risk_audit",
            WorkbenchLens::Quant,
            "Risk & Validity Audit",
            "Audit existing backtest result for DSR, lookahead bias, and overfitting risk.",
            &["tearsheet_json_path"],
            &["risk_audit_report_json"],
            ExecutionTopology::T0Direct,
            "risk_supervisor",
            &["deflated_sharpe_verifier", "lookahead_bias_checker"],
            "dsr > 0, lookahead_clean = true, overfit_risk_score < 0.5",
        ),
        WorkbenchJob::new(
            "quant__execution_plan",
            WorkbenchLens::Quant,
            "Execution Plan Generation",
            "Generate order tickets with slippage model and circuit breakers for human approval.",
            &["approved_signal_id", "risk_audit_id", "capital_usd"],
            &["execution_plan_json"],
            ExecutionTopology::T1SingleWorker,
            "exec_broker",
            &["user_acceptance_oracle"],
            "Human reviews and approves plan; circuit_breakers configured",
        ),
    ]
}
