use crate::pack::WorkerRole;

pub fn quant_roles() -> Vec<WorkerRole> {
    vec![
        WorkerRole::new(
            "alpha_scout",
            "quant",
            "Alpha Scout",
            "Discover alpha signals; compute IC/IR and Benjamini-Hochberg FDR from market data. Read-only.",
            "You are a quantitative analyst. Scan provided market data and identify \
             candidate alpha signals. Calculate IC (Information Coefficient), IR (Information Ratio), \
             and Benjamini-Hochberg FDR statistics using native quant tools. \
             Output structured JSON: { signal_name, ic_mean, ic_std, ir, q_value, classification }. \
             Never hallucinate numbers. Set lookahead_clean=true only if verified by AST inspection.",
            8_000,
            &["file_read", "lexical_search", "http_fetch", "shell_exec"],
            true,
        ),
        WorkerRole::new(
            "backtest_runner",
            "quant",
            "Backtest Runner",
            "Execute native vectorized portfolio backtests. Produces tearsheet JSON.",
            "You are a backtest execution agent. Run the strategy through the native \
             Rust simulation engine with slippage, commission, and protection parameters. \
             Report: { annualized_sharpe, deflated_sharpe, max_drawdown, total_return, turnover, win_rate, profit_factor }. \
             Flag suspiciously perfect results (Sharpe > 4) for mandatory human review.",
            20_000,
            &["file_read", "shell_exec"],
            false,
        ),
        WorkerRole::new(
            "risk_supervisor",
            "quant",
            "Risk Supervisor",
            "Audit backtest for DSR, lookahead bias, and overfitting indicators. Read-only.",
            "You are a risk and compliance auditor for algorithmic strategies. \
             Read the backtest tearsheet. Apply Bailey-de Prado Deflated Sharpe Ratio logic. \
             Scan signal logic for lookahead bias patterns and verify drawdown & leverage limits. \
             Output: { dsr_value, lookahead_clean: bool, overfit_risk_score: 0-1, verdict: string }.",
            10_000,
            &["file_read"],
            true,
        ),
        WorkerRole::new(
            "exec_broker",
            "quant",
            "Execution Broker",
            "Generate order tickets with slippage model, protection rules, and circuit breakers for human approval.",
            "You are an execution planning agent. Given an approved signal and risk audit, \
             generate: { target_positions: dict, slippage_estimate_bps, protection: { stop_loss_pct, take_profit_pct }, \
             risk_permit_id, circuit_breaker_rules }. \
             Never execute unapproved trades. All output is queued for human approval via Outbox.",
            15_000,
            &["file_read", "shell_exec"],
            false,
        ),
    ]
}
