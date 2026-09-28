# Implementation status: evidence rules

Current local audit: [2026-09-28 local development audit](local-dev-audit-2026-09-28.md). The [2026-09-27 audit](local-dev-audit-2026-09-27.md) is preserved as the preceding snapshot.

**Status:** current method, not a claim that Custos is fully wired. Earlier reports in this directory are immutable dated snapshots. The active codebase has substantial uncommitted changes; a status result without a commit SHA **and** dirty-state description cannot be transferred to another checkout.

## Record format

```text
Capability / use case:
Date, branch, commit SHA, dirty-state summary:
Status: Designed | Implemented | Wired | Verified | Degraded | Unknown
Entrypoint and call path:
Evidence: exact command, test name, exit code, log/artifact hash:
Failure paths exercised:
Limitations and untested claims:
Owner, independent reviewer, next review:
```

The status claim is scoped to the path tested. `cargo check --workspace` establishes compilation of workspace members, not daemon composition, migration correctness or tool-effect safety. A table existing in SQLite does not establish that the daemon writes it. A type named `ExecutionReceipt` does not establish that an external effect has been safely executed and reconciled.

## Required vertical evidence

| Flow | Minimum proof before `Verified` |
|---|---|
| F1 read-only explain | Request enters the daemon process, real ContextPack reaches the provider port, exact source-span/hash citation check, restart reloads session/outcome, unsupported claims do not Pass. |
| F2 bug fix | Typed ActionIntent, policy/permit, real executor, durable attempt/receipt, crash-after-dispatch uncertainty and reconciliation, artifact and criterion evidence. |
| F3 research to code | Source-versioned research claims, support/contradiction assessment, handoff contract, implemented patch, independent checks, limitations in outcome. |

Use the [implementation blueprint](../development/implementation-blueprint.md) for current build order and the [PR-00 gates](../development/pr-00-gates.md) for baseline commands and fixtures. The older [feature state](feature-state.md), [source path map](source-path-map.md) and [inventory](codebase-inventory.md) reflect a different checkout/layout and are not the active status board.

The [architecture gap matrix](architecture-gap-matrix.md) is the current 2026-09-28 boundary summary. It must be refreshed against a pinned clean checkout before a PR or release claim.
