# Comprehensive Workspace Test Matrix & Baseline

> **Classification:** Normative Quality Assurance & Test Baseline  
> **Source of Truth:** Authoritatively defined in [Custos Master Specification](../../Custos.md).  
> **Directory Index:** See [Custos Developer Hub](README.md).

This document establishes the official test inventory, test distribution across the 20 Cargo workspace packages, continuous quality gate results, and invariant verification suites for Custos.

---

## 1. Continuous Quality Gates Verification Status (`cargo xtask gate`)

All changes merged into the codebase must achieve **100% PASS** on the 5 automated gates:

```text
================================================================================
           CUSTOS SADE COMPREHENSIVE REPOSITORY INTEGRITY GATE
================================================================================
Gate 1: Manifests & Descriptors               --> PASS (4 workbenches, 8 capabilities)
Gate 2: Desktop UI Build & Typecheck          --> PASS (0 errors, 1.1s)
Gate 3: Canvas Smoke Invariants               --> PASS (51/51 tests, 0.59s)
Gate 4: Architecture Invariants & Contracts   --> PASS (Line ceiling <= 600, 0.62s)
Gate 5: Workspace Cargo Check                 --> PASS (20 packages, 25.1s)
================================================================================
ALL 5 QUALITY GATES PASSED (Exit Code: 0)
================================================================================
```

---

## 2. Test Architecture Topology Across Rings

The test suites are distributed across the 3 concentric architectural rings:

### Ring 0: Platform Core Packages
| Package | Physical Path | Primary Test Focus | Suite Type |
|---|---|---|---|
| `custos-contracts` | `platform/contracts` | Pure entity state machines, ID generators, error types, port interfaces | Unit & Invariant (`tests/contract`) |
| `custos-storage` | `platform/storage` | SQLite WAL operations, outbox staging, crash recovery | Contract & Integration |
| `custos-integrations`| `platform/integrations`| Model router, local inference, ProcessDropGuard, PTY terminal, MCP Hub | System & OS Integration |
| `custos-engine` | `platform/engine` | Governed loop, DAG coordinators, T5LeadWorker, FastPathFilter, verifiers | Unit & State Transition |

### Ring 1: Specialized Workbenches
| Package | Physical Path | Primary Test Focus | Suite Type |
|---|---|---|---|
| `custos-workbench-assistant` | `workbenches/assistant` | Outbox approvals, draft management, message queue | Unit & Storage |
| `custos-workbench-coding` | `workbenches/coding` | 3-way worktree diffs, candidate patch integration | Unit & Git Integration |
| `custos-workbench-research` | `workbenches/research` | Citation coverage oracle, recipe reproduction | Unit & Verification |
| `custos-workbench-quant` | `workbenches/quant` | Backtest engine, PBO computation, pre-trade gates | Unit & Numerical Matrix |

### Ring 2: Protocol, SDK & Applications
| Package | Physical Path | Primary Test Focus | Suite Type |
|---|---|---|---|
| `custos-protocol` | `sdk/protocol` | Wire protocol serialization, schema round-trip | Schema & Unit |
| `custos-sdk` | `sdk/client-rust` | Local API client methods, reconnect loops | Unit & Mock API |
| `custos-daemon` | `apps/daemon` | Bootstrap orchestration, Local API endpoints, workbench dispatch | Integration & System |
| `custos-app-cli` | `apps/cli` | Command-line parsing, output formatting, terminal UX | Integration & Smoke |
| `custos-desktop` | `apps/desktop/native` | Tauri v2 host bindings, webview configuration | Native Platform Smoke |
|---|---|---|---|
| `custos-protocol` | `sdk/protocol` | Wire schema serialization, JSON-RPC 2.0 envelopes | Contract & Roundtrip |
| `custos-sdk` | `sdk/client-rust` | Asynchronous IPC client, streaming event parser | Integration |
| `custos-app-daemon` | `apps/daemon` | 197+ RPC handler tests across 4 bounded contexts | API Integration |
| `custos-app-cli` | `apps/cli` | CLI command parsing, terminal rendering, exit codes | CLI Functional |
| `custos-app-desktop` | `apps/desktop/native`| Native desktop bootstrap, IPC channel initialization | Desktop Host |
| `ui-desktop` | `ui/desktop` | 51 Headless DOM invariants (zero-mock, canvas layout) | DOM Canvas Smoke |

### Shared Test Infrastructure
| Package | Physical Path | Primary Test Focus | Suite Type |
|---|---|---|---|
| `custos-testkit` | `tests/testkit` | Mock servers, test directories, invariant assertions | Test Harness Support |
| `custos-tests-contract` | `tests/contract` | Migration durability, schema stability, crash matrix | Contract & Chaos |
| `custos-tests-e2e` | `tests/e2e` | Multi-topology execution canary (`oi_topologies_slice`) | End-to-End Canary |
| `xtask` | `xtask` | Architectural boundary enforcement, line ceiling check | Build-time Gate |

---

## 3. The 10 Master Themes Test Coverage Matrix

Every one of the 10 Master Thematic Pillars is directly covered and verified by automated tests:

| Theme | Thematic Pillar | Verifying Test Suite | Key Invariant Checked |
|---|---|---|---|
| **Theme 1** | SADE Vision & Agent Lake | `tests/e2e/`, `apps/daemon/src/api/tests/` | Multi-lens session state continuity |
| **Theme 2** | Task Kernel & State Machine | `platform/contracts`, `platform/engine`, `tests/contract` | Pure FSM transitions, atomic commits (INV-01, INV-02) |
| **Theme 3** | Authority & Capability Gateway | `platform/contracts`, `platform/engine`, `apps/daemon/src/api/tests/` | Zero ambient authority, single-use permits (INV-05, INV-10) |
| **Theme 4** | Governed Loop & Process Lifecycle | `platform/engine/src/execution/`, `platform/integrations/src/sandbox/` | `ProcessDropGuard` SIGKILL to `-pid` (INV-12) |
| **Theme 5** | Evidence Engine & Completion Gate | `platform/engine/src/evidence/`, `workbenches/research` | Verifier-isolated receipts, anti-tampering (INV-06, INV-07) |
| **Theme 6** | Measured OI & Economic Optimization | `platform/engine/src/orchestration/`, `tests/e2e/tests/oi_topologies_slice.rs` | T1–T5 execution topologies, Pareto selector |
| **Theme 7** | Storage Architecture & SQLite WAL | `platform/storage`, `tests/contract` | WAL crash recovery, batch rollback (INV-02, INV-13) |
| **Theme 8** | Heterogeneous Compute & GPU Admission| `platform/integrations/src/compute/`, `apps/daemon/src/api/tests/` | WorkloadAdmissionCoordinator VRAM feasibility |
| **Theme 9** | Vertical Specialist Workbenches | `workbenches/*/`, `apps/daemon/src/api/tests/workspaces/`, `quant/` | Workbench isolation, zero kernel-workbench leaks |
| **Theme 10**| Desktop UI & Operational Parity | `tests/desktop/canvas-smoke.mjs` (43+ tests) | Strict zero-mock in UI, 3-tier hierarchy |

---

## 4. Execution Commands for Developers

```bash
# Ensure Cargo and Node are on PATH
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"

# 1. Run all gates in sequence
cargo xtask check-boundaries

# 2. Run pure unit and contract tests
cargo test --package tests-testkit --package tests-contract --lib --tests --offline

# 3. Run individual platform crate tests
cargo test -p custos-contracts --lib --offline
cargo test -p custos-storage --lib --offline
cargo test -p custos-integrations --lib --offline
cargo test -p custos-engine --lib --offline

# 4. Run Desktop UI canvas smoke invariants
node tests/desktop/canvas-smoke.mjs

# 5. Run end-to-end topology slice
cargo test -p tests-e2e --test oi_topologies_slice --offline
```
