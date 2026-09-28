# Canonical Schema to Rust Type Mapping & Drift Audit

> **Status notice (2026-09-28):** Historical drift-audit snapshot with obsolete crate paths and unverified “Aligned/Identical” labels. Do not use it as a current contract map. Re-run schema-to-type conformance and publish a dated replacement before changing shared DTOs.

> **Status:** Historical baseline v4.0 (Gate 0 / GOV03)
> **Source:** `schemas/protocol/*.v1.schema.json` & `schemas/packs/*.v1.schema.json` vs. Rust Workspaces

This document provides the definitive mapping between JSON Schema definitions (v1) and Rust domain structures across the Custos workspace. It identifies field-level alignment, type contracts, and planned evolution vectors.

---

## 1. Protocol Schemas (Wire Contracts)

| Schema File | JSON Schema Title | Rust Type | Defining Crate | Alignment Status |
|---|---|---|---|---|
| `envelope.v1.schema.json` | `ProtocolEnvelope` | `ProtocolEnvelope` | `crates/core-domain::packet` | **Aligned** (v2 evolution drafted in ADR-0004) |
| `error.v1.schema.json` | `ProtocolError` | `DomainError` / `ProtocolError` | `crates/core-domain::error` | **Aligned** (maps error codes to enum variants) |
| `provider-request.v1.schema.json` | `ProviderRequest` | `ProviderRequest` | `crates/provider-sdk::request` | **Identical** (fields 1:1 match) |
| `provider-event.v1.schema.json` | `ProviderEvent` | `ProviderEvent` | `crates/provider-sdk::events` | **Identical** (streaming event taxonomy match) |
| `judgment-request.v1.schema.json` | `JudgmentRequest` | `JudgmentRequest` | `crates/judgment-contracts` | **Identical** (criteria, candidate, context) |
| `judgment-result.v1.schema.json` | `JudgmentResult` | `JudgmentResult` | `crates/judgment-contracts` | **Identical** (score, confidence, rationale) |
| `capability-request.v1.schema.json` | `CapabilityRequest` | `CapabilityRequest` | `crates/capability-gateway` | **Identical** (permit_id, action, params) |
| `execution-receipt.v1.schema.json` | `ExecutionReceipt` | `ExecutionReceipt` (`Receipt`) | `crates/core-domain::authority` | **Identical** (output_digest, duration, status) |
| `continuation-packet.v1.schema.json` | `ContinuationPacket` | `ContinuationPacket` | `crates/core-domain::continuation` | **Identical** (SHA-256 integrity hash verification) |

---

## 2. Pack Schemas (Domain Declarations)

| Schema File | JSON Schema Title | Rust Type | Defining Crate | Alignment Status |
|---|---|---|---|---|
| `pack-manifest.v1.schema.json` | `PackManifest` | `DomainPackManifest` | `crates/domain-pack-sdk` | **Aligned** |
| `workflow.v1.schema.json` | `WorkflowDefinition` | `WorkflowPlan`, `WorkflowStep` | `crates/core-domain::workflow` | **Aligned** (revision tracking added in Gate 1) |
| `verifier-profile.v1.schema.json` | `VerifierProfile` | `EvidenceKind`, `ContractEvidence` | `crates/core-domain::task`, `crates/evidence-engine` | **Aligned** |

---

## 3. Detailed Field-Level Alignment & Drift Notes

### 3.1 `envelope.v1.schema.json` vs. Rust Packet
- **Schema Fields:**
  - `version`: string (must be `"1.0"`)
  - `message_id`: string (format `^msg_[a-f0-9]+$`)
  - `correlation_id`: string (nullable)
  - `timestamp`: string (ISO 8601 / RFC 3339 date-time)
  - `kind`: string
  - `payload`: object
- **Drift / Evolution Note:**
  The multi-worker research blueprint proposes adding `task_id`, `step_id`, `run_id`, `command_id`, `expected_task_revision`, and `deadline_ms` to the envelope.
  Per ADR-0004 and the Implementation Plan, these additions will be published as optional fields in `envelope.v1.1.schema.json` or `envelope.v2.schema.json` without breaking v1 backward compatibility.

### 3.2 `error.v1.schema.json` vs. `DomainError`
- **Schema Fields:**
  - `code`: enum (`INVALID_REQUEST`, `NOT_FOUND`, `CONFLICT`, `UNAUTHORIZED`, `BUDGET_EXCEEDED`, `TIMEOUT`, `INTERNAL_ERROR`)
  - `message`: string
  - `retryable`: boolean
  - `details`: object (nullable)
- **Rust Implementation:**
  - In `core-domain`, `DomainError` uses `thiserror` with typed variants (`InvalidStateTransition`, `IntegrityCheckFailed`, `BudgetExceeded`, `Conflict`, `Validation`, `NotFound`, `InvariantViolation`, `Unauthorized`).
  - Serialization to `ProtocolError` maps variants directly to schema error codes with deterministic `retryable` flags.

### 3.3 `continuation-packet.v1.schema.json` vs. `ContinuationPacket`
- **Schema Fields:**
  - `task_id`: string
  - `from_span`: integer (>= 0)
  - `to_span`: integer (>= 0)
  - `provider`: string
  - `model`: string
  - `task_summary`: string
  - `current_state`: object
  - `integrity_hash`: string (regex `^[a-f0-9]{64}$`)
  - `created_at`: string (date-time)
- **Rust Implementation:**
  - `crates/core-domain/src/continuation.rs` fields and canonical SHA-256 computation match the schema 100%. Tested in contract tests.

### 3.4 `execution-receipt.v1.schema.json` vs. `ExecutionReceipt`
- **Schema Fields:**
  - `receipt_id`: string
  - `permit_id`: string
  - `action_id`: string
  - `status`: enum (`success`, `failure`, `timeout`, `rejected`)
  - `output_digest`: string (SHA-256 digest)
  - `output_data`: object (optional)
  - `error_message`: string (optional)
  - `duration_ms`: integer (optional)
  - `executed_at`: string (date-time)
- **Rust Implementation:**
  - Implemented in `crates/core-domain/src/authority.rs` with `pub type Receipt = ExecutionReceipt;`. All serde attributes match schema constraints.

---

## 4. Contract Test Verification

Contract test suite at `tests/contract/tests/schema_compat.rs` executes during `cargo test --workspace` to ensure:
1. Every canonical JSON schema file exists and is parseable.
2. Serialization of Rust types validates against the JSON Schema specifications.
