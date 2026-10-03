# RFC 003: Orchestration Intelligence (OI) Decision Flow

**Status:** Draft  
**Target Crates:** `custos-runtime`, `custos-core`, `custos-packs`  

## 1. Problem Statement
Orchestration Intelligence (OI) should not be a monolithic, untestable AI black box that dictates the entire execution. It must be a structured pipeline that ingests constraints, evaluates candidates, proposes a strategy, and submits it to a deterministic compiler. A strong native coding agent must always be the baseline.

## 2. The Decision Pipeline

OI operates through a strict sequence of immutable data structures:

### A. DecisionSnapshot
- **Owner:** Domain & Kernel.
- **Content:** The current state of reality. Task version, source/policy versions, capability versions, current criteria, remaining budget, pending effects, and unknowns.
- **Rule:** OI reads this. It does NOT create its own source of truth.

### B. StrategyProposal
- **Owner:** OI.
- **Content:** The proposed approach (e.g., Direct model vs. One worker vs. DAG). Contains assumptions, workspace/context strategy, candidate models/harnesses, expected cost/latency ranges, and the reasoning behind the choice.

### C. Admissibility & Compilation
- **Owner:** Compiler / Core.
- **Rule:** `StrategyProposal` is NOT an executable workflow. The Compiler checks the proposal against hard constraints (Model Pin, Egress rules, Budget). If admissible, it generates a `WorkflowRevision` (the DAG with types, read/write sets, and obligations).

### D. NodePlacement & DecisionRecord
- **Owner:** OI proposes, Runtime confirms.
- **Content:** `NodePlacement` maps a worker role to a selected backend, acquires a workspace lease, and reserves a budget slice. 
- **Ledger:** The `DecisionRecord` is appended to the ledger, logging alternatives considered, the chosen route, reasons, and the active policy version.

## 3. Re-planning & The Baseline

- **Baseline Fallback:** A single, strong native coding agent (e.g., Claude 3.5 Sonnet direct) is always the baseline candidate. Complex DAG topologies are only chosen if the baseline cannot satisfy localization, reproducibility, or context window constraints.
- **Replan Trigger:** Re-planning is only triggered when assumptions fail (e.g., unexpected test failure, missing evidence) or external state changes. It produces a `ReplanBrief` containing the delta from the old plan, failed assumptions, and evidence refs.

## 4. Fixtures for the Three Packs
We will define static `TaskContract` fixtures for the three main domain packs (Engineering, Research, Assistant) to test this OI pipeline deterministically before attaching real LLMs.
