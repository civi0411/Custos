use super::fixtures::setup_quant_env;
use crate::api::*;

#[tokio::test]
async fn test_quant_orders_lifecycle_and_outbox_governance() {
    let (dispatcher, task_id) = setup_quant_env().await;

    // 1. Create Order Tickets
    let create_req = ApiRequest {
        id: "orders_create".into(),
        method: METHOD_QUANT_ORDERS_CREATE.into(),
        params: serde_json::json!({
            "strategy_id": "strat_momentum_1",
            "task_id": task_id,
            "orders": [
                {
                    "symbol": "NVDA",
                    "direction": "buy",
                    "quantity": 120.0,
                    "target_weight": 0.12,
                    "estimated_slippage_bps": 2.5
                },
                {
                    "symbol": "INTC",
                    "direction": "sell_short",
                    "quantity": 90.0,
                    "target_weight": -0.09,
                    "limit_price": 24.50
                }
            ]
        }),
    };

    let create_resp = dispatcher.handle_request(create_req).await;
    assert!(create_resp.is_success(), "{:?}", create_resp.error);
    let created = create_resp.result.unwrap();
    let orders_arr = created["created_orders"].as_array().unwrap();
    assert_eq!(orders_arr.len(), 2);
    assert_eq!(orders_arr[0]["status"], "draft");
    assert_eq!(orders_arr[1]["status"], "draft");

    let order_id = orders_arr[0]["id"].as_str().unwrap().to_string();

    // 2. List Orders
    let list_req = ApiRequest {
        id: "orders_list".into(),
        method: METHOD_QUANT_ORDERS_LIST.into(),
        params: serde_json::json!({
            "strategy_id": "strat_momentum_1",
            "task_id": task_id,
        }),
    };

    let list_resp = dispatcher.handle_request(list_req).await;
    assert!(list_resp.is_success(), "{:?}", list_resp.error);
    let list_arr = list_resp.result.unwrap()["orders"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(list_arr.len(), 2);

    // 3. Human-in-the-Loop Operator signs permit and approves order
    let approve_req = ApiRequest {
        id: "orders_approve".into(),
        method: METHOD_QUANT_ORDERS_APPROVE.into(),
        params: serde_json::json!({
            "order_id": order_id,
            "permit_id": "permit_risk_audit_passed_2026",
            "task_id": task_id,
        }),
    };

    let approve_resp = dispatcher.handle_request(approve_req).await;
    assert!(approve_resp.is_success(), "{:?}", approve_resp.error);
    let approved_res = approve_resp.result.unwrap();
    assert_eq!(approved_res["status"], "approved");
    assert_eq!(approved_res["permit_id"], "permit_risk_audit_passed_2026");

    // 4. Verify Outbox Governance: approved order MUST be enqueued in Outbox!
    let outbox_req = ApiRequest {
        id: "outbox_check".into(),
        method: METHOD_ASSISTANT_OUTBOX_LIST.into(),
        params: serde_json::json!({
            "task_id": task_id,
        }),
    };

    let outbox_resp = dispatcher.handle_request(outbox_req).await;
    assert!(outbox_resp.is_success(), "{:?}", outbox_resp.error);
    let outbox_items = outbox_resp.result.unwrap().as_array().unwrap().clone();
    assert!(
        !outbox_items.is_empty(),
        "Approved order must be bridged to Outbox entries"
    );
    let quant_item = outbox_items
        .iter()
        .find(|item| item["action_type"] == "quant_order_execution")
        .unwrap();
    assert_eq!(quant_item["permit_id"], "permit_risk_audit_passed_2026");
    assert_eq!(quant_item["status"], "approved");
}

#[tokio::test]
async fn test_quant_gate_evaluate_rpc() {
    let (dispatcher, _) = setup_quant_env().await;

    // 1. Test normal entry pass
    let req = ApiRequest {
        id: "gate_entry_pass".into(),
        method: METHOD_QUANT_GATE_EVALUATE.into(),
        params: serde_json::json!({
            "symbol": "BTCUSDT",
            "direction": "buy",
            "quantity": 0.1,
            "estimated_price": 50_000.0,
            "max_order_notional": 10_000.0,
        }),
    };
    let resp = dispatcher.handle_request(req).await;
    assert!(resp.is_success(), "{:?}", resp.error);
    let res = resp.result.unwrap();
    assert_eq!(res["is_allowed"], true);
    assert_eq!(res["order_notional"], 5000.0);

    // 2. Test exit immunity (always allowed even if huge)
    let exit_req = ApiRequest {
        id: "gate_exit_immunity".into(),
        method: METHOD_QUANT_GATE_EVALUATE.into(),
        params: serde_json::json!({
            "symbol": "BTCUSDT",
            "direction": "sell",
            "quantity": 100.0,
            "estimated_price": 50_000.0,
            "is_risk_reducing": true,
            "max_order_notional": 10_000.0,
        }),
    };
    let exit_resp = dispatcher.handle_request(exit_req).await;
    assert!(exit_resp.is_success(), "{:?}", exit_resp.error);
    let exit_res = exit_resp.result.unwrap();
    assert_eq!(exit_res["is_allowed"], true);
    assert_eq!(exit_res["outcome"]["status"], "BypassExitImmunity");

    // 3. Test notional cap breach rejection
    let breach_req = ApiRequest {
        id: "gate_breach".into(),
        method: METHOD_QUANT_GATE_EVALUATE.into(),
        params: serde_json::json!({
            "symbol": "BTCUSDT",
            "direction": "buy",
            "quantity": 1.0,
            "estimated_price": 50_000.0,
            "max_order_notional": 10_000.0,
        }),
    };
    let breach_resp = dispatcher.handle_request(breach_req).await;
    assert!(breach_resp.is_success(), "{:?}", breach_resp.error);
    let breach_res = breach_resp.result.unwrap();
    assert_eq!(breach_res["is_allowed"], false);
    assert_eq!(breach_res["outcome"]["status"], "Reject");
    assert_eq!(
        breach_res["outcome"]["details"]["breach_type"],
        "MaxOrderNotionalExceeded"
    );
}

#[tokio::test]
async fn test_quant_review_compute_rpc() {
    let (dispatcher, _) = setup_quant_env().await;

    let fills = vec![
        serde_json::json!({
            "fill_id": "f_buy_1",
            "order_id": "o_1",
            "symbol": "ETHUSDT",
            "direction": "buy",
            "price": 3000.0,
            "quantity": 2.0,
            "commission": 6.0,
            "timestamp": 1700000000,
            "exit_reason": null,
        }),
        serde_json::json!({
            "fill_id": "f_sell_1",
            "order_id": "o_2",
            "symbol": "ETHUSDT",
            "direction": "sell",
            "price": 3150.0,
            "quantity": 2.0,
            "commission": 6.3,
            "timestamp": 1700000600, // 600s duration -> 5-30m cohort
            "exit_reason": "take_profit",
        }),
    ];

    let req = ApiRequest {
        id: "review_compute".into(),
        method: METHOD_QUANT_REVIEW_COMPUTE.into(),
        params: serde_json::json!({
            "strategy_id": "eth_trend_v1",
            "symbol": "ETHUSDT",
            "fills": fills,
        }),
    };
    let resp = dispatcher.handle_request(req).await;
    assert!(resp.is_success(), "{:?}", resp.error);
    let report = resp.result.unwrap();

    assert_eq!(report["strategy_id"], "eth_trend_v1");
    assert_eq!(report["total_trades"], 1);
    assert_eq!(report["gross_pnl"], 300.0);
    assert_eq!(report["total_commission"], 12.3);
    assert_eq!(report["net_pnl"], 287.7);

    let cohorts = report["holding_cohorts"].as_array().unwrap();
    let cohort_5_30m = cohorts.iter().find(|c| c["label"] == "5-30m").unwrap();
    assert_eq!(cohort_5_30m["trade_count"], 1);
    assert_eq!(cohort_5_30m["net_pnl"], 287.7);

    let exits = report["exit_cohorts"].as_array().unwrap();
    assert_eq!(exits[0]["reason"], "take_profit");
}

#[tokio::test]
async fn test_quant_orderbook_get_rpc() {
    let (dispatcher, _) = setup_quant_env().await;

    // 1. Without ingested bars, Zero-Mock orderbook must reject honestly.
    let empty_req = ApiRequest {
        id: "orderbook_empty".into(),
        method: METHOD_QUANT_ORDERBOOK_GET.into(),
        params: serde_json::json!({
            "symbol": "BTC/USDT",
            "depth": 5,
        }),
    };
    let empty_resp = dispatcher.handle_request(empty_req).await;
    assert!(!empty_resp.is_success());

    // 2. Even with observed OHLCV bars, Zero-Mock forbids synthesizing fake L2 depth.
    let ingest_req = ApiRequest {
        id: "orderbook_ingest".into(),
        method: METHOD_QUANT_MARKET_BARS_INGEST.into(),
        params: serde_json::json!({
            "symbol": "BTC/USDT",
            "timeframe": "1m",
            "bars": [
                { "timestamp": 1700000000, "open": 64800.0, "high": 65200.0, "low": 64700.0, "close": 65000.0, "volume": 42.5 }
            ]
        }),
    };
    let ingest_resp = dispatcher.handle_request(ingest_req).await;
    assert!(ingest_resp.is_success(), "{:?}", ingest_resp.error);

    let req = ApiRequest {
        id: "orderbook_test".into(),
        method: METHOD_QUANT_ORDERBOOK_GET.into(),
        params: serde_json::json!({
            "symbol": "BTC/USDT",
            "depth": 5,
        }),
    };

    let resp = dispatcher.handle_request(req).await;
    assert!(!resp.is_success());
    let err = resp.error.unwrap_or_default();
    assert!(
        err.contains("Live L2 orderbook feed is not connected"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn test_quant_productized_closed_loop_and_pack_disable_isolation() {
    let (dispatcher, task_id) = setup_quant_env().await;

    // 1. Ingest real daily bars with explicit MarketCategory
    let bars: Vec<serde_json::Value> = (0..40)
        .map(|i| {
            let base = 500.0 + (i as f64) * 1.25;
            serde_json::json!({
                "timestamp": 1_700_000_000_i64 + (i as i64) * 86_400,
                "open": base - 0.5,
                "high": base + 2.0,
                "low": base - 1.5,
                "close": base,
                "volume": 100_000.0 + (i as f64) * 500.0
            })
        })
        .collect();

    let ingest_resp = dispatcher
        .handle_request(ApiRequest {
            id: "prod_ingest".into(),
            method: METHOD_QUANT_MARKET_BARS_INGEST.into(),
            params: serde_json::json!({
                "symbol": "SPY",
                "timeframe": "1d",
                "market": "USStock",
                "venue": "verified_test_feed",
                "bars": bars
            }),
        })
        .await;
    assert!(ingest_resp.is_success(), "{:?}", ingest_resp.error);

    // 2. QuantPack::execute_tool runs real zero-mock backtest & rejects missing symbols
    let quant_pack = dispatcher
        .workbenches
        .pack_registry
        .find_pack_for_tool("run_backtest")
        .expect("QuantPack must be registered when quant is installed");

    let missing_tool = quant_pack
        .execute_tool(
            "run_backtest",
            &serde_json::json!({ "symbol": "NO_SUCH_TICKER", "timeframe": "1d" }),
        )
        .await;
    assert!(
        missing_tool.is_err(),
        "QuantPack must never generate synthetic sine waves for missing symbols"
    );

    let live_tool = quant_pack
        .execute_tool(
            "run_backtest",
            &serde_json::json!({
                "strategy_id": "strat_prod_loop",
                "symbol": "SPY",
                "timeframe": "1d",
                "formula": "dual_ma(5, 15)"
            }),
        )
        .await
        .expect("QuantPack run_backtest should succeed against persisted SQLite bars");
    assert_eq!(live_tool["strategy_id"], "strat_prod_loop");
    assert!(live_tool["bar_count"].as_u64().unwrap() >= 40);

    // 3. Strategy registry + Evidence quality assessment
    let strat_resp = dispatcher
        .handle_request(ApiRequest {
            id: "prod_strats".into(),
            method: METHOD_QUANT_STRATEGIES_LIST.into(),
            params: serde_json::json!({}),
        })
        .await;
    assert!(strat_resp.is_success(), "{:?}", strat_resp.error);
    let strategies = strat_resp.result.unwrap()["strategies"]
        .as_array()
        .unwrap()
        .clone();
    assert!(strategies.iter().any(|s| s["id"] == "strat_prod_loop"));

    let evidence_resp = dispatcher
        .handle_request(ApiRequest {
            id: "prod_evidence".into(),
            method: METHOD_QUANT_EVIDENCE_ASSESS.into(),
            params: serde_json::json!({
                "symbol": "SPY",
                "market": "USStock",
                "required_metrics": ["close", "volume"],
                "observations": [
                    {
                        "evidence_id": "ev_close_1",
                        "metric": "close",
                        "source": "verified_test_feed",
                        "value": 548.75,
                        "as_of_ts": chrono::Utc::now().timestamp(),
                        "freshness_limit_secs": 3600,
                        "required": true
                    },
                    {
                        "evidence_id": "ev_vol_1",
                        "metric": "volume",
                        "source": "verified_test_feed",
                        "value": 119500.0,
                        "as_of_ts": chrono::Utc::now().timestamp(),
                        "freshness_limit_secs": 3600,
                        "required": true
                    }
                ]
            }),
        })
        .await;
    assert!(evidence_resp.is_success(), "{:?}", evidence_resp.error);
    assert_eq!(evidence_resp.result.unwrap()["coverage_ratio"], 1.0);

    // 4. Draft -> Approve with real cryptographic auto_issue permit & paper settlement
    let create_resp = dispatcher
        .handle_request(ApiRequest {
            id: "prod_order_create".into(),
            method: METHOD_QUANT_ORDERS_CREATE.into(),
            params: serde_json::json!({
                "strategy_id": "strat_prod_loop",
                "task_id": task_id,
                "orders": [{
                    "symbol": "SPY",
                    "direction": "buy",
                    "quantity": 10.0,
                    "target_weight": 0.1,
                    "limit_price": 510.0
                }]
            }),
        })
        .await;
    assert!(create_resp.is_success(), "{:?}", create_resp.error);
    let order_id = create_resp.result.unwrap()["created_orders"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let approve_resp = dispatcher
        .handle_request(ApiRequest {
            id: "prod_order_approve".into(),
            method: METHOD_QUANT_ORDERS_APPROVE.into(),
            params: serde_json::json!({
                "order_id": order_id,
                "permit_id": "auto_issue",
                "task_id": task_id,
                "execute_paper": true,
                "execution_price": 510.0
            }),
        })
        .await;
    assert!(approve_resp.is_success(), "{:?}", approve_resp.error);
    let approve_val = approve_resp.result.unwrap();
    assert_eq!(approve_val["status"], "filled");
    assert_ne!(approve_val["permit_id"], "auto_issue");

    let pos_resp = dispatcher
        .handle_request(ApiRequest {
            id: "prod_pos_open".into(),
            method: METHOD_QUANT_POSITIONS_LIST.into(),
            params: serde_json::json!({ "strategy_id": "strat_prod_loop" }),
        })
        .await;
    assert!(pos_resp.is_success(), "{:?}", pos_resp.error);
    let open_positions = pos_resp.result.unwrap()["positions"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(open_positions.len(), 1);
    assert_eq!(open_positions[0]["symbol"], "SPY");
    assert_eq!(open_positions[0]["quantity"], 10.0);

    // 5. Close position via Exit Immunity -> settles fill & clears position -> Review reads SQLite fills
    let close_resp = dispatcher
        .handle_request(ApiRequest {
            id: "prod_pos_close".into(),
            method: METHOD_QUANT_ORDERS_CLOSE_POSITION.into(),
            params: serde_json::json!({
                "strategy_id": "strat_prod_loop",
                "task_id": task_id,
                "symbol": "SPY",
                "direction": "long",
                "quantity": 10.0,
                "mark_price": 525.0,
                "permit_id": "auto_issue",
                "execute_paper": true
            }),
        })
        .await;
    assert!(close_resp.is_success(), "{:?}", close_resp.error);
    assert_eq!(
        close_resp.result.unwrap()["gate_outcome"]["status"],
        "BypassExitImmunity"
    );

    let pos_after = dispatcher
        .handle_request(ApiRequest {
            id: "prod_pos_closed".into(),
            method: METHOD_QUANT_POSITIONS_LIST.into(),
            params: serde_json::json!({ "strategy_id": "strat_prod_loop" }),
        })
        .await;
    assert!(pos_after.is_success(), "{:?}", pos_after.error);
    assert!(pos_after.result.unwrap()["positions"]
        .as_array()
        .unwrap()
        .is_empty());

    let review_resp = dispatcher
        .handle_request(ApiRequest {
            id: "prod_review_sqlite".into(),
            method: METHOD_QUANT_REVIEW_COMPUTE.into(),
            params: serde_json::json!({
                "strategy_id": "strat_prod_loop",
                "symbol": "SPY",
                "fills": []
            }),
        })
        .await;
    assert!(review_resp.is_success(), "{:?}", review_resp.error);
    let review = review_resp.result.unwrap();
    assert_eq!(review["total_trades"], 1);
    assert!(review["gross_pnl"].as_f64().unwrap() > 140.0);

    // 6. Disabling 'quant' immediately locks all quant RPCs and removes QuantPack
    let disable_resp = dispatcher
        .handle_request(ApiRequest {
            id: "prod_disable_quant".into(),
            method: "packages.disable".into(),
            params: serde_json::json!({ "component_id": "quant" }),
        })
        .await;
    assert!(disable_resp.is_success(), "{:?}", disable_resp.error);
    assert!(!dispatcher.workbenches.pack_registry.has_pack("quant"));

    let blocked_pos = dispatcher
        .handle_request(ApiRequest {
            id: "prod_pos_blocked".into(),
            method: METHOD_QUANT_POSITIONS_LIST.into(),
            params: serde_json::json!({}),
        })
        .await;
    assert!(!blocked_pos.is_success());
    assert!(blocked_pos.error.unwrap().contains("pack_disabled"));
}
