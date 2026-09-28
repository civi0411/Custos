# PR-00: execution and recovery gates

**Status:** proposed release preparation, 2026-09-27. Numeric targets are hypotheses to measure, not achieved service levels.

## Ownership disagreement protocol

Truong drafts an ADR that quotes the conflicting paths in `AGENTS.md` and `dev_docs/MODULE_OWNERSHIP.md`, names affected files, proposes one writer and reviewers, and gives testable safety/data invariants. Vi and Vinh submit concrete objections within one working day. Shared domain, security, persistence, public DTO or effect semantics require all three maintainers' recorded approval. Any maintainer may veto with a failing scenario and proposed acceptance test.

If still 2–1 after 48 hours, do not merge the shared-contract PR. Hold a 30-minute decision session, narrow the change or run a one-day fixture spike. Keep the existing contract while disagreement remains; independent PRs may proceed. The product lead may prioritize scope, but cannot silently waive a documented safety/data objection. An accepted ADR records date, code SHA, decision, alternatives, owner/reviewers, tests and how governance docs are reconciled. This protocol is a proposal until the maintainers accept it.

## Session binding migration proposal

The current runner loads migrations `0001` through `0008`. Reserve `0009_session_binding.sql` only if no intervening PR occupies that number. Use an additive table and dual-read path before rewriting legacy status. A new `PromoteSession` transaction creates Task, revision, binding and event with command-id uniqueness. Backfill only rows whose legacy `Promoted {task_id}` references a valid Task; quarantine malformed or orphan rows. Current `SessionRepository::get_session` defaults an unparseable status to `Active`; remove that silent fallback before enabling new writes.

Before migration, freeze the writer and take a WAL-consistent SQLite backup plus a manifest of referenced content-addressed blobs, with checksums and binary/schema version. Test restoration on a copy. If no new writes happened, disabling the feature and restoring the backup may be safe. Once new bindings exist, prefer roll-forward with a dual-read binary; never drop the binding table as a generic “down migration.” Downgrade only after an audited export/reconciliation proves the old binary can represent every new write.

Fixture matrix: schema `0008` and `0009`; Active/Paused/Closed/Promoted; malformed status; orphan Task; duplicate command ID; crash between Task/binding/event; reopen with old and new binary; backup/restore and recovery scan. Pin a scrubbed legacy DB fixture hash.

## Activation and rollback for each PR

Every PR records activation path, kill switch if behavior or side effects change, old-binary compatibility, data written while enabled, rollback versus roll-forward, backup/restore proof, owner on call, telemetry and failure drill. A docs-only or additive dormant-schema PR need not invent a runtime flag. Proposed flags for behavior are `session_persist_v1`, `contextpack_v1`, `bounded_runtime_v1`, `f1_daemon_v1`, `effect_ledger_v1`, `effect_dispatch_v1`, `evidence_gate_v1`; names are not implemented merely by listing them here.

The effect kill switch stops **new dispatch**. Attempts already sent but without a receipt remain Uncertain for reconciliation; no switch can undo an external effect. On suspected data corruption, freeze writers, take a consistent backup and CAS manifest, inspect a copy, recover into a separate instance, reconcile by idempotency keys and record affected Task/Action/Session IDs before reopening writes.

## F1 acceptance

Pin fixture repository revision and dirty-state snapshot. Each factual citation carries claim ID, path, exact byte/line span and source content hash. Verifier rereads the exact bytes from the snapshot. An initial quality target is at least 80% of atomic factual claims with exact-span support; remaining statements must be explicitly labeled inference or uncertainty, never silently Pass. Inject forged path/hash/span and unsupported causal claim; they must Fail or remain Unknown. Restart must reload answer, route/event ledger and source references. Stale source must be detected before reusing a citation.

## Performance measurement budget

Measure on a documented reference machine/OS/fixture with warm and cold cases, 30 runs, p50 and p95. Isolate local overhead from provider/network/model latency. Failure of a target triggers profiling and an explicit threshold review, not a false Verified label.

| Metric | Initial target | Measurement boundary |
|---|---:|---|
| Daemon ready after warm binary start | p50 < 500 ms | Small DB, no model download |
| Context compilation | p50 < 200 ms | Warm index, small fixture repo |
| Local gateway policy/dispatch overhead | p50 < 50 ms | Fake executor; excludes tool/provider time |
| Idle daemon RSS | < 100 MB | No local model loaded |
| F1 fake-provider end-to-end | p50 < 5 s | Real daemon process, no network |

## Exit checklist

- [ ] Cargo member/manifests and dirty checkout are pinned; current tests and commands logged.
- [ ] Ownership ADR is accepted, or shared-contract PRs remain blocked.
- [ ] C-01–C-04 have valid/invalid fixtures and producer/consumer reviewers.
- [ ] Session migration has a legacy fixture, dual-read plan and backup/restore drill.
- [ ] F1 citation rubric and performance measurement machine are recorded.
- [ ] Every PR has activation/recovery notes; effect dispatch remains off until G2/F2 gates.
