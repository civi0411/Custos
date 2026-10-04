# Orchestration Intelligence (OI) Evaluation Suite

This directory documents the empirical evaluations comparing Custos Orchestration Intelligence (OI) against the unguided sovereign single-agent baseline (Claude Code) across three canonical domain packs.

## Benchmark Matrices

| Domain Pack | Canonical Task | Baseline Strategy | OI Strategy | Parallelism | Assurance / Guarantee |
|---|---|---|---|---|---|
| **Engineering** | Fix Workspace Compile & Test Errors | NativeBaseline (single worker) | T4 Worktree / T6 Repair Loop | 2x | Worktree branch isolation, clean merge guarantee |
| **Research** | Multi-Source Literature Synthesis | NativeBaseline (sequential reading) | T3 Read Fan-Out | 3x | CoverageThreshold join policy (0.75 coverage oracle) |
| **Assistant** | Daily Agenda & Inbox Triage | NativeBaseline (agent bootstrap) | T0 Direct Execution | 1x | Sub-50ms deterministic instant execution |

## Evaluation Criteria
1. **Cost Efficiency**: Token budget estimation and elimination of redundant context reloading.
2. **Execution Latency**: Speedup via read fan-out and worktree isolation.
3. **Assurance & Safety**: Elimination of write-set collisions and enforcement of verifier oracles.
