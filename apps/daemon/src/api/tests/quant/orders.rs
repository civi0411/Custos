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

    // 2. Ingest observed market bars, then build orderbook around latest close.
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
    assert!(resp.is_success(), "{:?}", resp.error);
    let ob = resp.result.unwrap();

    assert_eq!(ob["symbol"], "BTC/USDT");
    assert!((ob["mid_price"].as_f64().unwrap() - 65000.0).abs() < 200.0);
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
