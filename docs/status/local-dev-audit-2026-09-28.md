# Local development audit — 2026-09-28

**Status:** source-backed audit of an uncommitted checkout after a major local restructure. **HEAD:** `205468e32c19395c3d907d898da3845aabfa9d82` on `dev`, 0 ahead/0 behind `origin/dev`. The dirty tree is part of the observation and is not a reproducible team baseline.

## Decision summary

The restructure materially improved the executable spine: CLI uses the Local API client, daemon and Local API share DTOs, Session state is store-backed, direct `advance -> Succeeded` is rejected, and process tests cover daemon restart. Rust format, compile, strict Clippy, contract tests, and focused E2E are green.

Custos is still **not trust-closed**. The completion API accepts client-supplied `VerificationClaim` objects and trusts their `passed` and `verifier_id` fields. Controlled effects execute in the E2E test process rather than through the daemon composition root. The experimental `custos-gateway` remains an unwired mock control plane. Freeze broad feature expansion until these boundaries and the Git baseline are closed.

## Measured snapshot

| Measurement | Result |
|---|---|
| Git | 53 modified, 162 deleted, 2,405 untracked; 2,620 file-level entries after the documentation restructuring |
| Tracked diff | 215 files, 9,354 insertions, 12,184 deletions |
| Cargo | 42 workspace packages; five additional non-member manifests |
| Nexus | 2,267 files; 10,207 Rust/Python AST symbols; 70,354 syntactic call references; 18,744 Custos chunks after the final 2026-09-28 refresh |
| Test inventory | 1,238 tests listed by the workspace |
| Goose naming guard | Pass; 574 compatibility/debt files reported |

Nexus is a discovery index. Its edges are syntactic and do not by themselves prove dynamic dispatch, authorization, persistence, TypeScript behavior, or production reachability.

## Verification performed

| Command | Result | Scope and limitation |
|---|---|---|
| `cargo metadata --offline --no-deps --format-version 1` | Pass | Resolves 42 workspace packages only. |
| `bash scripts/check_deps.sh` | Pass | Hard rules pass; four transitional edges remain. |
| `bash scripts/check_naming.sh` | Pass | Classification guard passes; naming migration is not complete. |
| `cargo fmt --all -- --check` | Pass | Rust workspace formatting only. |
| `cargo check --workspace --offline` | Pass | Compilation only. |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | Pass | Rust workspace/all targets under current features. |
| `cargo test -p custos-tests-contract --offline` | Pass, 2 tests | Fake provider and schema-presence contracts. |
| `cargo test -p custos-tests-e2e --offline` | Pass, 4 tests | Includes daemon process restart and an effect/proof fixture; limitations below. |
| `cargo test --workspace --offline` | Environment-limited | 610 tests passed before sandbox denied a loopback bind. The exact failing test passed outside the sandbox. A single unrestricted full-suite run was not performed. |
| UI, bot, OIDC | Not run | Dependencies are not installed and current Rust CI does not cover them. |

## Confirmed improvements

1. `custos-cli` now creates `ProcessTransport` and `LocalApiClient`; it no longer imports SQLite or constructs `TaskService`.
2. Daemon `api.rs` reuses request/response types from `custos-local-api`, removing the previous DTO fork.
3. `SessionManager::with_store` is injected from daemon bootstrap; the process E2E proves Session, journal, attachment, and Task rows survive daemon restart.
4. Both the API and Kernel reject direct advancement to `Succeeded`; completion routes through `execute_complete`.
5. Required evidence kinds are checked by `CompletionGate`, and missing/failed fixtures are rejected.
6. `DeterministicGate` now performs bounded `read_file`, `list_files`, and non-mutating `patch_preview` operations with path containment and receipts.
7. Dependency debt reduced to `bridge -> session`, `bridge -> persistence`, and CLI dependencies on MCP/providers.

## P0 findings

### P0.1 — The local restructure is not shareable

Most of the new tree remains untracked while the old layout appears as tracked deletions. A clean clone of `dev` does not reproduce this checkout. Do not use the current local test result as a merge or release claim until the migration is classified and reviewed on a feature branch.

### P0.2 — Proof closure trusts the caller

`CompleteTaskRequest` accepts serialized `VerificationClaim` values from the Local API client. `CompletionGate` searches for `claim.passed == true` and an accepted verifier-name string, but it does not load an immutable verifier-issued claim from trusted storage. It also does not currently enforce claim Task identity, stable criterion identity, contract revision, source/subject revision, verifier version, signature/provenance, or unresolved effect state.

A malicious or buggy client can construct a passing claim with a recognized `verifier_id`. The new E2E proves rejection of missing/partial claims, not rejection of forged claims. Until the daemon resolves trusted claim IDs from an evidence repository, this path is `Wired, not trust-closed` and must not be advertised as proof-carrying completion.

### P0.3 — Controlled effects are not daemon-composed

The controlled-effects E2E constructs `DeterministicGate` and `EvidencePipeline` inside the test process, executes tools there, then sends resulting claims to the daemon. Nexus finds `dispatch_for_task` callers only in security tests, the trait bridge, and this E2E; daemon source does not compose the gate or evidence pipeline. Persisted attempt-before-dispatch, crash-after-dispatch uncertainty, reconciliation, and OS sandbox adapters are therefore not yet proven on the production path.

### P0.4 — Two cognitive/gateway authorities remain

`custos-cognitive::CognitivePipeline` can select/call provider registry entries. `custos-gateway` reuses its routing policy but dispatches formatted mock strings and synthetic token/cost values. Nexus finds `route_and_execute` called only by the gateway's own test. No 9Router or Agentgateway integration exists in this package. Do not connect clients or daemon to it; decide whether to delete it, reduce it to ports, or replace it with named adapters.

## P1 findings

- Daemon manifests include workflow, cognitive, security, and MCP dependencies, but bootstrap only constructs store, Task service, Session manager, Bridge, and Local API dispatcher.
- CLI is thin at runtime but still directly depends on provider, agent, and MCP packages. Remove unused/legacy dependencies after supported command review.
- `custos-bridge` remains a core package depending on runtime Session and concrete persistence; move orchestration outward or introduce narrow ports.
- Context source still contains duplicate/dead compiler, compaction, and memory trees. Confirm exported modules before deletion; do not bulk-clean.
- Five manifests remain outside workspace validation: `custos-sdk`, `custos-engine`, `custos-acp-macros`, `custos-test`, and `custos-test-support`.
- Current GitHub workflows duplicate Rust CI responsibilities and use different cargo-deny action versions.
- Desktop remains Goose-derived ACP UI, VS Code is a manifest/readme shell, and the bot directly constructs an Anthropic client. None is yet a governed Custos Task client.
- The full workspace has strong test volume, but several adapter packages report zero tests and non-Rust products are outside the gate.

## Agent/documentation risks corrected in this pass

- Tool-specific rule files no longer define stale layouts or conflicting ownership.
- `AGENTS.md` now states instruction hierarchy, dirty-tree protection, and Nexus evidence limits.
- Owner workspace READMEs now contain active boundaries only; dated Vietnamese reports are explicitly historical and non-executable.
- Root architecture no longer promotes the Vietnamese hybrid document as a second canonical authority.
- `ARCH-FLOW-01` now separates target Session/Task, F1/F2/F3, routing, MCP, effect, evidence and recovery flows from current implementation status.
- The active work hub now has one decision queue and one contract/fixture/rollback handoff template; legacy “Canonical Baseline” pages are explicitly non-normative.
- Workspace counts and Nexus/naming measurements were refreshed.

## Required next sequence

1. **Baseline PR:** classify all untracked/deleted paths, remove duplicate CI, and prove the same green gates from a clean checkout.
2. **Trusted evidence contract:** Local API receives claim/evidence IDs, not authority-bearing client booleans; daemon loads current immutable assessments and validates Task, criterion, revision, verifier, and effect closure transactionally.
3. **Daemon-owned effect slice:** expose one read-only action command through daemon, persist intent/attempt before dispatch, execute through a real bounded adapter, store receipt/evidence, and test crash/uncertain/reconcile.
4. **Composition cleanup:** wire or remove unused daemon dependencies; remove legacy CLI dependencies; resolve Bridge placement.
5. **Gateway decision:** keep model routing, external-agent lifecycle, MCP capability transport, and effect authority as separate ports. Spike 9Router and Agentgateway only behind those ports with pinned conformance tests.
6. **Client gates:** connect Desktop/VS Code/bot through the versioned Local API or declared external-agent protocol; add non-Rust CI before product claims.

## Go/no-go rule

Proceed with narrow kernel/evidence/daemon work. Do not broaden UI, multi-agent, 9Router, Agentgateway, or self-setup claims until a clean checkout reproduces the baseline and forged evidence cannot close a Task.
