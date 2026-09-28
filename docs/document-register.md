# Document register and disposition

**As of:** 2026-09-28. **Status:** navigation inventory, not a claim that every legacy statement is current. No historical document was deleted in this restructuring.

| Existing family | Current role | Next action |
|---|---|---|
| `00-start-here.md`, `canonical-specification.md` | Legacy entrypoint and V8 design baseline | Use [`README.md`](README.md) as the new entrypoint; keep V8 readable with a status warning. |
| `product/identity.md`, `scope.md`, `capability-map.md` | Product intent | Review for target-vs-implemented language; product lead proposes, technical owners review boundaries. |
| `architecture/overview.md`, `task-lifecycle.md`, `cognitive-fabric.md`, `capability-gateway.md`, `evidence-verification.md` | Earlier architectural perspectives | Reconcile against [target architecture](architecture/target-architecture.md) and accepted contracts before treating normative snippets as current. |
| `architecture/reference-architecture.md`, `target-architecture.md`, `runtime-flows.md`, `connectivity-hubs.md`, `intelligence-hub.md` | Active target architecture set | Keep mutually consistent; implementation claims still require `status/` evidence. |
| `architecture/communication.md`, `context-memory.md`, `crash-recovery.md`, `deployment.md`, `persistence.md`, `provider-interop.md` | Earlier subsystem designs | Keep for detail; attach source-backed status and versioned contracts as implementation proceeds. |
| `architecture/COGNITIVE_ARCHITECTURE_S1_S2.md` | Research/design proposal | Evaluate against the [multi-candidate Hub design](architecture/intelligence-hub.md), not a second canonical router. |
| `architecture/CUSTOS_HYBRID_MASTER_ARCHITECTURE_2026.md` | Vietnamese research and implementation companion | Non-normative; extract accepted decisions into English ADRs or ARCH-REF-01 before implementation. |
| `security/capability-model.md`, `privacy.md`, `threat-model.md` | Security design | Keep; reconcile authority and effect claims with the Action/Attempt/Receipt contract. |
| `domains/engineering.md`, `research.md`, `personal.md`, `AGENT_PATHOLOGIES_AND_SOLUTIONS.md` | Pack designs and research | Mark features by release gate; pathology analysis remains research input. |
| `development/codebase.md`, `observability.md`, `oss-adoption.md`, `roadmap.md`, `testing.md` | Earlier contributor plan | The [implementation blueprint](development/implementation-blueprint.md) owns current sequencing; the 16-week roadmap is a historical hypothesis. |
| `reference/comparisons.md`, `concepts.md`, `invariants.md`, `naming.md`, `schema-mapping.md`, `sources.md` | Glossary and reference | Keep; link normative invariants to accepted contract IDs and tested source versions. |
| `adr/README.md`, `action-intent-and-permit-contracts.md`, `task-state-machine-alignment.md` | Decision proposals/history | Each ADR needs explicit Proposed/Accepted/Superseded status and reviewers. |
| `status/codebase-inventory.md`, `feature-state.md`, `source-path-map.md` | 2026-09-25 baseline snapshots | Preserve as history; use [`status/README.md`](status/README.md) for live status methodology. |
| `specifications/ASSEMBLY_PLAN.md`, `SOURCE_MAP.md`, `CUSTOS_*`, `GOOSE_*` | Older plans, extraction and source analyses | Preserve with source/date; import decisions through ADRs. Their crate counts and layouts do not override Cargo metadata. |
| `archive/{CUSTOS_SOVEREIGN_MASTER_PLAN,NEXUS_FULL_AUDIT_LATEST,NEXUS_REPOS_DETAILED_ANALYSIS}.md` | Legacy analyses moved from the docs root | Treat numerical and “latest” claims as dated snapshots. |
| `archive/dev_docs-2026-09-27/**` | Divergent duplicate of the work hub | Historical mirror only. The root [`dev_docs/`](../dev_docs/README.md) is the operational path; unique content is preserved. |
| `i18n/**` | Localized copies | Must identify their source document and version; not normative on their own. |
| `goose/**` | Imported upstream documentation/source | Vendor reference, not Custos product documentation; retain license/provenance and exclude from current-status claims. |
| `status/goose-naming-migration.md`, `development/naming-conventions.md` | Current naming authority | Classifies upstream identity, compatibility surfaces and accidental product naming; governs staged rename work. |

The current end-state design is [ARCH-REF-01](architecture/reference-architecture.md), with [ARCH-FLOW-01](architecture/runtime-flows.md) for execution semantics, [ARCH-CONNECT-01](architecture/connectivity-hubs.md) for model/agent/MCP connections and [DEV-REPO-01](development/repository-structure.md) for repository boundaries. These are target designs; [implementation status](status/README.md) remains source-and-test based. Root `README.md` and `ARCHITECTURE.md` point to this set; prior drafts are historical.

## Change protocol

For every page newly claimed as current, include document ID, owner, status, date, superseded ID, and a code SHA when making an implementation claim. Cross-team contracts require producer and consumer tests plus recorded maintainer approval. Do not bulk-delete older documents until inbound links, provenance and unique content are checked.
