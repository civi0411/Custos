# Custos work item and handoff template

Use this template for an issue, scoped PR plan or owner handoff. Keep one independently reviewable outcome per item.

```text
Title:
Owner:
Required reviewers:
Target branch / baseline SHA / dirty-state note:

Outcome:
User or system value:
In scope:
Out of scope:

Affected contract / ADR / API version:
Canonical writer and storage boundary:
Producer and consumers:
Migration or compatibility requirement:

Success fixture:
Failure, denial and stale-input fixtures:
Crash / retry / cancellation fixture:
Exact verification commands:

Activation path or feature flag:
Rollback or roll-forward:
Telemetry and sensitive-data rules:

Changed files/packages:
Evidence attached:
Known limitations:
Follow-up decision or work item:
```

## Review checklist

- No client, sidecar, model or agent writes canonical Task state or Custos SQLite directly.
- No model confidence, gateway telemetry or caller-authored claim grants authority or completion.
- State identity, revision, idempotency and retry semantics are explicit.
- A timeout after a possible external effect is handled as uncertainty, not automatic failure/retry.
- Status claims distinguish `Implemented`, `Wired`, `Verified` and `Trust-closed`.
- New dependencies, new crates and shared-type renames have the approvals required by `AGENTS.md`.
- Documentation, contract fixture and process-level evidence are updated with the code change.
