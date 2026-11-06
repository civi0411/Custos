# Custos Developer Hub & Getting Started

**Base + downloadable workbench design:** [Modular workbench refactor plan](modular-workbench-refactor-plan.md) is the proposed packaging campaign: Assistant built-in, Coding/Research/Quant optional; one shared agent runtime/data spine; exact crate/UI placement, data and install flows, compatibility/security, multi-agent resource economics and P00–P11 delivery gates. It does not mark earlier correctness/migration work complete. M1 optional activation/downloadable engines and M2 independently downloadable UI/runtime bundles have different acceptance tests; review this plan before implementing installer or replacing hardcoded workbench registration.

The same plan's §§19–25 cover the subsequent partial implementation, Rust composition/OOP boundaries, shared engine dependencies, data retention, stateful resource lifetimes and a consolidated delivery order. No separate packaging architecture or campaign is required.

**Current crates and workbench audit:** [Three-reference follow-up](workbench-core-and-controls-audit.md#current-custos-crates-ui-and-three-reference-follow-up) traces installation/admission, truthful UI, Research assessment, Quant metrics and worker transport against OrCa, Open Science and QuantDinger. Findings C01–C16 feed §26 of the existing modular plan; source inventory and passing build/smoke checks do not certify complete feature parity.

**Implementation starts here:** [Full-system execution plan §§27–33](modular-workbench-refactor-plan.md#27-phạm-vi-refactor-bắt-buộc-và-cấu-trúc-đích) details all twelve P00–P11 packets, exact topic homes, UI state ownership, data migration, per-packet gates/rollback and five delivery waves. Start with baseline/compatibility and truthful UI/navigation, then transactional inventory + admission; independently downloadable UI follows real vertical execution. No production implementation is claimed by this plan update.

**Supplied master design review:** [D01–D22](workbench-core-and-controls-audit.md#review-of-the-supplied-master-architectural-design) reconciles the full 229-line proposal with current source and primary SQLite/Jupyter/Tauri/Rust/update-security documentation. It preserves the concise overview while correcting crate counts, data ownership, package lifecycle/trust and kernel/resource guarantees; adopted planning clarifications live in the existing P00–P11 queue.

**Crate assembly:** [SADE design §§10–13](../architecture/sade-agent-system-design.md#10-cohesive-assembly-blocks-inside-the-current-crates) explains the actual block homes, single composition graph, Rust interface/lifetime rules and workbench/skill/executor assembly. [Current A01–A09 findings](workbench-core-and-controls-audit.md#crate-assembly-follow-up) distinguish structural inventory from traced gaps; follow the existing P04/P07/P10/P11 packets to repair them.

**Thứ tự triển khai để duyệt hiện hành:** [Backend–desktop convergence §12](sade-frontend-backend-convergence-plan.md#12-refactor-crates-và-desktop-theo-ownership) map toàn bộ 11 nhóm crate và frontend vào 12 gói: baseline, effect/driver correctness, one Screen/layout owner, AppContext/transport extraction, shell/center/inspector, settings, Coding/Copilot/Research, performance/visual, OI và nghiệm thu. UI cleanup có thể bắt đầu sau baseline, không đợi full GPU/OI. Các queue §2/§7 và snapshot `4b73cce` là lịch sử khi khác §12; plan không đồng nghĩa đã triển khai. [Upstream matching ledger](upstream-source-map.md#cross-source-nexus-audit-orca-and-open-science) xác định mức đã hiện thực của OrCa/Open Science.

Để refactor đúng từng crate, đọc [blueprint §22](workspace-restructuring-plan.md#22-crate-blueprint-và-chuyển-lõi-orca-theo-trách-nhiệm): actual/target dependencies, module addresses, OrCa core decomposition, atomic storage operations, resource/lease gap và skill/model/native integration.

Để giao triển khai kết hợp desktop/headless, bắt đầu từ [superplan §21](workspace-restructuring-plan.md#21-superplan-kết-hợp-custos-và-orca-cho-desktop-và-headless): reuse decisions, source findings, module map, packets đầu và gates. Mobile nằm ngoài scope; ba pack cùng phát triển trên foundation OF/R/P hiện có.

**Chiến dịch hiện tại:** [SADE refactor plan](workspace-restructuring-plan.md#11-quyết-định-khóa-cho-chiến-dịch-sade) — source gaps → live worker/cost ledger → native resources → Coding/Research/Assistant → coordination → measured optimization. [OrCa foundation §20](workspace-restructuring-plan.md#20-chương-trình-hấp-thụ-orca-vào-nền-custos) map toàn bộ capability families sang Custos OF0–OF7; đây là functional assimilation, không nhúng OrCa runtime. [Domain design](../architecture/domain-packs-and-workflows.md) và [evaluation contract](../../evals/oi/README.md) đi cùng; không đợi OI hoàn thiện mới đo economics.

**Implementation plan (execution level):** [sade-implementation-plan.md](sade-implementation-plan.md) — PR-by-PR breakdown of F01–F12 with verified findings, UI conflict verdict, tests and rollback.


For the concrete source refactor, read [audit and R0–R10 packets](workspace-restructuring-plan.md#6-audit-code-và-quyết-định-giữchuyểnhợp-nhất): exact current paths, keep/move/consolidate decisions, target imports, compatibility and verification gates. Package names come from Cargo metadata; folder names alone do not establish active runtime behavior.

The [OrCa source study](orca-source-study.md) pins a local upstream commit and separates what OrCa already implements from the Custos SADE contracts and migration gates. Read it before designing worktree, native agent launch, worker supervision or multi-run UI.

Start product restructuring with the [Agent Workspace migration plan](workspace-restructuring-plan.md), [workspace/UI specification](../architecture/agent-workspace-and-ui.md), and [capability/skill ownership](../architecture/capability-catalog-and-skills.md). These define current-to-target mappings and measurable gates; they do not claim source files have already moved. Build the read-only workspace UI alongside the execution spine, then enable mutation/domain features only after their gates pass.

> **Classification:** Normative Developer Onboarding & Engineering Hub  
> **Source of Truth:** Authoritatively defined in [Custos Master Specification](../../Custos.md).  
> **Repository Documentation Hub:** See [Custos Documentation Overview](../README.md).

Welcome to the Custos developer portal. Custos is a local-first, human-governed runtime for proof-carrying agentic AI work. This directory contains practical guides for building, testing, contributing to, and operating within the Custos monorepo.

---

## 1. Quick-Start & Development Commands

Ensure you have installed the pinned Rust toolchain (`rust-toolchain.toml`) and SQLite runtime dependencies before developing.

```bash
# Check compilation across the entire 11-crate workspace
cargo check --workspace

# Run pure unit and schema contract tests
cargo test --workspace

# Run strict clippy linter (zero warnings policy)
cargo clippy --workspace --all-targets -- -D warnings

# Check code formatting compliance
cargo fmt --all -- --check

# Audit external dependencies and licenses
cargo deny check
```

---

## 2. Development Guides Directory

The development documentation includes focused engineering guides and an upstream source audit:

| Guide | Scope & Key Topics | Primary Target |
|---|---|---|
| **[Monorepo Topology & Crate Boundaries](codebase-architecture.md)** | The 11 canonical product crates layout (`crates/`), unidirectional dependency rules, zero-I/O domain policy, and monorepo packaging. | All Rust developers & contributors |
| **[Testing & Verification Standards](testing-and-verification.md)** | The 5-layer testing pyramid, automated crash recovery matrix, 8 product acceptance gates (Gates A–OPT), and status claim standards. | Test engineers, security reviewers, CI maintainers |
| **[Engineering Standards & Observability](engineering-standards.md)** | 5 open source adoption modes, dependency whitelist, license compliance (`cargo-deny`), OpenTelemetry tracing, and pre-log secret redaction. | Platform engineers, DevOps, security auditors |
| **[Delivery Blueprint & Release Gates](delivery-blueprint.md)** | Vertical slice engineering methodology (observe $\rightarrow$ choose $\rightarrow$ work $\rightarrow$ authorize $\rightarrow$ verify $\rightarrow$ continue), release gates (G0–G5), and PR delivery sequence (PR-00 to PR-11). | Release managers, team leads, system architects |
| **[Upstream Source Inventory](upstream-source-map.md)** | Goose-derived source locations, runtime status, provenance gaps, and attribution audit fields. | Integrators and release reviewers |
| **[OrCa Source Study](orca-source-study.md)** | Pinned OrCa source evidence, actual runtime/worktree/orchestration/agent/UI boundaries, limits of Nexus inventory, and Custos implementation gates. | Runtime, UI and architecture implementers |

---

## 3. Role-Based Onboarding Pathways

| Discipline | Focus Areas | Recommended Reading Path |
|---|---|---|
| **Systems & Core Runtime** | Kernel state machines, SQLite WAL persistence, capability sandbox, process lifecycle. | 1. [Custos.md](../../Custos.md) (Parts 1, 2, 3, 15)<br>2. [Monorepo Topology](codebase-architecture.md)<br>3. [Testing & Verification](testing-and-verification.md) |
| **AI & Cognition Architects** | Orchestration intelligence, cognitive routing, context compilation, memory tiers, model contracts. | 1. [Custos.md](../../Custos.md) (Parts 6, 9, 13, 14)<br>2. [Canonical Glossary](../reference/glossary.md)<br>3. [Delivery Blueprint](delivery-blueprint.md) |
| **Security & Platform** | Threat model, capability permits, taint tracking, cryptographic evidence, sandbox containment. | 1. [Custos.md](../../Custos.md) (Parts 4, 5, 8)<br>2. [System Invariants](../reference/system-invariants.md)<br>3. [Engineering Standards](engineering-standards.md) |
| **Client & CLI Developers** | Local API daemon, SDK client contracts, CLI operations, external harness adapters. | 1. [Custos.md](../../Custos.md) (Part 7)<br>2. [Schema & Type Mapping](../reference/schema-and-type-mapping.md)<br>3. [Monorepo Topology](codebase-architecture.md) |

---

## 4. Document Authority Order

When technical disagreements or documentation drift arise, adhere to this strict precedence order:

1. **Current Active Source Code & Automated Tests:** Rust crates (`crates/`) and test suites (`tests/`).
2. **Repository Governance:** [`AGENTS.md`](../../AGENTS.md) (contribution standards, agent boundaries, safety rules).
3. **Definitive Master Specification:** [`Custos.md`](../../Custos.md) (canonical single source of truth).
4. **Specialized Guides:** Deep-dive specifications in `docs/architecture/`, `docs/reference/`, and `docs/development/`.
