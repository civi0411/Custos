use super::super::*;
use std::sync::Arc;
use custos_core::TaskService;
use custos_runtime::session::SessionManager;
use custos_bridge::BridgeService;
use custos_core::CreateTask;

async fn setup_quant_env() -> (LocalApiDispatcher, String) {
    let store = Arc::new(custos_persistence::SqliteTaskStore::new_in_memory().unwrap());
    let service = Arc::new(TaskService::new(store.clone()));
    let task = service
        .execute_create(CreateTask {
            title: "Quant Pipeline Test Task".into(),
            metadata: None,
            contract: None,
        })
        .await
        .unwrap()
        .0;

    let sessions = Arc::new(SessionManager::new());
    let bridge = Arc::new(BridgeService::new(sessions.clone(), service.clone()));
    let outbox = Arc::new(store.outbox().clone());
    let dispatcher = LocalApiDispatcher::new(service, sessions, bridge)
        .with_sqlite_store(store)
        .with_outbox(outbox);

    (dispatcher, task.id)
}

#[tokio::test]
async fn test_quant_backtest_submit_and_tearsheet_get() {
    let (dispatcher, task_id) = setup_quant_env().await;

    let req = ApiRequest {
        id: "backtest_submit".into(),
        method: METHOD_QUANT_BACKTEST_SUBMIT.into(),
        params: serde_json::json!({
            "strategy_id": "strat_momentum_1",
            "task_id": task_id,
            "formula": "ts_rank(close / delay(close, 5), 20)",
            "annualized_sharpe": 1.85,
            "max_drawdown": 0.12,
            "total_return": 0.35,
            "trials_count": 25,
            "sample_length": 252,
            "volatility": 0.16,
            "win_rate": 0.58,
        }),
    };

    let resp = dispatcher.handle_request(req).await;
    assert!(resp.is_success(), "{:?}", resp.error);
    let sheet = resp.result.unwrap();

    assert_eq!(sheet["strategy_id"], "strat_momentum_1");
    assert_eq!(sheet["annualized_sharpe"], 1.85);
    assert!(sheet["deflated_sharpe"].as_f64().unwrap() > 0.0);
    assert!((sheet["calmar_ratio"].as_f64().unwrap() - (0.35 / 0.12)).abs() < 1e-4);

    // Retrieve tearsheet via GET endpoint
    let get_req = ApiRequest {
        id: "tearsheet_get".into(),
        method: METHOD_QUANT_TEARSHEET_GET.into(),
        params: serde_json::json!({
            "strategy_id": "strat_momentum_1",
        }),
    };

    let get_resp = dispatcher.handle_request(get_req).await;
    assert!(get_resp.is_success(), "{:?}", get_resp.error);
    let fetched = get_resp.result.unwrap();
    assert_eq!(fetched["strategy_id"], "strat_momentum_1");
    assert_eq!(fetched["annualized_sharpe"], 1.85);
    assert_eq!(fetched["deflated_sharpe"], sheet["deflated_sharpe"]);
}

#[tokio::test]
async fn test_quant_backtest_lookahead_bias_rejection() {
    let (dispatcher, task_id) = setup_quant_env().await;

    let req = ApiRequest {
        id: "backtest_submit_leak".into(),
        method: METHOD_QUANT_BACKTEST_SUBMIT.into(),
        params: serde_json::json!({
            "strategy_id": "strat_leaky",
            "task_id": task_id,
            "formula": "close.shift(-1) / close",
            "annualized_sharpe": 3.5,
            "max_drawdown": 0.02,
            "total_return": 1.5,
        }),
    };

    let resp = dispatcher.handle_request(req).await;
    assert!(!resp.is_success(), "Must reject formulas with lookahead bias");
    assert!(resp.error.unwrap().contains("Lookahead bias check failed"));
}

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
    let list_arr = list_resp.result.unwrap()["orders"].as_array().unwrap().clone();
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
    assert!(!outbox_items.is_empty(), "Approved order must be bridged to Outbox entries");
    let quant_item = outbox_items.iter().find(|item| item["action_type"] == "quant_order_execution").unwrap();
    assert_eq!(quant_item["permit_id"], "permit_risk_audit_passed_2026");
    assert_eq!(quant_item["status"], "approved");
}

#[tokio::test]
async fn test_quant_grid_evaluate_rpc() {
    let (dispatcher, _) = setup_quant_env().await;

    let req = ApiRequest {
        id: "grid_eval_test".into(),
        method: METHOD_QUANT_GRID_EVALUATE.into(),
        params: serde_json::json!({
            "symbol": "BTCUSDT",
            "lower_price": 60000.0,
            "upper_price": 70000.0,
            "grid_count": 5,
            "mode": "arithmetic",
            "total_capital": 50000.0,
            "current_price": 65000.0,
            "stop_loss_price": 55000.0,
            "take_profit_price": 75000.0,
        }),
    };

    let resp = dispatcher.handle_request(req).await;
    assert!(resp.is_success(), "{:?}", resp.error);
    let res = resp.result.unwrap();
    assert_eq!(res["symbol"], "BTCUSDT");
    assert_eq!(res["cells_count"], 4);
    assert_eq!(res["levels"].as_array().unwrap().len(), 5);
}

#[tokio::test]
async fn test_quant_market_bars_ingest_and_get() {
    let (dispatcher, _) = setup_quant_env().await;

    // Ingest bars
    let ingest_req = ApiRequest {
        id: "ingest_test".into(),
        method: METHOD_QUANT_MARKET_BARS_INGEST.into(),
        params: serde_json::json!({
            "symbol": "ETHUSDT",
            "timeframe": "1h",
            "bars": [
                { "timestamp": 1000, "open": 3000.0, "high": 3050.0, "low": 2980.0, "close": 3020.0, "volume": 100.0 },
                { "timestamp": 4600, "open": 3020.0, "high": 3080.0, "low": 3010.0, "close": 3070.0, "volume": 120.0 },
            ]
        }),
    };
    let ingest_resp = dispatcher.handle_request(ingest_req).await;
    assert!(ingest_resp.is_success(), "{:?}", ingest_resp.error);

    // Get bars
    let get_req = ApiRequest {
        id: "get_test".into(),
        method: METHOD_QUANT_MARKET_BARS_GET.into(),
        params: serde_json::json!({
            "symbol": "ETHUSDT",
            "timeframe": "1h",
            "limit": 10,
        }),
    };
    let get_resp = dispatcher.handle_request(get_req).await;
    assert!(get_resp.is_success(), "{:?}", get_resp.error);
    let data = get_resp.result.unwrap();
    assert_eq!(data["symbol"], "ETHUSDT");
    assert_eq!(data["count"], 2);
    let bars = data["bars"].as_array().unwrap();
    assert_eq!(bars.len(), 2);
    assert_eq!(bars[0]["close"], 3020.0);
}

#[tokio::test]
async fn test_pipeline_status_rpc() {
    let (dispatcher, _) = setup_quant_env().await;

    let req = ApiRequest {
        id: "pipe_status".into(),
        method: METHOD_PIPELINE_STATUS.into(),
        params: serde_json::json!({}),
    };
    let resp = dispatcher.handle_request(req).await;
    assert!(resp.is_success(), "{:?}", resp.error);
    let status = resp.result.unwrap();
    assert_eq!(status["status"], "active");
    assert_eq!(status["stages"]["bronze"]["status"], "ingesting");
    assert_eq!(status["stages"]["silver"]["status"], "verified");
    assert_eq!(status["stages"]["gold"]["status"], "point_in_time");
    assert_eq!(status["modalities"]["market"]["lookahead_prevention"], "closed_bar_invariant");
}

#[tokio::test]
async fn test_quant_indicators_compute_rpc() {
    let (dispatcher, _) = setup_quant_env().await;

    // Ingest 35 synthetic bars
    let mut bars = Vec::new();
    let mut price = 100.0;
    for i in 0..35 {
        price += if i % 2 == 0 { 2.0 } else { -1.0 };
        bars.push(serde_json::json!({
            "timestamp": i * 86400,
            "open": price - 0.5,
            "high": price + 1.0,
            "low": price - 1.0,
            "close": price,
            "volume": 1000.0,
        }));
    }

    let ingest_req = ApiRequest {
        id: "ingest_bars".into(),
        method: METHOD_QUANT_MARKET_BARS_INGEST.into(),
        params: serde_json::json!({
            "symbol": "SOLUSDT",
            "timeframe": "1d",
            "bars": bars,
        }),
    };
    let ingest_resp = dispatcher.handle_request(ingest_req).await;
    assert!(ingest_resp.is_success());

    // Compute indicators
    let compute_req = ApiRequest {
        id: "compute_ind".into(),
        method: METHOD_QUANT_INDICATORS_COMPUTE.into(),
        params: serde_json::json!({
            "symbol": "SOLUSDT",
            "timeframe": "1d",
            "limit": 50,
        }),
    };
    let compute_resp = dispatcher.handle_request(compute_req).await;
    assert!(compute_resp.is_success(), "{:?}", compute_resp.error);
    let result = compute_resp.result.unwrap();
    assert_eq!(result["symbol"], "SOLUSDT");
    assert_eq!(result["total_bars_evaluated"], 35);
    let indicators = &result["indicators"];
    assert!(indicators["sma_20"].as_f64().unwrap() > 0.0);
    assert!(indicators["rsi_14"].as_f64().unwrap() > 0.0);
    assert!(indicators["bollinger_bands_20_2"]["middle"].as_f64().unwrap() > 0.0);
    assert!(indicators["macd_12_26_9"]["macd"].is_number());
    assert!(indicators["atr_14"].as_f64().unwrap() > 0.0);
    assert!(indicators["vwap"].as_f64().unwrap() > 0.0);
}

#[tokio::test]
async fn test_pipeline_structure_detect_rpc() {
    let (dispatcher, _) = setup_quant_env().await;

    let csv_content = "BANNER HEADER: UNSTRUCTURED PATIENT REPORT\nPID,Test_Code,Result_Value,Is_Abnormal\nP001,GLUCOSE,5.4,false\nP002,CHOLESTEROL,6.2,true\nP003,TRIGLYCERIDE,1.8,false";

    let req = ApiRequest {
        id: "detect_csv".into(),
        method: METHOD_PIPELINE_STRUCTURE_DETECT.into(),
        params: serde_json::json!({
            "content": csv_content,
        }),
    };
    let resp = dispatcher.handle_request(req).await;
    assert!(resp.is_success(), "{:?}", resp.error);
    let detected = resp.result.unwrap();
    assert_eq!(detected["delimiter"], ",");
    assert_eq!(detected["header_row_index"], 1);
    assert_eq!(detected["headers"].as_array().unwrap().len(), 4);
    assert!(detected["quality_score"].as_f64().unwrap() > 0.9);
}

#[tokio::test]
async fn test_pipeline_run_rpc() {
    let (dispatcher, _) = setup_quant_env().await;

    let csv_content = "PID,Test_Code,Result_Value\nP001,GLUCOSE,5.4\nP002,CHOLESTEROL,6.2";

    let req = ApiRequest {
        id: "run_pipe".into(),
        method: METHOD_PIPELINE_RUN.into(),
        params: serde_json::json!({
            "content": csv_content,
            "modality": "Science",
            "salt": "vinmec_secret_salt_123",
        }),
    };
    let resp = dispatcher.handle_request(req).await;
    assert!(resp.is_success(), "{:?}", resp.error);
    let result = resp.result.unwrap();
    assert_eq!(result["status"], "success");
    assert_eq!(result["stage_reached"], "Gold");
    assert_eq!(result["modality"], "Science");
    assert_eq!(result["rows_ingested"], 2);
    assert_eq!(result["headers"].as_array().unwrap().len(), 3);
    assert!(result["anonymization_salt_applied"].as_bool().unwrap());
    assert_eq!(result["anonymized_columns"].as_array().unwrap()[0], "PID");
    assert!(result["content_hash"].as_str().unwrap().len() > 10);
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
    assert_eq!(breach_res["outcome"]["details"]["breach_type"], "MaxOrderNotionalExceeded");
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

    let req = ApiRequest {
        id: "orderbook_test".into(),
        method: METHOD_QUANT_ORDERBOOK_GET.into(),
        params: serde_json::json!({
            "symbol": "BTC/USDT",
            "depth": 5,
        }),
    };

    let resp = dispatcher.handle_request(req).await;
    assert!(resp.is_success(), "{:?}", resp.error);
    let ob = resp.result.unwrap();

    assert_eq!(ob["symbol"], "BTC/USDT");
    assert!(ob["mid_price"].as_f64().unwrap() > 0.0);
    assert!(ob["spread"].as_f64().unwrap() > 0.0);
    assert!(ob["microprice"].as_f64().unwrap() > 0.0);

    let asks = ob["asks"].as_array().unwrap();
    let bids = ob["bids"].as_array().unwrap();
    assert_eq!(asks.len(), 5);
    assert_eq!(bids.len(), 5);

    // Asks must be strictly greater than bids
    assert!(asks[0]["price"].as_f64().unwrap() > bids[0]["price"].as_f64().unwrap());
    // Asks must be monotonically increasing
    for i in 1..asks.len() {
        assert!(asks[i]["price"].as_f64().unwrap() >= asks[i - 1]["price"].as_f64().unwrap());
    }
    // Bids must be monotonically decreasing
    for i in 1..bids.len() {
        assert!(bids[i]["price"].as_f64().unwrap() <= bids[i - 1]["price"].as_f64().unwrap());
    }
}



