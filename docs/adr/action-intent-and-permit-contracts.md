# ADR: ActionIntent, ExecutionPermit, and ExecutionReceipt Contracts

**Status:** historical accepted record pending current-path and conformance review. The trust-closure requirements in [C-03](../contracts/README.md) govern new work; old paths below describe the pre-restructure tree.

## Context & Problem Statement

The Custos canonical specification and `AGENTS.md` define the capability security boundary as:
1. An agent or worker proposes an **`ActionIntent`** (an untrusted, unverified action description).
2. The `AuthorityEngine` evaluates policy, constraints, and risk, minting a single-use, time-bound **`ExecutionPermit`**.
3. The `CapabilityGateway` verifies the permit against the action digest and executes the side-effect within a sandbox.
4. The execution generates an **`ExecutionReceipt`** (containing output digest, timing, and evidence) proving the action occurred.

In the early Rust codebase:
- `crates/core/custos-domain/src/action.rs` names the type `Action`.
- `crates/core/custos-domain/src/authority.rs` names the permit type `Permit`.
- `crates/runtime/custos-security/src/gateway/traits.rs` names the execution outcome `ExecutionResult`.
- `schemas/protocol/execution-receipt.v1.schema.json` defines `ExecutionReceipt`.

This naming divergence created confusion about whether `Action` and `ActionIntent` were different types, and whether `Permit` satisfied the requirements of an `ExecutionPermit`.

## Decision Outcome

1. **Unify Core Domain Types with Canonical Specification:**
   - In `crates/core/custos-domain/src/action.rs`: provide `pub type ActionIntent = Action;` as the canonical alias representing proposed side effects.
   - In `crates/core/custos-domain/src/authority.rs`: provide `pub type ExecutionPermit = Permit;` as the canonical alias representing authority tokens.
   - In `crates/core/custos-domain/src/authority.rs`: define `ExecutionReceipt` and `pub type Receipt = ExecutionReceipt;`, matching `schemas/protocol/execution-receipt.v1.schema.json`.

2. **Invariants Preserved:**
   - No side-effect action may be executed by the `CapabilityGateway` without an active, unexpired `ExecutionPermit`.
   - Every completed side-effect must produce an `ExecutionReceipt` bearing an `output_digest`.
   - Zero breaking changes to existing call sites using `Action` or `Permit`.

## Considered Options

- **Option A (Destructive Rename):** Completely rename `Action` to `ActionIntent` and `Permit` to `ExecutionPermit` across all crates.
  - *Cons:* Breaches many existing crates and tests simultaneously without providing extra functional safety.
- **Option B (Canonical Type Aliases & Schema-Aligned Types):** Add type aliases `ActionIntent = Action` and `ExecutionPermit = Permit`, and implement `ExecutionReceipt` directly in `core-domain`.
  - *Pros:* Fully satisfies the glossary requirement in `AGENTS.md`, matches JSON Schema v1, maintains 100% backward compatibility.

## Consequences

- **Positive:** Both naming forms are valid and interoperable across the entire workspace.
- **Negative / Risks:** Developers must remember that `Action` and `ActionIntent` refer to the same immutable domain model.

## Verification & References

- `crates/core/custos-domain/src/action.rs`
- `crates/core/custos-domain/src/authority.rs`
- `crates/runtime/custos-security/src/gateway/deterministic.rs`
- `schemas/protocol/execution-receipt.v1.schema.json`
