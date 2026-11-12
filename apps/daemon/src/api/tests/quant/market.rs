use super::fixtures::setup_quant_env;
use crate::api::*;

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
    assert!(
        indicators["bollinger_bands_20_2"]["middle"]
            .as_f64()
            .unwrap()
            > 0.0
    );
    assert!(indicators["macd_12_26_9"]["macd"].is_number());
    assert!(indicators["atr_14"].as_f64().unwrap() > 0.0);
    assert!(indicators["vwap"].as_f64().unwrap() > 0.0);
}
