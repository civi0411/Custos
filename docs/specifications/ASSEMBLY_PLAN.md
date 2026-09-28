# Custos New Assembly Plan

> **Historical proposal:** This assembly plan predates the current workspace layout and [implementation blueprint](../development/implementation-blueprint.md). Keep it for design provenance, not active PR sequencing.

## 1. Construction Strategy

Treat the new system like a decorated dry apricot tree: the trunk is small and load-bearing; branches attach through explicit joints; flowers are replaceable capabilities. Build and load-test the trunk before adding branches.

- Trunk: Domain, Kernel, persistence, workflow machine, Authority, Gateway.
- Primary branches: Cognitive, Context, Evidence, provider and protocol ports.
- Secondary branches: Engineering, Research, Assistant packs.
- Flowers: model adapters, MCP servers, tools, UI features, optional multi-workers.

## 2. Delivery Gates

### Gate 0: Repository Contract

- Freeze package names, dependency directions, provenance format, error taxonomy, IDs, and event envelope.
- Create an independent Cargo workspace only after dependency validation.
- Add CI for format, clippy, tests, dependency direction, licenses, and secret scanning.

Exit: empty composition root builds; forbidden dependency tests fail correctly.

### Gate 1: Durable Trunk

- Adapt Task/Run/Action/Evidence value objects.
- Adapt Kernel command/event/reducer flow.
- Move SQLite behind `TaskStorePort` with event + projection + outbox atomicity.
- Implement budget reservation and settlement.

Exit: stale commands fail; restart restores exact state; migration is idempotent.

### Gate 2: Worker Operation Machine

- Port the Goose operation protocol, not its application types.
- Implement cancellation, steering, limits, approval wait, tool result, retry, inference, and completion operations.
- Persist and reload after every applied operation.
- Reject duplicate tools and repeated-action cycles.

Exit: deterministic replay, cancellation, max-turn, and crash-between-steps tests pass.

### Gate 3: Cognitive and Context Branch

- Integrate hard constraint filtering and cost-aware S1/S2 routing.
- Add abstaining `JudgmentPort` and capability/quality registry.
- Compile immutable ContextPacks with provenance and token reservations.
- Implement tool-pair reduction, semantic compaction, and safe continuation packets.

Exit: user-pin, privacy, context, budget, stale-context, and quality-regression vectors pass.

### Gate 4: Controlled Execution

- Adapt grants, approvals, permit minting, revocation, and receipt recording.
- Port read/tree/search first; add edits and shell only inside sandbox/worktree policy.
- Reconcile unknown effects; never retry non-idempotent effects blindly.

Exit: no-permit, revoked-permit, changed-payload, symlink escape, timeout, and crash-post-effect tests pass.

### Gate 5: First Vertical Slice

- Run `repo_explain` through CLI -> daemon -> Kernel -> Context -> router -> worker -> provider -> Evidence.
- Use FakeProvider in CI and one opt-in real provider conformance job.
- Remove manual orchestration from the E2E test.

Exit: verified citations, usage ledger, artifacts, timeline, and restart resume are visible.

### Gate 6: Engineering Pack

- Add bug reproduction, localization, patch proposal, scoped apply, targeted tests, full verification, and human integration.
- Preserve dirty worktrees and reject stale bases.

Exit: a fixture bug is fixed with a verified diff and no unauthorized mutation.

### Gate 7: MCP and Provider Fleet

- Implement stdio and Streamable HTTP clients against the MCP specification.
- Add provider capability probes, streaming conformance, health, usage, and fallback at safe points.
- Keep provider-governed agents visibly distinct from Custos-mediated tools.

Exit: disconnect/resume, cancellation, protocol version, Origin/auth, and tool approval tests pass.

### Gate 8: Research and Assistant Packs

- Research: versioned sources, claims, passages, contradiction checks, citation support, experiment artifacts.
- Assistant: privacy classification, draft-first flow, identity resolution, exact-action approval, external receipts.

Exit: unsupported citation and wrong-recipient vectors create zero false success/effects.

### Gate 9: Bounded Multi-worker Runtime

- Add DAG admission, leases, heartbeats, structured handoffs, cancellation propagation, and orphan recovery.
- Compare single-worker, sequential-role, and parallel-explorer outcomes.

Exit: parallel work must improve a measured objective without exceeding coordination budgets or duplicating effects.

## 3. PR Sequence

| PR | Scope | Maximum review surface |
|---|---|---|
| N00 | workspace, provenance, dependency guards | build files and checks |
| N01 | pure domain contracts | domain only |
| N02 | Kernel plus store port | no adapters |
| N03 | SQLite adapter and recovery | persistence only |
| N04 | operation machine | workflow runtime only |
| N05 | routing and ContextPack | cognitive/context only |
| N06 | Authority and Gateway | controlled effects only |
| N07 | real `repo_explain` | one E2E slice |
| N08 | engineering read/write workflow | one pack |
| N09 | MCP/provider adapters | integrations only |
| N10 | research and assistant packs | one pack per PR |
| N11 | bounded multi-worker execution | after all prior gates |

## 4. Migration and Rollback

- Legacy Custos remains executable until Custos New passes Gate 5.
- Goose remains a pinned read-only upstream reference.
- No PR performs both extraction and behavioral redesign without characterization tests.
- Each gate has an adapter seam permitting rollback to FakeProvider or deterministic tools.
- Data migration is copy-and-verify; never mutate the legacy database in place.
- Promotion occurs by explicit user opt-in, followed by parallel read-only verification before writes are enabled.

## 5. Success Metrics

Measure per task class: verified success, false completion, total model cost, tokens, latency, human approval time, context precision/recall, retry count, duplicate work, recovery correctness, and unauthorized-effect count. Optimize cost only under hard security constraints and an agreed verified-quality floor.
