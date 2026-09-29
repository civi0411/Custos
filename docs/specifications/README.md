# Historical Specifications & Extraction Archive

> **Status:** Historical Archive / Non-Normative Reference  
> **Document ID:** SPEC-ARCHIVE-01  
> **Authority:** This directory contains historical planning documents, code extraction maps, and early architecture deep-dives produced prior to the 2026-09-28 workspace reorganization.  
> **Active Authority:** For the current, normative system design, refer to [`docs/architecture/reference-architecture.md`](../architecture/reference-architecture.md), [`docs/architecture/runtime-flows.md`](../architecture/runtime-flows.md), and [`docs/contracts/README.md`](../contracts/README.md). For current workspace layout, see [`docs/development/repository-structure.md`](../development/repository-structure.md).

---

## 1. Scope and Use of These Documents

The files in this directory are preserved for engineering provenance, design rationale, and historical decision tracing. **None of these documents override the 42-crate Cargo workspace metadata, active contracts, or empirical audit results.**

| Document | Nature & Historical Purpose | Current Normative Replacement |
|---|---|---|
| [`ASSEMBLY_PLAN.md`](ASSEMBLY_PLAN.md) | Early phased assembly gates and construction strategy. | [`docs/development/implementation-blueprint.md`](../development/implementation-blueprint.md) |
| [`CUSTOS_COMPLETE_ARCHITECTURE_DEEP_DIVE_VI.md`](CUSTOS_COMPLETE_ARCHITECTURE_DEEP_DIVE_VI.md) | Deep-dive Vietnamese code flow analysis based on early 34-crate snapshot. | [`docs/architecture/reference-architecture.md`](../architecture/reference-architecture.md) |
| [`CUSTOS_DINH_HINH_BAI_TOAN_VA_KIEN_TRUC_SAN_PHAM_VI.md`](CUSTOS_DINH_HINH_BAI_TOAN_VA_KIEN_TRUC_SAN_PHAM_VI.md) | Early Vietnamese product formulation and architecture rationale. | [`docs/product/identity.md`](../product/identity.md) & [`docs/product/scope.md`](../product/scope.md) |
| [`CUSTOS_NEW_FULL_CODEBASE_REASSEMBLY_MAP.md`](CUSTOS_NEW_FULL_CODEBASE_REASSEMBLY_MAP.md) | Codebase reorganization map from early restructuring spike. | [`docs/development/repository-structure.md`](../development/repository-structure.md) |
| [`CUSTOS_CODEBASE_ROOT_AND_GOOSE_EXTRACTION_V1_VI.md`](CUSTOS_CODEBASE_ROOT_AND_GOOSE_EXTRACTION_V1_VI.md) | Upstream Goose component extraction analysis. | [`docs/research/README.md`](../research/README.md) & [`docs/status/goose-naming-migration.md`](../status/goose-naming-migration.md) |
| [`CUSTOS_NEXUS_AUDIT_AND_REASSEMBLY_V2_VI.md`](CUSTOS_NEXUS_AUDIT_AND_REASSEMBLY_V2_VI.md) | Nexus AST and call-graph audit snapshot. | [`docs/status/local-dev-audit-2026-09-28.md`](../status/local-dev-audit-2026-09-28.md) |
| [`CUSTOS_SUPER_PLAN_REFACTOR_VA_PHAT_TRIEN_V1_VI (2).md`](CUSTOS_SUPER_PLAN_REFACTOR_VA_PHAT_TRIEN_V1_VI%20%282%29.md) | Early Vietnamese refactoring plan and contract definitions. | [`docs/development/implementation-blueprint.md`](../development/implementation-blueprint.md) |
| [`CUSTOS_UNIFIED_AGENT_SUPERPLAN_2026.md`](CUSTOS_UNIFIED_AGENT_SUPERPLAN_2026.md) | Early unified agent superplan. | [`docs/architecture/target-architecture.md`](../architecture/target-architecture.md) |
| [`GOOSE_COMPLETE_ARCHITECTURE_DEEP_DIVE_VI.md`](GOOSE_COMPLETE_ARCHITECTURE_DEEP_DIVE_VI.md) | Deep analysis of upstream Goose codebase for feature extraction. | [`docs/vendor/README.md`](../vendor/README.md) & [`docs/goose/`](../goose/README.md) |
| [`SOURCE_MAP.md`](SOURCE_MAP.md) | Initial mapping of source components. | [`docs/status/source-path-map.md`](../status/source-path-map.md) |

---

## 2. Path Mapping for Historical Crate References

When reading historical documents in this folder, note the following path migrations:

| Historical Path in Spec | Current Actual Crate Path |
|---|---|
| `crates/core-domain` | `crates/core/custos-domain` |
| `crates/task-kernel` | `crates/core/custos-kernel` |
| `crates/authority-engine` | Merged into `crates/runtime/custos-security` |
| `crates/evidence-engine` | Merged into `crates/runtime/custos-security` |
| `crates/capability-gateway` | `crates/runtime/custos-security/src/gateway/` |
| `crates/cognitive-runtime` | `crates/runtime/custos-cognitive` |
| `crates/workflow-runtime` | `crates/runtime/custos-workflow` |
| `crates/persistence-sqlite` | `crates/infrastructure/custos-persistence` |
| `crates/local-api` | `crates/app/custos-local-api` |
| `apps/custosd` | `crates/app/custos-daemon` |
| `crates/provider-sdk` | `crates/core/custos-provider-sdk` |
| `crates/context-compiler` | `crates/runtime/custos-context` |
