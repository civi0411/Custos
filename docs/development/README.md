# Custos Developer Hub & Getting Started

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
