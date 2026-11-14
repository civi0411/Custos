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
| `custos-domain` | `platform/domain` | Pure entity state machines, ID generators, error types | Unit & Property (`proptest`) |
| `custos-core` | `platform/core` | TaskService, Two-phase authority, supervisor contracts | Unit & Invariant |
| `custos-persistence` | `platform/storage` | SQLite WAL operations, outbox staging, crash recovery | Contract & Integration |
| `custos-provider` | `platform/ai` | Model router, prompt cache affinity, local inference | Unit & Mock Provider |
| `custos-runtime` | `platform/runtime` | TaskRuntime, DAG coordinators, T5LeadWorker, FastPathFilter | Unit & State Transition |
| `custos-adapters` | `platform/adapters` | ProcessDropGuard, worktree isolation, PTY terminal, MCP Hub | System & OS Integration |
| `custos-packs` | `platform/components` | Pack manifest parsing, verifier registry, role discovery | Unit & Schema |

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
| **Theme 2** | Task Kernel & State Machine | `platform/domain`, `platform/core`, `tests/contract` | Pure FSM transitions, atomic commits (INV-01, INV-02) |
| **Theme 3** | Authority & Capability Gateway | `platform/core`, `apps/daemon/src/api/tests/shared_capabilities.rs` | Zero ambient authority, single-use permits (INV-05, INV-10) |
| **Theme 4** | Governed Loop & Process Lifecycle | `platform/runtime/src/agent/runner.rs`, `platform/adapters` | `ProcessDropGuard` SIGKILL to `-pid` (INV-12) |
| **Theme 5** | Evidence Engine & Completion Gate | `platform/components`, `workbenches/research` | Verifier-isolated receipts, anti-tampering (INV-06, INV-07) |
| **Theme 6** | Measured OI & Economic Optimization | `platform/runtime/src/oi/`, `tests/e2e/tests/oi_topologies_slice.rs` | T1–T5 execution topologies, Pareto selector |
| **Theme 7** | Storage Architecture & SQLite WAL | `platform/storage`, `tests/contract` | WAL crash recovery, batch rollback (INV-02, INV-13) |
| **Theme 8** | Heterogeneous Compute & GPU Admission| `platform/runtime/src/compute/`, `apps/daemon/src/api/tests/` | WorkloadAdmissionCoordinator VRAM feasibility |
| **Theme 9** | Vertical Specialist Workbenches | `workbenches/*/`, `apps/daemon/src/api/tests/workspaces/`, `quant/` | Workbench isolation, zero kernel-workbench leaks |
| **Theme 10**| Desktop UI & Operational Parity | `ui/desktop/scripts/canvas-smoke.mjs` (51 tests) | Strict zero-mock in UI, 3-tier hierarchy |

---

## 4. Execution Commands for Developers

```bash
# Ensure Cargo and Node are on PATH
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"

# 1. Run all 5 gates in sequence
cargo xtask gate

# 2. Run pure unit and contract tests (use --lib --tests to avoid Darwin dyld hangs)
cargo test -p custos-testkit -p custos-tests-contract --lib --tests --offline

# 3. Run individual crate tests
cargo test -p custos-domain --lib --offline
cargo test -p custos-core --lib --offline
cargo test -p custos-persistence --lib --offline
cargo test -p custos-runtime --lib --offline

# 4. Run Desktop UI canvas smoke invariants
node ui/desktop/scripts/canvas-smoke.mjs

# 5. Run end-to-end topology slice
cargo test -p custos-tests-e2e --test oi_topologies_slice --offline
```
