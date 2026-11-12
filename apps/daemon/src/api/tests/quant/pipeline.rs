use super::fixtures::setup_quant_env;
use crate::api::*;

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
    assert_eq!(
        status["modalities"]["market"]["lookahead_prevention"],
        "closed_bar_invariant"
    );
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
            "salt": "custos_secret_salt_123",
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
async fn test_quant_pack_dynamic_admission_after_disable_and_enable() {
    let (dispatcher, task_id) = setup_quant_env().await;

    // 1. Initial execution call succeeds because quant is enabled
    let backtest_req = ApiRequest {
        id: "backtest_initial".into(),
        method: METHOD_QUANT_BACKTEST_SUBMIT.into(),
        params: serde_json::json!({
            "strategy_id": "strat_mom",
            "task_id": task_id,
            "formula": "ts_rank(close, 10)",
        }),
    };
    let resp1 = dispatcher.handle_request(backtest_req.clone()).await;
    assert!(resp1.is_success(), "{:?}", resp1.error);

    // 2. Disable quant pack via packages.disable RPC
    let disable_req = ApiRequest {
        id: "disable_quant".into(),
        method: "packages.disable".into(),
        params: serde_json::json!({ "component_id": "quant" }),
    };
    let disable_resp = dispatcher.handle_request(disable_req).await;
    assert!(disable_resp.is_success(), "{:?}", disable_resp.error);
    assert!(!dispatcher.has_quant(), "Disabling quant must release in-memory quant repo");

    // 3. Subsequent execution call must be dynamically rejected without restart
    let resp2 = dispatcher.handle_request(backtest_req.clone()).await;
    assert!(!resp2.is_success());
    assert!(resp2.error.unwrap().contains("pack_disabled"));

    // 4. Re-enable quant pack via packages.enable RPC
    let enable_req = ApiRequest {
        id: "enable_quant".into(),
        method: "packages.enable".into(),
        params: serde_json::json!({ "component_id": "quant" }),
    };
    let enable_resp = dispatcher.handle_request(enable_req).await;
    assert!(enable_resp.is_success(), "{:?}", enable_resp.error);
    assert!(dispatcher.has_quant(), "Enabling quant must lazily reattach quant repo");

    // 5. Execution call succeeds again
    let resp3 = dispatcher.handle_request(backtest_req).await;
    assert!(resp3.is_success(), "{:?}", resp3.error);
}
