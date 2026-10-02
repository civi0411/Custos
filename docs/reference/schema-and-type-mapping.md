# Canonical Wire Schemas, Rust Type Mapping & Error Taxonomy

> **Classification:** Wire Contract & Type Reference  
> **Source of Truth:** Authoritatively defined in [Custos Master Specification](../../Custos.md).  
> **Directory Index:** See [Custos Developer Reference](README.md).

This document specifies the exact mapping between external JSON wire schemas, internal Rust domain types, protocol error codes, and SQLite persistence structures.

---

## 1. Protocol Wire Schemas (Wire Contracts)

| Schema Specification | JSON Schema Title | Rust Type | Defining Crate | Alignment Status |
|---|---|---|---|---|
| `envelope.schema.json` | `ProtocolEnvelope` | `ProtocolEnvelope` | `crates/custos-domain` | **Aligned** (RFC 3339 timestamp, UUIDs) |
| `error.schema.json` | `ProtocolError` | `DomainError` / `ProtocolError` | `crates/custos-domain` | **Aligned** (Error taxonomy mapping) |
| `provider-request.schema.json` | `ProviderRequest` | `ProviderRequest` | `crates/custos-provider` | **Identical** (1:1 field compatibility) |
| `provider-event.schema.json` | `ProviderEvent` | `ProviderEvent` | `crates/custos-provider` | **Identical** (Streaming event taxonomy) |
| `judgment-request.schema.json` | `JudgmentRequest` | `JudgmentRequest` | `crates/custos-core` | **Identical** (Criteria, candidate, context) |
| `judgment-result.schema.json` | `JudgmentResult` | `JudgmentResult` | `crates/custos-core` | **Identical** (Score, confidence, rationale) |
| `capability-request.schema.json`| `CapabilityRequest`| `CapabilityRequest` | `crates/custos-core` | **Identical** (Permit ID, action, arguments) |
| `execution-receipt.schema.json` | `ExecutionReceipt` | `ExecutionReceipt` | `crates/custos-domain` | **Identical** (Output digest, duration, status) |
| `continuation-packet.schema.json`| `ContinuationPacket`| `ContinuationPacket`| `crates/custos-domain` | **Identical** (SHA-256 integrity hash) |

---

## 2. Pack Domain Schemas

| Schema Specification | JSON Schema Title | Rust Type | Defining Crate | Scope |
|---|---|---|---|---|
| `pack-manifest.schema.json` | `PackManifest` | `DomainPackManifest` | `crates/custos-packs` | Pack metadata, tools, and workflows |
| `workflow.schema.json` | `WorkflowDefinition`| `WorkflowPlan`, `WorkflowStep` | `crates/custos-domain` | Directed acyclic graph execution nodes |
| `verifier-profile.schema.json` | `VerifierProfile` | `EvidenceKind`, `ContractEvidence` | `crates/custos-domain` | Acceptance criteria verifier rules |

---

## 3. Protocol Error Taxonomy

Every protocol error emitted across Unix Domain Sockets, HTTP, or SSE strictly conforms to the following taxonomy:

| Error Code | Rust `DomainError` Variant | HTTP Status | Retryable | Description |
|---|---|---|---|---|
| `INVALID_REQUEST` | `Validation` | 400 | False | Malformed payload, invalid JSON, or failed schema check |
| `UNAUTHORIZED` | `Unauthorized` | 401 | False | Missing or invalid authentication token |
| `FORBIDDEN` | `Forbidden` | 403 | False | Action exceeds granted scope or violation of boundary policy |
| `NOT_FOUND` | `NotFound` | 404 | False | Requested Task, Session, Permit, or Artifact does not exist |
| `CONFLICT` | `Conflict` | 409 | False | Optimistic concurrency conflict or revision mismatch |
| `PRECONDITION_FAILED`| `InvalidStateTransition` | 412 | False | FSM state does not permit the requested transition |
| `BUDGET_EXCEEDED` | `BudgetExceeded` | 429 | False | Task token or monetary limit cap reached |
| `TIMEOUT` | `Timeout` | 504 | True | Upstream model inference or tool execution timed out |
| `INTERNAL_ERROR` | `Internal` | 500 | True | Unrecoverable storage or daemon runtime error |

---

## 4. SQLite Relational Schema Mapping

The persistence layer in `crates/custos-persistence` manages durable state using SQLite in WAL mode. Core relational tables map to domain aggregates as follows:

| Table Name | Primary Key | Key Columns | Mapped Aggregate / Entity |
|---|---|---|---|
| `tasks` | `task_id` (TEXT) | `status`, `pack_kind`, `created_at`, `updated_at`, `revision` | `Task` Aggregate Root |
| `task_contracts` | `task_id` (TEXT) | `goal`, `scope_json`, `budget_limit`, `deadline`, `local_only` | `TaskContract` Value Object |
| `task_events` | `event_id` (TEXT) | `task_id`, `sequence_number`, `actor_id`, `event_type`, `payload` | Event Sourcing Event Stream |
| `execution_permits` | `permit_id` (TEXT)| `task_id`, `action_id`, `argument_hash`, `expires_at`, `used` | `ExecutionPermit` Entity |
| `effect_attempts` | `attempt_id` (TEXT)| `permit_id`, `status`, `receipt_digest`, `started_at` | `EffectAttempt` Tracking |
| `evidence_records` | `evidence_id` (TEXT)| `criterion_id`, `verdict`, `cas_hash`, `confidence` | `EvidenceRecord` Entity |
| `personal_facts` | `fact_id` (TEXT) | `subject`, `predicate`, `object`, `valid_from`, `valid_until` | LongMemEval Fact Store |
| `outbox_messages` | `message_id` (TEXT)| `topic`, `payload_json`, `dispatched`, `created_at` | Outbox Transactional Queue |

---

## 5. Contract Verification Testing

Schema-to-type alignment is automatically verified during continuous integration:
```bash
cargo test --package custos-domain --test schema_compat
```
The test suite ensures that:
1. Every JSON Schema specification file exists in the canonical repository schema directory.
2. Rust structures serialize and deserialize symmetrically against their corresponding JSON Schemas.
3. No field additions or deprecations occur without an explicit schema version bump.
