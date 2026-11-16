//! OpenTelemetry HTTP/JSON (OTLP) trace & log ingest adapter for Run Supervision.
//!
//! Normalizes external agent, harness, and SDK OTLP payloads into Custos
//! `NormalizedTelemetryBatch` without violating RunStatus sovereignty:
//! child span failures record `StepSpanStatus::Error` and `StepError`,
//! never forcibly marking an active root run as failed.

use custos_core::contracts::supervision::NormalizedTelemetryBatch;
use custos_domain::supervision::{
    compute_error_signature, LogSeverity, RunStepSpan, StepError, StepSpanKind, StepSpanStatus,
    SupervisionLogEntry, SupervisionRunRecord, SupervisionRunStatus,
};
use custos_domain::{new_id, DomainError, UsageMeasurement};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportTraceRequest {
    #[serde(default)]
    resource_spans: Vec<ResourceSpans>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResourceSpans {
    resource: Option<OtlpResource>,
    #[serde(default)]
    scope_spans: Vec<ScopeSpans>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScopeSpans {
    #[serde(default)]
    spans: Vec<OtlpSpan>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportLogsRequest {
    #[serde(default)]
    resource_logs: Vec<ResourceLogs>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResourceLogs {
    resource: Option<OtlpResource>,
    #[serde(default)]
    scope_logs: Vec<ScopeLogs>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScopeLogs {
    #[serde(default)]
    log_records: Vec<OtlpLogRecord>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpResource {
    #[serde(default)]
    attributes: Vec<KeyValue>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeyValue {
    key: String,
    value: Option<AnyValue>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnyValue {
    string_value: Option<String>,
    int_value: Option<Value>,
    double_value: Option<f64>,
    bool_value: Option<bool>,
    array_value: Option<ArrayValue>,
    kvlist_value: Option<KvListValue>,
}

#[derive(Debug, Default, Deserialize)]
struct ArrayValue {
    #[serde(default)]
    values: Vec<AnyValue>,
}

#[derive(Debug, Default, Deserialize)]
struct KvListValue {
    #[serde(default)]
    values: Vec<KeyValue>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpSpan {
    trace_id: Option<String>,
    span_id: Option<String>,
    parent_span_id: Option<String>,
    name: Option<String>,
    start_time_unix_nano: Option<Value>,
    end_time_unix_nano: Option<Value>,
    #[serde(default)]
    attributes: Vec<KeyValue>,
    #[serde(default)]
    events: Vec<OtlpEvent>,
    status: Option<OtlpStatus>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpEvent {
    name: Option<String>,
    time_unix_nano: Option<Value>,
    #[serde(default)]
    attributes: Vec<KeyValue>,
}

#[derive(Debug, Default, Deserialize)]
struct OtlpStatus {
    code: Option<i32>,
    message: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OtlpLogRecord {
    time_unix_nano: Option<Value>,
    severity_number: Option<i32>,
    severity_text: Option<String>,
    body: Option<AnyValue>,
    trace_id: Option<String>,
    span_id: Option<String>,
    #[serde(default)]
    attributes: Vec<KeyValue>,
}

/// Normalizes an OTLP JSON `ExportTraceServiceRequest` into spans, errors, and run summaries.
pub fn normalize_otlp_traces_json(bytes: &[u8]) -> Result<NormalizedTelemetryBatch, DomainError> {
    let req: ExportTraceRequest = serde_json::from_slice(bytes)
        .map_err(|e| DomainError::Validation(format!("Invalid OTLP traces JSON: {e}")))?;

    let mut runs_map: HashMap<String, SupervisionRunRecord> = HashMap::new();
    let mut spans_out = Vec::new();
    let mut errors_out = Vec::new();

    for resource_span in req.resource_spans {
        let res_attrs = attributes_to_map(
            resource_span
                .resource
                .as_ref()
                .map(|r| r.attributes.as_slice())
                .unwrap_or(&[]),
        );
        let agent = attr_str(&res_attrs, "service.name")
            .unwrap_or("external-agent")
            .to_string();
        let workbench_id = attr_str(&res_attrs, "custos.workbench_id").map(str::to_string);

        for scope in resource_span.scope_spans {
            for span in scope.spans {
                let attrs = attributes_to_map(&span.attributes);
                let span_id = span.span_id.clone().unwrap_or_else(|| new_id("step"));
                let run_id = attr_str(&attrs, "custos.run_id")
                    .map(str::to_string)
                    .or_else(|| span.trace_id.clone())
                    .unwrap_or_else(|| span_id.clone());
                let worker_run_id = attr_str(&attrs, "custos.worker_run_id").map(str::to_string);
                let is_root = span
                    .parent_span_id
                    .as_deref()
                    .is_none_or(|p| p.is_empty() || p.bytes().all(|b| b == b'0'));
                let parent_id = if is_root {
                    None
                } else {
                    span.parent_span_id.clone()
                };

                let started_at_ms = span
                    .start_time_unix_nano
                    .as_ref()
                    .map(nanos_value_to_millis)
                    .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
                let ended_at_ms = span
                    .end_time_unix_nano
                    .as_ref()
                    .map(nanos_value_to_millis)
                    .filter(|ts| *ts > 0);
                let duration_ms = ended_at_ms.map(|e| e.saturating_sub(started_at_ms));
                let status_code = span.status.as_ref().and_then(|s| s.code).unwrap_or(0);
                let status = if status_code == 2 {
                    StepSpanStatus::Error
                } else if ended_at_ms.is_some() {
                    StepSpanStatus::Success
                } else {
                    StepSpanStatus::Running
                };

                let inferred_kind = infer_span_kind(&attrs, is_root);
                let name = span
                    .name
                    .as_deref()
                    .filter(|n| !n.is_empty())
                    .unwrap_or(inferred_kind.as_str())
                    .to_string();

                let mut err_count_for_span = 0usize;
                for (idx, event) in span.events.iter().enumerate() {
                    if event.name.as_deref() != Some("exception") {
                        continue;
                    }
                    err_count_for_span += 1;
                    let ev_attrs = attributes_to_map(&event.attributes);
                    let err_type = attr_str(&ev_attrs, "exception.type")
                        .unwrap_or("Exception")
                        .to_string();
                    let err_msg = attr_str(&ev_attrs, "exception.message")
                        .unwrap_or("Unknown execution error")
                        .to_string();
                    let stack = attr_str(&ev_attrs, "exception.stacktrace").map(str::to_string);
                    let ts = event
                        .time_unix_nano
                        .as_ref()
                        .map(nanos_value_to_millis)
                        .unwrap_or_else(|| ended_at_ms.unwrap_or(started_at_ms));
                    let (_, sig_hash) = compute_error_signature(&err_type, &err_msg);
                    errors_out.push(StepError {
                        id: format!("{span_id}-exception-{idx}"),
                        run_id: run_id.clone(),
                        worker_run_id: worker_run_id.clone(),
                        span_id: Some(span_id.clone()),
                        error_type: err_type,
                        message: err_msg,
                        stack,
                        agent: agent.clone(),
                        signature_hash: sig_hash,
                        timestamp_ms: ts,
                    });
                }

                if status == StepSpanStatus::Error && err_count_for_span == 0 {
                    let msg = span
                        .status
                        .as_ref()
                        .and_then(|s| s.message.clone())
                        .unwrap_or_else(|| format!("Span \"{name}\" failed"));
                    let (_, sig_hash) = compute_error_signature("SpanError", &msg);
                    errors_out.push(StepError {
                        id: format!("{span_id}-status-error"),
                        run_id: run_id.clone(),
                        worker_run_id: worker_run_id.clone(),
                        span_id: Some(span_id.clone()),
                        error_type: "SpanError".into(),
                        message: msg,
                        stack: None,
                        agent: agent.clone(),
                        signature_hash: sig_hash,
                        timestamp_ms: ended_at_ms.unwrap_or(started_at_ms),
                    });
                }

                let tokens = extract_tokens(&attrs);
                let run_workbench = attr_str(&attrs, "custos.workbench_id")
                    .map(str::to_string)
                    .or_else(|| workbench_id.clone());

                let run_entry = runs_map.entry(run_id.clone()).or_insert_with(|| {
                    SupervisionRunRecord {
                        run_id: run_id.clone(),
                        workbench_id: run_workbench.clone(),
                        agent: agent.clone(),
                        status: SupervisionRunStatus::Running,
                        started_at_ms,
                        ended_at_ms: None,
                        duration_ms: None,
                        tokens: None,
                        cost_usd: None,
                        usage_measurement: UsageMeasurement::Unknown,
                    }
                });
                run_entry.started_at_ms = run_entry.started_at_ms.min(started_at_ms);
                if let Some(tok) = tokens {
                    let prev = run_entry.tokens.unwrap_or(0);
                    let total = prev.saturating_add(tok);
                    run_entry.tokens = Some(total);
                    run_entry.usage_measurement = UsageMeasurement::Known(total);
                }
                if is_root {
                    run_entry.ended_at_ms = ended_at_ms;
                    run_entry.duration_ms = duration_ms;
                    run_entry.status = match status {
                        StepSpanStatus::Error => SupervisionRunStatus::Error,
                        StepSpanStatus::Success => SupervisionRunStatus::Success,
                        StepSpanStatus::Running => SupervisionRunStatus::Running,
                    };
                }

                spans_out.push(RunStepSpan {
                    id: span_id,
                    run_id,
                    worker_run_id,
                    parent_id,
                    kind: inferred_kind,
                    name,
                    status,
                    started_at_ms,
                    ended_at_ms,
                    duration_ms,
                    metadata: Value::Object(attrs),
                });
            }
        }
    }

    Ok(NormalizedTelemetryBatch {
        runs: runs_map.into_values().collect(),
        spans: spans_out,
        errors: errors_out,
        logs: Vec::new(),
    })
}

/// Normalizes an OTLP JSON `ExportLogsServiceRequest` into structured logs and run stubs.
pub fn normalize_otlp_logs_json(bytes: &[u8]) -> Result<NormalizedTelemetryBatch, DomainError> {
    let req: ExportLogsRequest = serde_json::from_slice(bytes)
        .map_err(|e| DomainError::Validation(format!("Invalid OTLP logs JSON: {e}")))?;

    let mut logs_out = Vec::new();
    let mut runs_map: HashMap<String, SupervisionRunRecord> = HashMap::new();

    for resource_log in req.resource_logs {
        let res_attrs = attributes_to_map(
            resource_log
                .resource
                .as_ref()
                .map(|r| r.attributes.as_slice())
                .unwrap_or(&[]),
        );
        let agent = attr_str(&res_attrs, "service.name")
            .unwrap_or("external-agent")
            .to_string();
        let workbench_id = attr_str(&res_attrs, "custos.workbench_id").map(str::to_string);

        for scope in resource_log.scope_logs {
            for record in scope.log_records {
                let attrs = attributes_to_map(&record.attributes);
                let run_id = attr_str(&attrs, "custos.run_id")
                    .map(str::to_string)
                    .or_else(|| record.trace_id.clone())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "default-session".into());
                let worker_run_id = attr_str(&attrs, "custos.worker_run_id").map(str::to_string);
                let timestamp_ms = record
                    .time_unix_nano
                    .as_ref()
                    .map(nanos_value_to_millis)
                    .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
                let level = record
                    .severity_text
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .map(LogSeverity::parse)
                    .unwrap_or_else(|| {
                        LogSeverity::from_otlp_severity_number(record.severity_number.unwrap_or(9))
                    });
                let message = record
                    .body
                    .as_ref()
                    .map(decode_any_value_to_string)
                    .unwrap_or_default();

                runs_map.entry(run_id.clone()).or_insert_with(|| {
                    SupervisionRunRecord {
                        run_id: run_id.clone(),
                        workbench_id: workbench_id.clone(),
                        agent: agent.clone(),
                        status: SupervisionRunStatus::Running,
                        started_at_ms: timestamp_ms,
                        ended_at_ms: None,
                        duration_ms: None,
                        tokens: None,
                        cost_usd: None,
                        usage_measurement: UsageMeasurement::Unknown,
                    }
                });

                logs_out.push(SupervisionLogEntry {
                    id: new_id("slog"),
                    run_id,
                    worker_run_id,
                    span_id: record.span_id.filter(|s| !s.is_empty()),
                    timestamp_ms,
                    level,
                    message,
                    metadata: Value::Object(attrs),
                });
            }
        }
    }

    Ok(NormalizedTelemetryBatch {
        runs: runs_map.into_values().collect(),
        spans: Vec::new(),
        errors: Vec::new(),
        logs: logs_out,
    })
}

fn infer_span_kind(attrs: &Map<String, Value>, is_root: bool) -> StepSpanKind {
    if let Some(declared) = attr_str(attrs, "custos.span.kind") {
        return StepSpanKind::parse(declared);
    }
    if attrs.contains_key("gen_ai.request.model") || attrs.contains_key("llm.request.type") {
        return StepSpanKind::Llm;
    }
    if attrs.contains_key("tool.name") {
        return StepSpanKind::Tool;
    }
    if is_root {
        StepSpanKind::Agent
    } else {
        StepSpanKind::Tool
    }
}

fn extract_tokens(attrs: &Map<String, Value>) -> Option<u64> {
    let total = attr_u64(attrs, "gen_ai.usage.total_tokens")
        .or_else(|| attr_u64(attrs, "llm.usage.total_tokens"));
    if total.is_some() {
        return total;
    }
    let input = attr_u64(attrs, "gen_ai.usage.input_tokens")
        .or_else(|| attr_u64(attrs, "llm.usage.prompt_tokens"));
    let output = attr_u64(attrs, "gen_ai.usage.output_tokens")
        .or_else(|| attr_u64(attrs, "llm.usage.completion_tokens"));
    match (input, output) {
        (Some(i), Some(o)) => Some(i.saturating_add(o)),
        (Some(i), None) => Some(i),
        (None, Some(o)) => Some(o),
        (None, None) => None,
    }
}

fn attributes_to_map(attrs: &[KeyValue]) -> Map<String, Value> {
    let mut map = Map::new();
    for kv in attrs {
        if let Some(ref val) = kv.value {
            map.insert(kv.key.clone(), decode_any_value(val));
        }
    }
    map
}

fn decode_any_value(v: &AnyValue) -> Value {
    if let Some(ref s) = v.string_value {
        return Value::String(s.clone());
    }
    if let Some(b) = v.bool_value {
        return Value::Bool(b);
    }
    if let Some(d) = v.double_value {
        return serde_json::json!(d);
    }
    if let Some(ref iv) = v.int_value {
        if let Some(n) = iv.as_i64() {
            return serde_json::json!(n);
        }
        if let Some(s) = iv.as_str().and_then(|raw| raw.parse::<i64>().ok()) {
            return serde_json::json!(s);
        }
        return iv.clone();
    }
    if let Some(ref arr) = v.array_value {
        return Value::Array(arr.values.iter().map(decode_any_value).collect());
    }
    if let Some(ref kv) = v.kvlist_value {
        return Value::Object(attributes_to_map(&kv.values));
    }
    Value::Null
}

fn decode_any_value_to_string(v: &AnyValue) -> String {
    match decode_any_value(v) {
        Value::String(s) => s,
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn attr_str<'a>(map: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    map.get(key).and_then(|v| v.as_str()).filter(|s| !s.is_empty())
}

fn attr_u64(map: &Map<String, Value>, key: &str) -> Option<u64> {
    map.get(key).and_then(|v| {
        v.as_u64()
            .or_else(|| v.as_i64().filter(|n| *n >= 0).map(|n| n as u64))
            .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
    })
}

fn nanos_value_to_millis(val: &Value) -> i64 {
    let nanos = val
        .as_u64()
        .or_else(|| val.as_i64().filter(|n| *n >= 0).map(|n| n as u64))
        .or_else(|| val.as_str().and_then(|s| s.parse::<u64>().ok()))
        .unwrap_or(0);
    (nanos / 1_000_000) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_child_error_without_failing_active_root_run() {
        let payload = br#"{
          "resourceSpans": [{
            "resource": { "attributes": [
              { "key": "service.name", "value": { "stringValue": "external-harness" } },
              { "key": "custos.workbench_id", "value": { "stringValue": "coding" } }
            ]},
            "scopeSpans": [{
              "spans": [{
                "traceId": "run_abc",
                "spanId": "sp_child",
                "parentSpanId": "sp_root",
                "name": "grep_search",
                "startTimeUnixNano": "1700000000000000000",
                "endTimeUnixNano": "1700000001000000000",
                "status": { "code": 2, "message": "ripgrep exit 2" },
                "events": [{
                  "name": "exception",
                  "attributes": [
                    { "key": "exception.type", "value": { "stringValue": "ToolError" } },
                    { "key": "exception.message", "value": { "stringValue": "Pattern syntax invalid" } }
                  ]
                }]
              }]
            }]
          }]
        }"#;

        let batch = normalize_otlp_traces_json(payload).unwrap();
        assert_eq!(batch.spans.len(), 1);
        assert_eq!(batch.spans[0].status, StepSpanStatus::Error);
        assert_eq!(batch.errors.len(), 1);
        assert_eq!(batch.runs.len(), 1);
        // Active root run status is NOT forced to Error merely because a child tool span failed.
        assert_eq!(batch.runs[0].status, SupervisionRunStatus::Running);
        assert_eq!(batch.runs[0].workbench_id.as_deref(), Some("coding"));
    }

    #[test]
    fn normalizes_otlp_logs_payload() {
        let payload = br#"{
          "resourceLogs": [{
            "resource": { "attributes": [{ "key": "service.name", "value": { "stringValue": "research-worker" } }]},
            "scopeLogs": [{
              "logRecords": [{
                "timeUnixNano": "1700000005000000000",
                "severityNumber": 17,
                "body": { "stringValue": "Rate limit backoff triggered" },
                "traceId": "run_xyz",
                "spanId": "sp_1"
              }]
            }]
          }]
        }"#;

        let batch = normalize_otlp_logs_json(payload).unwrap();
        assert_eq!(batch.logs.len(), 1);
        assert_eq!(batch.logs[0].level, LogSeverity::Error);
        assert_eq!(batch.logs[0].run_id, "run_xyz");
    }
}

