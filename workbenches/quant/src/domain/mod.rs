//! Quant Bounded Context
//!
//! Models financial domain entities for the Quantitative Trading Workbench:
//! - `factor`: AlphaFactor, predictive metrics (IC, IR), factor decay, lookahead validation
//! - `backtest`: BacktestTearsheet, MonthlyReturn, Sharpe, Deflated Sharpe, Calmar ratios
//! - `risk`: RiskPermitConfig, RiskBreach, leverage & drawdown audit hurdles
//! - `order`: OrderTicket, OrderDirection, OrderType, OrderTicketStatus, and OutboxActionItem bridge

pub mod backtest;
pub mod contracts;
pub mod engine;
pub mod factor;
pub mod gate;
pub mod grid;
pub mod lease;
pub mod metrics;
pub mod order;
pub mod orderbook;
pub mod pbo;
pub mod pipeline_invariant;
pub mod portal;
pub mod review;
pub mod risk;

pub use backtest::{calculate_dsr, normal_cdf, probit, BacktestTearsheet, MonthlyReturn};
pub use contracts::{
    ExchangeOrderAdapter, ExecutionStatus, FillSnapshot, LimitThenMarketExecutor,
    MarketOrderExecutor, MarketType, OrderExecutionStrategy, OrderIntent, OrderSide,
    PositionSnapshot, RestingLimitExecutor,
};
pub use engine::{
    BacktestConfig, DualMovingAverageRule, NativeBacktestEngine, OhlcvBar, SignalAction,
    StrategyRule,
};
pub use factor::{
    compute_rank_ic, detect_lookahead_bias, AlphaFactor, BenjaminiHochberg, FactorClassification,
};
pub use gate::{DecisionContext, GateOutcome, PreTradeDecisionGate};
pub use grid::{
    GridCell, GridConfig, GridMode, GridOrderAction, GridStatusSummary, NativeGridEngine,
};
pub use lease::{
    LeaseGuard, LeaseRecord, LeaseTier, StrategyCommand, StrategyCommandStatus, StrategyCommandType,
};
pub use metrics::{
    calculate_capture_ratios, calculate_cvar, calculate_downside_deviation, calculate_dual_beta,
    calculate_omega_ratio, calculate_sortino_ratio, compute_institutional_suite,
    InstitutionalMetrics,
};
pub use order::{OrderDirection, OrderTicket, OrderTicketStatus, OrderType};
pub use orderbook::{OrderBookLevel, OrderBookSnapshot};
pub use pbo::{calculate_pbo_cscv, PboResult};
pub use pipeline_invariant::ClosedBarInvariant;
pub use portal::{BarTimeframe, BollingerBandsResult, MacdResult, NativeDataPortal};
pub use review::{
    DiagnosticCheck, ExitReasonCohort, HoldingCohort, PostTradeAnalyzer, PostTradeReviewReport,
    TradeFillRecord,
};
pub use risk::{ProtectionSpec, ProtectionState, RiskBreach, RiskPermitConfig};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deflated_sharpe_ratio_math_and_extreme_value_theory() {
        // Single trial, Sharpe 1.5, T=252 days -> DSR should be very high (> 0.99)
        let dsr_single = calculate_dsr(1.5, 1, 252, 0.0, 3.0, 1.0);
        assert!(
            dsr_single > 0.99,
            "Expected high DSR for single trial, got {dsr_single}"
        );

        // 100 trials with same Sharpe 1.5, T=252 -> DSR drops due to multiple testing
        let dsr_multi = calculate_dsr(1.5, 100, 252, 0.0, 3.0, 1.0);
        assert!(dsr_multi < dsr_single, "DSR must penalize multiple testing (DSR_multi: {dsr_multi} vs DSR_single: {dsr_single})");

        // Negative Sharpe -> DSR near zero
        let dsr_neg = calculate_dsr(-0.5, 10, 252, 0.0, 3.0, 1.0);
        assert!(dsr_neg < 0.01, "Negative Sharpe must produce near-zero DSR");
    }

    #[test]
    fn test_detect_lookahead_bias_scanner() {
        assert!(detect_lookahead_bias("ts_rank(close / delay(close, 5), 20)").is_ok());
        assert!(detect_lookahead_bias("close.shift(-1) / close").is_err());
        assert!(detect_lookahead_bias("lead(close, 1)").is_err());
        assert!(detect_lookahead_bias("future_price - close").is_err());
    }

    #[test]
    fn test_alpha_factor_lifecycle_and_validation() {
        let factor = AlphaFactor::new(
            "momentum_5d",
            "5-day rolling cross-sectional price momentum",
            "ts_rank(close / delay(close, 5), 20)",
            "us_equities_top500",
            5,
        )
        .expect("valid factor");

        assert_eq!(factor.name, "momentum_5d");
        assert!(!factor.is_viable(), "Initial factor should not be viable");

        let evaluated = factor.with_metrics(0.045, 0.05, true);
        assert!(
            evaluated.is_viable(),
            "Factor with IC=0.045, IR=0.9 and clean lookahead must be viable"
        );

        // Validation failures
        assert!(AlphaFactor::new("", "hyp", "formula", "univ", 5).is_err());
        assert!(AlphaFactor::new("name", "hyp", "", "univ", 5).is_err());
        assert!(AlphaFactor::new("name", "hyp", "formula", "univ", 0).is_err());
    }

    #[test]
    fn test_backtest_tearsheet_and_hurdle_check() {
        let sheet = BacktestTearsheet::new("momentum_v1", 1.85, 0.12, 0.35)
            .expect("valid tearsheet")
            .with_deflated_sharpe(1.2)
            .with_extended_metrics(
                0.15,
                0.58,
                1.9,
                2.4,
                vec![
                    MonthlyReturn {
                        year: 2026,
                        month: 1,
                        return_pct: 0.035,
                    },
                    MonthlyReturn {
                        year: 2026,
                        month: 2,
                        return_pct: 0.041,
                    },
                ],
            );

        assert!(sheet.is_passing_hurdle(1.0, 0.15));
        assert!(!sheet.is_passing_hurdle(2.0, 0.15));
        assert!(!sheet.is_passing_hurdle(1.0, 0.10));
        assert!((sheet.calmar_ratio - (0.35 / 0.12)).abs() < 1e-4);

        // Validation failures
        assert!(BacktestTearsheet::new("", 1.0, 0.1, 0.2).is_err());
        assert!(BacktestTearsheet::new("strat", 1.0, -0.1, 0.2).is_err());
        assert!(BacktestTearsheet::new("strat", 1.0, 1.5, 0.2).is_err());
    }

    #[test]
    fn test_risk_permit_config_audits() {
        let risk = RiskPermitConfig::new("momentum_v1", "permit_123", 0.10, 1.5, 0.05)
            .expect("valid risk permit")
            .with_drawdown_and_dsr_limits(0.15, 0.5);

        let good_sheet = BacktestTearsheet::new("momentum_v1", 1.5, 0.10, 0.25)
            .unwrap()
            .with_deflated_sharpe(0.8);
        assert!(risk.audit_tearsheet(&good_sheet).is_ok());

        let bad_dd_sheet = BacktestTearsheet::new("momentum_v1", 1.5, 0.22, 0.25)
            .unwrap()
            .with_deflated_sharpe(0.8);
        match risk.audit_tearsheet(&bad_dd_sheet) {
            Err(RiskBreach::DrawdownExceeded { actual, limit }) => {
                assert_eq!(actual, 0.22);
                assert_eq!(limit, 0.15);
            }
            other => panic!("Expected DrawdownExceeded, got {other:?}"),
        }

        let bad_dsr_sheet = BacktestTearsheet::new("momentum_v1", 1.5, 0.10, 0.25)
            .unwrap()
            .with_deflated_sharpe(0.2);
        match risk.audit_tearsheet(&bad_dsr_sheet) {
            Err(RiskBreach::DsrBelowThreshold { actual, threshold }) => {
                assert_eq!(actual, 0.2);
                assert_eq!(threshold, 0.5);
            }
            other => panic!("Expected DsrBelowThreshold, got {other:?}"),
        }

        // Position sizing checks
        assert!(risk.validate_order_weight(0.08).is_ok());
        assert!(risk.validate_order_weight(0.12).is_err());
    }

    #[test]
    fn test_order_ticket_and_outbox_handoff() {
        let mut ticket = OrderTicket::new("momentum_v1", "AAPL", OrderDirection::Buy, 100.0, 0.05)
            .expect("valid order ticket")
            .with_limit_price(182.50)
            .with_algorithmic_execution(OrderType::Twap, 3.5);

        assert_eq!(ticket.status, OrderTicketStatus::Draft);
        assert!(!ticket.approved_by_human);

        // Convert to OutboxActionItem while in draft
        let outbox_item = ticket.to_outbox_action_item("task_quant_99");
        assert_eq!(outbox_item.action_type, "quant_order_execution");
        assert_eq!(outbox_item.status, "pending_approval");
        assert!(outbox_item.payload_digest.len() == 64);
        assert!(outbox_item.payload_json.is_some());

        // Approve ticket
        ticket.approve_by_human(Some("permit_live_77".into()));
        assert_eq!(ticket.status, OrderTicketStatus::Approved);
        assert!(ticket.approved_by_human);

        let approved_outbox = ticket.to_outbox_action_item("task_quant_99");
        assert_eq!(approved_outbox.status, "approved");
        assert_eq!(approved_outbox.permit_id.as_deref(), Some("permit_live_77"));
    }

    #[test]
    fn test_benjamini_hochberg_fdr_and_classification() {
        let p_values = vec![0.001, 0.008, 0.03, 0.12, 0.45];
        let q_values = BenjaminiHochberg::compute_q_values(&p_values);
        assert_eq!(q_values.len(), 5);
        // Smallest p-value has smallest q-value
        assert!(q_values[0] < q_values[3]);

        // Classification test: stable positive IC with significant q-value -> Alive
        let class_alive = BenjaminiHochberg::classify_factor(0.05, 0.04, 0.02, 0.05, 0.02, 50);
        assert_eq!(class_alive, FactorClassification::Alive);

        // Sign flipped between in-sample and out-of-sample -> Reversed
        let class_reversed = BenjaminiHochberg::classify_factor(0.05, -0.04, 0.02, 0.05, 0.02, 50);
        assert_eq!(class_reversed, FactorClassification::Reversed);
    }

    #[test]
    fn test_spearman_rank_correlation_ic() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let perfect_ic = compute_rank_ic(&x, &y);
        assert!((perfect_ic - 1.0).abs() < 1e-4);

        let y_inv = vec![10.0, 8.0, 6.0, 4.0, 2.0];
        let neg_ic = compute_rank_ic(&x, &y_inv);
        assert!((neg_ic - (-1.0)).abs() < 1e-4);
    }

    #[test]
    fn test_native_backtest_simulation_dual_ma() {
        // Generate synthetic upward trending bars
        let mut bars = Vec::new();
        let mut price = 100.0;
        for i in 0..50 {
            price += if i % 2 == 0 { 1.5 } else { -0.5 };
            bars.push(OhlcvBar::new(
                i as i64 * 86400,
                price - 0.2,
                price + 0.5,
                price - 0.5,
                price,
                1000.0,
            ));
        }

        let config = BacktestConfig {
            initial_capital: 10_000.0,
            commission_rate: 0.0005,
            slippage_bps: 1.0,
            leverage: 1.0,
            protection: Some(ProtectionSpec::new(0.05, 0.15, 0.03, 0.05)),
        };

        let rule = Box::new(DualMovingAverageRule::new(3, 8, 1.0));
        let sheet = NativeBacktestEngine::run("trend_follow_v1", &bars, &config, rule)
            .expect("simulation succeeded");

        assert_eq!(sheet.strategy_id, "trend_follow_v1");
        assert!(
            sheet.total_return > 0.0,
            "Trend-following on trending bars must generate positive return"
        );
        assert!(sheet.max_drawdown >= 0.0);
        assert!(sheet.sortino_ratio != 0.0);
        assert!(sheet.omega_ratio > 0.0);
    }

    #[test]
    fn test_native_grid_engine_arithmetic_and_geometric() {
        let arith_levels =
            NativeGridEngine::generate_levels(100.0, 200.0, 5, GridMode::Arithmetic).unwrap();
        assert_eq!(arith_levels, vec![100.0, 125.0, 150.0, 175.0, 200.0]);

        let geo_levels =
            NativeGridEngine::generate_levels(100.0, 400.0, 3, GridMode::Geometric).unwrap();
        assert_eq!(geo_levels, vec![100.0, 200.0, 400.0]);

        let cfg = GridConfig::new("BTCUSDT", 100.0, 200.0, 5, GridMode::Arithmetic, 10_000.0)
            .unwrap()
            .with_protection(Some(90.0), Some(220.0));

        let mut engine = NativeGridEngine::new(cfg).unwrap();
        assert_eq!(engine.cells.len(), 4);

        // Baseline tick at 110.0 (inside Cell 0) establishes price watermark
        let init_actions = engine.evaluate_price(110.0);
        assert_eq!(init_actions.len(), 0);

        // Price drops into cell 0 lower bound (100.0) -> triggers Buy Limit
        let actions = engine.evaluate_price(99.0);
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            GridOrderAction::PlaceLimit {
                side, cell_index, ..
            } => {
                assert_eq!(*side, OrderDirection::Buy);
                assert_eq!(*cell_index, 0);
            }
            _ => panic!("Expected PlaceLimit Buy"),
        }

        // Price rises above cell 0 upper bound (125.0) -> triggers paired Sell Limit and locks profit
        let actions_exit = engine.evaluate_price(126.0);
        assert_eq!(actions_exit.len(), 1);
        match &actions_exit[0] {
            GridOrderAction::PlaceLimit {
                side, cell_index, ..
            } => {
                assert_eq!(*side, OrderDirection::Sell);
                assert_eq!(*cell_index, 0);
            }
            _ => panic!("Expected PlaceLimit Sell"),
        }
        assert!(engine.total_realized_pnl() > 0.0);

        // Price crashes below stop loss -> triggers emergency stop
        let stop_actions = engine.evaluate_price(89.0);
        assert_eq!(stop_actions.len(), 1);
        match &stop_actions[0] {
            GridOrderAction::TriggerEmergencyStop { reason } => {
                assert!(reason.contains("Stop loss breached"));
            }
            _ => panic!("Expected EmergencyStop"),
        }
        assert!(engine.is_stopped);
    }

    #[test]
    fn test_institutional_metrics_suite_calculation() {
        let returns = vec![0.02, 0.01, -0.01, 0.03, -0.02, 0.015, -0.005, 0.025];
        let benchmark = vec![0.01, 0.005, -0.015, 0.02, -0.01, 0.01, -0.005, 0.015];

        let suite = compute_institutional_suite(&returns, Some(&benchmark), 252.0);
        assert!(suite.sortino_ratio > 0.0);
        assert!(suite.downside_deviation > 0.0);
        assert!(suite.omega_ratio > 1.0);
        assert!(suite.var_95 >= 0.0);
        assert!(suite.cvar_95 >= suite.var_95);
        assert!(suite.bull_beta > 0.0);
        assert!(suite.bear_beta > 0.0);
    }

    #[test]
    fn test_native_data_portal_zero_lookahead_and_indicators() {
        let tf = BarTimeframe::Hour1;
        assert_eq!(tf.duration_seconds(), 3600);
        assert!(tf.is_bar_closed(0, 3600));
        assert!(!tf.is_bar_closed(0, 3599));

        // Create 5 1-hour bars:
        // Bar 0: open=0, close_ts=3600, price=100
        // Bar 1: open=3600, close_ts=7200, price=102
        // Bar 2: open=7200, close_ts=10800, price=105
        // Bar 3: open=10800, close_ts=14400, price=103
        // Bar 4: open=14400, close_ts=18000, price=108
        let bars = vec![
            OhlcvBar::new(0, 99.0, 101.0, 98.0, 100.0, 1000.0),
            OhlcvBar::new(3600, 100.0, 103.0, 99.5, 102.0, 1200.0),
            OhlcvBar::new(7200, 102.0, 106.0, 101.0, 105.0, 1500.0),
            OhlcvBar::new(10800, 105.0, 105.5, 102.0, 103.0, 1100.0),
            OhlcvBar::new(14400, 103.0, 109.0, 103.0, 108.0, 1800.0),
        ];

        let portal = NativeDataPortal::new(&bars, tf).expect("valid sorted bars");
        assert_eq!(portal.total_bars(), 5);

        // At T = 3599: Bar 0 has not closed yet! Visible slice must be empty
        let slice_0 = portal.slice_point_in_time(3599);
        assert!(
            slice_0.is_empty(),
            "No bar can be visible before it fully closes"
        );
        assert!(portal.current_closed_bar(3599).is_none());

        // At T = 3600: Bar 0 has exactly closed! Visible slice has 1 bar
        let slice_1 = portal.slice_point_in_time(3600);
        assert_eq!(slice_1.len(), 1);
        assert_eq!(slice_1[0].timestamp, 0);
        assert_eq!(portal.current_closed_bar(3600).unwrap().close, 100.0);

        // At T = 10799: Bar 0 and Bar 1 are closed; Bar 2 (opens at 7200, closes at 10800) is NOT closed
        let slice_2 = portal.slice_point_in_time(10799);
        assert_eq!(slice_2.len(), 2);
        assert_eq!(slice_2[1].timestamp, 3600);

        // At T = 18000: All 5 bars are closed
        let slice_all = portal.slice_point_in_time(18000);
        assert_eq!(slice_all.len(), 5);
        assert_eq!(portal.current_closed_bar(18000).unwrap().close, 108.0);

        // History window test: request last 3 bars at T = 18000
        let window = portal.history_window(18000, 3);
        assert_eq!(window.len(), 3);
        assert_eq!(window[0].close, 105.0);
        assert_eq!(window[1].close, 103.0);
        assert_eq!(window[2].close, 108.0);

        // SMA Calculation: period 3 at T = 18000 -> (105 + 103 + 108) / 3 = 105.3333...
        let sma = portal.calculate_sma(18000, 3).expect("sma calculated");
        assert!((sma - (316.0 / 3.0)).abs() < 1e-6);

        // EMA & RSI calculation sanity check
        let ema = portal.calculate_ema(18000, 3).expect("ema calculated");
        assert!(ema > 100.0);
        let rsi = portal.calculate_rsi(18000, 3).expect("rsi calculated");
        assert!((0.0..=100.0).contains(&rsi));

        // Bollinger Bands test
        let bb = portal
            .calculate_bollinger_bands(18000, 3, 2.0)
            .expect("bb calculated");
        assert_eq!(bb.middle, sma);
        assert!(bb.upper > bb.middle);
        assert!(bb.lower < bb.middle);
        assert!(bb.bandwidth > 0.0);

        // MACD test
        let macd = portal
            .calculate_macd(18000, 2, 4, 2)
            .expect("macd calculated");
        assert!(!macd.macd.is_nan());
        assert!(!macd.signal.is_nan());

        // ATR & VWAP test
        let atr = portal.calculate_atr(18000, 3).expect("atr calculated");
        assert!(atr > 0.0);
        let vwap = portal.calculate_vwap(18000, 5).expect("vwap calculated");
        assert!(vwap > 90.0 && vwap < 120.0);
    }
}
