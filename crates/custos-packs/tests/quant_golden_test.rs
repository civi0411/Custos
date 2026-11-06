//! Quant Pack Golden Acceptance Test
//!
//! Validates the end-to-end financial research-to-execution workflow across:
//! 1. Alpha Factor Signal Discovery & Lookahead Bias Audit
//! 2. Backtest Tearsheet Simulation & Deflated Sharpe Verification
//! 3. Quantitative Risk Permit Configuration & Auditing
//! 4. Algorithmic Order Ticket Generation & Governed Outbox Handoff

use custos_domain::quant::{
    AlphaFactor, BacktestTearsheet, MonthlyReturn, OrderDirection, OrderTicket,
    OrderTicketStatus, OrderType, RiskBreach, RiskPermitConfig,
};
use custos_domain::session::WorkbenchLens;
use custos_packs::create_default_pack_registry;
use custos_packs::pack::DomainPack;
use custos_packs::quant::QuantPack;
use custos_packs::verifier::{VerificationContext, VerificationResult};

#[tokio::test]
async fn test_quant_pack_manifest_roles_jobs_and_verifiers() {
    let pack = QuantPack::new();
    let manifest = pack.manifest();

    assert_eq!(manifest.id, "quant");
    assert_eq!(manifest.lens, WorkbenchLens::Quant);

    let roles = pack.roles();
    assert_eq!(roles.len(), 4);
    assert!(roles.iter().any(|r| r.role_id == "alpha_scout"));
    assert!(roles.iter().any(|r| r.role_id == "backtest_runner"));
    assert!(roles.iter().any(|r| r.role_id == "risk_supervisor"));
    assert!(roles.iter().any(|r| r.role_id == "exec_broker"));

    let jobs = pack.jobs();
    assert_eq!(jobs.len(), 4);
    assert!(jobs.iter().any(|j| j.job_id == "quant__alpha_discover"));
    assert!(jobs.iter().any(|j| j.job_id == "quant__backtest_full"));
    assert!(jobs.iter().any(|j| j.job_id == "quant__risk_audit"));
    assert!(jobs.iter().any(|j| j.job_id == "quant__execution_plan"));

    let verifiers = pack.verifiers();
    assert_eq!(verifiers.len(), 3);
    assert!(verifiers.iter().any(|v| v.verifier_id() == "deflated_sharpe_verifier"));
    assert!(verifiers.iter().any(|v| v.verifier_id() == "lookahead_bias_checker"));
    assert!(verifiers.iter().any(|v| v.verifier_id() == "pbo_overfitting_verifier"));
}

#[tokio::test]
async fn test_end_to_end_quant_research_to_execution_golden_flow() {
    let registry = create_default_pack_registry();
    let lookahead_verifier = registry
        .get_verifier("lookahead_bias_checker")
        .expect("lookahead_bias_checker must be registered");
    let dsr_verifier = registry
        .get_verifier("deflated_sharpe_verifier")
        .expect("deflated_sharpe_verifier must be registered");

    // =========================================================================
    // Stage 1: Alpha Factor Discovery & Lookahead Bias Audit
    // =========================================================================
    let mut factor = AlphaFactor::new(
        "momentum_reversal_5d",
        "Short-term mean reversion conditioned on 5d return and volume spikes",
        "-1 * ts_zscore(close / delay(close, 5), 20) * ts_rank(volume, 20)",
        "sp500_liquid",
        5,
    )
    .expect("valid alpha factor");

    // Fail check before lookahead bias audit
    let unverified_ctx = VerificationContext::new("task_quant_alpha_01").with_metadata(serde_json::json!({
        "lookahead_clean": factor.lookahead_clean,
        "ic_mean": 0.045,
    }));
    match lookahead_verifier.verify(&unverified_ctx).await {
        VerificationResult::Fail { reason } => {
            assert!(reason.contains("Lookahead bias not confirmed clean"));
        }
        other => panic!("Expected Fail, got {other:?}"),
    };

    // Mark verified clean after alpha_scout AST audit
    factor.mark_lookahead_checked(true);
    let factor = factor.with_metrics(0.045, 0.04, true);
    assert!(factor.is_viable());

    let clean_ctx = VerificationContext::new("task_quant_alpha_01").with_metadata(serde_json::json!({
        "lookahead_clean": factor.lookahead_clean,
        "ic_mean": factor.ic_mean,
        "ir": factor.ir,
    }));
    match lookahead_verifier.verify(&clean_ctx).await {
        VerificationResult::Pass { detail } => {
            assert!(detail.contains("Lookahead bias audit confirmed clean"));
        }
        other => panic!("Expected Pass, got {other:?}"),
    }

    // =========================================================================
    // Stage 2: Vectorized Backtest & Deflated Sharpe Verification
    // =========================================================================
    let tearsheet = BacktestTearsheet::new(&factor.name, 1.95, 0.11, 0.38)
        .expect("valid tearsheet")
        .with_deflated_sharpe(1.42)
        .with_extended_metrics(
            0.14,
            0.56,
            2.1,
            3.2,
            vec![
                MonthlyReturn { year: 2026, month: 1, return_pct: 0.04 },
                MonthlyReturn { year: 2026, month: 2, return_pct: 0.035 },
            ],
        );

    // Verify DSR Hurdle
    let dsr_ctx = VerificationContext::new("task_quant_backtest_02").with_metadata(serde_json::json!({
        "annualized_sharpe": tearsheet.annualized_sharpe,
        "dsr_value": tearsheet.deflated_sharpe,
        "max_drawdown": tearsheet.max_drawdown,
    }));
    match dsr_verifier.verify(&dsr_ctx).await {
        VerificationResult::Pass { detail } => {
            assert!(detail.contains("DSR/Sharpe passed"));
        }
        other => panic!("Expected Pass, got {other:?}"),
    }

    // =========================================================================
    // Stage 3: Risk Permit Auditing & Circuit Breakers
    // =========================================================================
    let risk_permit = RiskPermitConfig::new(&factor.name, "permit_live_alloc_88", 0.08, 1.5, 0.04)
        .expect("valid risk permit")
        .with_drawdown_and_dsr_limits(0.15, 0.5);

    // Ensure tearsheet satisfies risk hurdles
    assert!(risk_permit.audit_tearsheet(&tearsheet).is_ok());

    // Breaches are caught deterministically:
    let bad_tearsheet = BacktestTearsheet::new("reckless_strat", 2.5, 0.22, 0.50)
        .unwrap()
        .with_deflated_sharpe(0.1);
    assert!(matches!(
        risk_permit.audit_tearsheet(&bad_tearsheet),
        Err(RiskBreach::DrawdownExceeded { .. })
    ));

    // =========================================================================
    // Stage 4: Order Generation & Governed Cross-Workbench Outbox Handoff
    // =========================================================================
    let mut order = OrderTicket::new(
        &factor.name,
        "NVDA",
        OrderDirection::Buy,
        50.0,
        0.06, // 6% portfolio weight <= 8% max limit
    )
    .expect("valid order ticket")
    .with_limit_price(124.50)
    .with_algorithmic_execution(OrderType::Twap, 4.0);

    // Validate against risk permit limits
    assert!(risk_permit.validate_order_weight(order.target_weight).is_ok());

    // Sizing breach test
    assert!(risk_permit.validate_order_weight(0.10).is_err());

    // Stage 4b: Stage in Outbox for Human Approval (Cross-domain bridge)
    let outbox_item = order.to_outbox_action_item("task_quant_exec_04");
    assert_eq!(outbox_item.action_type, "quant_order_execution");
    assert_eq!(outbox_item.status, "pending_approval");
    assert!(outbox_item.payload_digest.len() == 64);
    assert!(outbox_item.permit_id.is_none());

    // Human grants approval
    order.approve_by_human(Some(risk_permit.permit_id.clone()));
    assert_eq!(order.status, OrderTicketStatus::Approved);
    assert!(order.approved_by_human);

    let approved_outbox_item = order.to_outbox_action_item("task_quant_exec_04");
    assert_eq!(approved_outbox_item.status, "approved");
    assert_eq!(
        approved_outbox_item.permit_id.as_deref(),
        Some("permit_live_alloc_88")
    );
}
