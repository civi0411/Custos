# ADR: Task State Machine & Status Taxonomy Alignment

**Status:** retained lifecycle decision; path references updated for the current tree. Evidence and completion semantics remain governed by the proposed C-04 contract until accepted and trust-closed.

## Context & Problem Statement

The canonical specification and `AGENTS.md` describe the Task lifecycle as:
`Draft` → `Ready` → `Running` → `Verifying` → `Succeeded` / `Failed` / `Paused`.

In the initial Rust implementation, now located at `crates/core/custos-domain/src/task.rs`, and SQLite schema, the status enum was implemented as:
`Draft`, `Queued`, `Running`, `Blocked`, `Succeeded`, `Failed`, `Cancelled`.

This created a divergence:
1. `Ready` in specification vs. `Queued` in Rust.
2. `Verifying` in specification vs. step-level execution under `Running` in Rust.
3. `Paused` in specification vs. `Blocked` in Rust.
4. `Cancelled` is present in Rust/persistence but omitted in the brief summary in `AGENTS.md`.

We need an authoritative architectural decision record establishing the canonical state machine taxonomy, explaining the mapping, and defining the transition invariants.

## Decision Outcome

1. **Retain Rust `TaskStatus` representation for runtime stability:**
   - `Draft`: Initial state, task contract being authored.
   - `Queued`: Task validated and queued for dispatcher/worker acquisition (corresponds to `Ready`).
   - `Running`: Worker has acquired lease and is executing steps.
   - `Blocked`: Task awaiting external resolution, e.g. human approval or tool permit (corresponds to `Paused`).
   - `Succeeded`: Terminal state, all acceptance conditions verified by evidence.
   - `Failed`: Terminal state, non-recoverable error or budget exhaustion.
   - `Cancelled`: Terminal state, explicitly terminated by human principal.

2. **Verification Stage (`Verifying`):**
   - In Custos, verification is executed deterministically by the `evidence-engine` as a mandatory phase before concluding a step or transitioning to `Succeeded`. A task remains in `Running` (with the step marked in verifying mode) until verification passes, preventing race conditions with intermediate status changes.

3. **Status Taxonomy Equivalence:**
   - `Ready` ≡ `Queued`
   - `Paused` ≡ `Blocked`
   - Explicit `Cancelled` state is recognized as a canonical terminal state.

## Considered Options

- **Option A (Breaking Rename to strict specification words):** Rename `Queued` → `Ready`, `Blocked` → `Paused`, add explicit `Verifying` enum variant.
  - *Cons:* Breaks existing database rows in SQLite, requires immediate database migration, invalidates serialized tasks.
- **Option B (Maintain semantic equivalence with documented mapping and alias support):** Keep current enum values in SQLite and Rust, document exact semantic equivalence, and support serde alias `ready` -> `Queued`, `paused` -> `Blocked`.
  - *Pros:* 100% backward compatible with existing migrations (0001-0003), zero downtime, zero serialization breakage.

## Consequences

- **Positive:** No database migration needed for Gate 0; existing CQRS reducer, commands, and tests continue to pass without risk.
- **Negative / Risks:** Developers reading `AGENTS.md` summary might notice `Queued` vs `Ready`; resolved by cross-referencing this ADR and adding serde aliases where applicable.

## Verification & References

- `crates/core/custos-domain/src/task.rs` unit tests
- `crates/core/custos-kernel/src/reducer.rs` transition tests
- `tests/e2e/tests/cli_lifecycle.rs` persistence tests
