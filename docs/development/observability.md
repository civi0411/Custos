# Observability and Metrics

> **Status:** Canonical Baseline v4.0  
> **Source:** Part VI (§34) Canonical Specification

The Custos observability system is engineered on top of the **OpenTelemetry** standard, providing comprehensive visibility into local operational internals while strictly preserving user privacy.

---

## 1. Trace Span Hierarchy

All operations within a Task are structured as an explicit span tree:

```text
Workspace
└── Task Trace (task_id: "tsk_01J8N6...")
    ├── Step Spans (step: 1, role: "engineering.explorer")
    │   ├── Context Compilation Span (scoring & token ranking)
    │   ├── Provider Call Span (model: "codex", streaming)
    │   └── Decision Case Span (RDC request/response)
    ├── Step Spans (step: 2, role: "engineering.patcher")
    │   ├── Tool Execution Span (tool: "apply_patch", sandbox)
    │   └── Verifier Span (tool: "cargo_test", receipt_id)
    └── Human Approval Span (waiting for user exact-payload approval)
```

---

## 2. Core Metrics

- **`custos_task_total`:** Total number of tasks categorized by terminal state (`Success`, `Failed`, `Cancelled`).
- **`custos_step_duration_seconds`:** Execution latency per step broken down by worker role.
- **`custos_tokens_consumed_total`:** Input and output token consumption categorized by model provider.
- **`custos_judgment_cost_ratio`:** Ratio of execution cost between System One (heuristic judgment) and System Two (deep LLM reasoning).
- **`custos_human_interruptions_total`:** Count of human interventions and approval interruptions required.
- **`custos_verifier_pass_ratio`:** Percentage of task attempts passing automated verification on the first attempt.

---

## 3. Structured Logging and Redaction

- Standard Rust `tracing` infrastructure exporting to JSON Lines (`.jsonl`).
- **Pre-Log Redaction:** Automated redacting of secrets, auth tokens, and sensitive credentials prior to disk persistence.
- Raw prompts are disabled by default; developers can enable them for local debugging using `CUSTOS_LOG_PROMPTS=1`.
