use super::super::*;

#[test]
fn antigravity_summary_keeps_four_independent_limits() {
    let parsed = parse_account_quota(
        "antigravity-summary",
        &serde_json::json!({"groups":[
        {"displayName":"Gemini Models","buckets":[
            {"bucketId":"gemini-5h","remainingFraction":0.9,"resetTime":"five-hour-reset"},
            {"bucketId":"gemini-weekly","remainingFraction":0.4,"resetTime":"weekly-reset"}]},
        {"displayName":"Claude and GPT Models","buckets":[
            {"bucketId":"3p-5h","remainingFraction":0.7},
            {"bucketId":"3p-weekly","remainingFraction":0.2}]}]}),
    );
    assert_eq!(parsed["source"], "provider");
    assert_eq!(parsed["windows"].as_array().map(Vec::len), Some(4));
    assert_eq!(parsed["windows"][0]["model_group"], "gemini");
    assert_eq!(parsed["windows"][1]["window_kind"], "weekly");
    assert_eq!(parsed["windows"][2]["model_group"], "claude_gpt");
    assert_eq!(parsed["windows"][2]["window_kind"], "five_hour");
    assert_eq!(parsed["windows"][3]["used_percent"], 80.0);
    assert!(parsed["windows"][3]["resets_at"].is_null());
    let partial = parse_account_quota(
        "antigravity-summary",
        &serde_json::json!({"groups":[{"buckets":[
        {"bucketId":"gemini-5h","remainingFraction":0},
        {"bucketId":"gemini-weekly"}]}]}),
    );
    assert_eq!(partial["windows"].as_array().map(Vec::len), Some(1));
    assert_eq!(partial["windows"][0]["used_percent"], 100.0);
}

#[test]
fn non_standard_openai_window_is_not_relabeled_as_five_hours() {
    let parsed = parse_account_quota(
        "openai",
        &serde_json::json!({"rate_limit":{"primary_window":{
        "used_percent":10,"limit_window_seconds":86400}}}),
    );
    assert_eq!(parsed["windows"][0]["window_kind"], "other");
}

#[test]
fn missing_quota_is_unknown_and_windows_remain_distinct() {
    let empty = parse_account_quota("openai", &serde_json::json!({"rate_limit":{}}));
    assert_eq!(empty["status"], "unavailable");
    let parsed = parse_account_quota(
        "openai",
        &serde_json::json!({"rate_limit":{"primary_window":{"used_percent":0,"reset_at":123},"secondary_window":{"used_percent":83}}}),
    );
    assert_eq!(parsed["windows"].as_array().unwrap().len(), 2);
    assert_eq!(parsed["windows"][0]["used_percent"], 0.0);
    assert!(parsed["windows"][1]["resets_at"].is_null());
}

#[test]
fn provider_quota_units_are_normalized_without_inventing_limits() {
    let google = parse_account_quota(
        "google-live",
        &serde_json::json!({"buckets":[{"modelId":"gemini-flash","remainingFraction":0.25,"resetTime":"2026-10-12T00:00:00Z"},{"modelId":"unknown"}]}),
    );
    assert_eq!(google["windows"].as_array().unwrap().len(), 1);
    assert_eq!(google["windows"][0]["used_percent"], 75.0);
    assert_eq!(google["source"], "provider");
    let claude = parse_account_quota(
        "anthropic",
        &serde_json::json!({"five_hour":{"utilization":50},"seven_day":{"utilization":999}}),
    );
    assert_eq!(claude["windows"].as_array().unwrap().len(), 1);
    let copilot = parse_account_quota(
        "copilot",
        &serde_json::json!({"quota_snapshots":{"premium_interactions":{"percent_remaining":20}}}),
    );
    assert_eq!(copilot["windows"][0]["used_percent"], 80.0);
}
