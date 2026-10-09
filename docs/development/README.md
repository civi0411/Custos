# Custos Developer Hub & Getting Started

**Thứ tự triển khai hiện hành:** [Backend–desktop convergence §2/§7](sade-frontend-backend-convergence-plan.md#2-những-vấn-đề-hiện-tại-phải-xử-lý-theo-thứ-tự-rủi-ro) dựa trên checkout `4b73cce`: khóa ingress/physical execution và capability UI, làm một chat→worker→answer thật, rồi Coding/Research vertical slices, continuity/Assistant, cuối cùng delegated multiworker và optimization. Các bảng PR/wave khác là kế hoạch dài hạn hoặc lịch sử nếu thứ tự khác. [Upstream matching ledger](upstream-source-map.md#cross-source-nexus-audit-orca-and-open-science) xác định mức đã hiện thực của OrCa/Open Science.

Để refactor đúng từng crate, đọc [blueprint §22](workspace-restructuring-plan.md#22-crate-blueprint-và-chuyển-lõi-orca-theo-trách-nhiệm): actual/target dependencies, module addresses, OrCa core decomposition, atomic storage operations, resource/lease gap và skill/model/native integration.

Để giao triển khai kết hợp desktop/headless, bắt đầu từ [superplan §21](workspace-restructuring-plan.md#21-superplan-kết-hợp-custos-và-orca-cho-desktop-và-headless): reuse decisions, source findings, module map, packets đầu và gates. Mobile nằm ngoài scope; ba pack cùng phát triển trên foundation OF/R/P hiện có.

**Chiến dịch hiện tại:** [SADE refactor plan](workspace-restructuring-plan.md#11-quyết-định-khóa-cho-chiến-dịch-sade) — source gaps → live worker/cost ledger → native resources → Coding/Research/Assistant → coordination → measured optimization. [OrCa foundation §20](workspace-restructuring-plan.md#20-chương-trình-hấp-thụ-orca-vào-nền-custos) map toàn bộ capability families sang Custos OF0–OF7; đây là functional assimilation, không nhúng OrCa runtime. [Domain design](../architecture/domain-packs-and-workflows.md) và [evaluation contract](../../evals/oi/README.md) đi cùng; không đợi OI hoàn thiện mới đo economics.

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
