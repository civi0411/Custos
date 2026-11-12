use super::fixtures::setup_quant_env;
use crate::api::*;

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
    assert!(
        !resp.is_success(),
        "Must reject formulas with lookahead bias"
    );
    assert!(resp.error.unwrap().contains("Lookahead bias check failed"));
}

#[tokio::test]
async fn test_quant_pbo_compute_rpc() {
    let (dispatcher, _) = setup_quant_env().await;

    let score_matrix = vec![
        vec![1.5, 2.0, 1.8, 1.2, 0.9, 1.4],
        vec![2.5, 2.2, 2.9, -1.0, -0.5, -0.8],
        vec![0.5, 0.6, 0.4, 0.5, 0.7, 0.6],
        vec![1.2, 1.1, 1.3, 1.4, 1.2, 1.3],
    ];

    let req = ApiRequest {
        id: "pbo_test".into(),
        method: METHOD_QUANT_PBO_COMPUTE.into(),
        params: serde_json::json!({
            "score_matrix": score_matrix,
            "max_combinations": 500,
        }),
    };

    let resp = dispatcher.handle_request(req).await;
    assert!(resp.is_success(), "{:?}", resp.error);
    let res = resp.result.unwrap();

    assert_eq!(res["available"], true);
    assert_eq!(res["samples"], 20);
    let prob = res["probability"].as_f64().unwrap();
    assert!((0.0..=1.0).contains(&prob));
    assert!(!res["logits"].as_array().unwrap().is_empty());
}
