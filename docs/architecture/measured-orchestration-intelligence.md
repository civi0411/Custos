# Measured Orchestration Intelligence & S1 vs. Pinned Baseline Optimization (F11)

> SADE Master Specification — Phase 10 / F11 Architectural Blueprint  
> Governs: `custos-domain`, `custos-core`, `custos-runtime`, and Golden Acceptance Tests `G12` & `oi_topologies_slice.rs`.

---

## 1. Context & Motivation

In naive multi-agent frameworks, complex orchestration graphs and small-model routing are often marketed as "cost-reducing optimizations" without empirical verification. In practice:
1. **Hidden Economic Cost:** A smaller, cheaper model might use fewer direct API tokens, but frequently fails subtle requirements, requiring human interventions, manual triage, and multiple retries that result in net economic losses.
2. **Quality Degradation:** Routing to lightweight models without strict quality non-inferiority margins leads to degraded accuracy, broken tests, and regressions on edge cases.
3. **Premature Convergence / Guessing:** Fast heuristic judges that guess on ambiguous or partial outputs corrupt evidence ledgers and invalidate downstream deliberation.
4. **Retroactive Authority Escalation:** Naive DAG replanners rewrite committed nodes or grant extra permits on failed steps, violating authority invariants.
5. **Phantom Candidates & Zero Uncertainty:** Planners suggest absent or unconfigured executors, and represent unknown factors as `0` rather than explicit uncertainty ranges.

Phase 10 (F11) and the SuperAgent OI Refactoring transform Orchestration Intelligence (OI) and S1/S2 deliberation from scaffold into an honest, measured optimization substrate anchored by Golden Acceptance Test `G12` and the `oi_topologies_slice` suite:
> *"OI/S1 so pinned baseline, failures/human/recovery included | Không claim cheaper/quality-preserved nếu chưa đạt margin"*

---

## 2. Core Architecture & Components

```
                    [ DecisionSnapshot ]
                             │
            ┌────────────────┴────────────────┐
            ▼                                 ▼
   CandidateBuilder                     HardFilters
(Prunes Absent Executors)          (Rejects Unavailable /
            │                        Pinned Mismatch)
            ▼                                 │
     [ FastPathFilter ]                       ▼
(0-Token Ingress <1ms)               Admissibility Gate
            │                                 │
            ▼                                 ▼
        Estimator                  [ Pareto Multi-Objective ]
(Real Model Pricing Catalog +             Selector
 Cache Affinity + Uncertainty)     (U = w_q*Q - w_c*C - w_t*T)
            │                                 │
            └────────────────┬────────────────┘
                             ▼
                 [ Execution Topologies ]
    T0 Direct Pinned | T1 Pass-Through | T2 Dual-Judge
  T3 Speculative Verif | T4 Hierarchical | T5 Lead-Worker DAG
                             │
            ├────────────────┴────────────────┐
            ▼                                 ▼
   [ S1 Fast Judge ]                    [ Replanner ]
- Calibrated Confidence (>= 0.75)    - Isolates Failed Subgraph
- Clean Evidence -> Fast Pass        - Preserves Settled Nodes
- Ambiguous / Partial -> Abstain     - Preserves Committed Authority
- Uncompressed Evidence Preserved    - INV-03: Blocks Uncertain Retry
            │                                 │
            └────────────────┬────────────────┘
                             ▼
                  [ OptimizationEvaluator ]
                  - Pinned Baseline vs. Candidate Runs
                  - Total Economic Cost = API + Retries + Human Time
                  - Quality Non-Inferiority Margin Check
                  - Cost Reduction Margin Check
                  - Inconclusive on Small Sample Sizes (< 3)
```

---

## 3. The Tenets of Measured Optimization

### 3.1 Pinned Baseline as First-Class Default
- A single, sovereign, strong native coding agent (e.g., Claude 3.5 Sonnet / Governed native harness) is the reference standard (`RFC 003 §3`).
- Complex multi-agent topologies are only admitted when empirical evaluation proves both cost and quality benefits over the baseline.

### 3.2 Total Economic Cost Accounting
Economic cost is never calculated from raw model token pricing alone:
$$\text{Total Cost} = \text{Direct API Cost} + \text{Recovery / Retry Cost} + (\text{Human Intervention Seconds} \times \text{Hourly Rate} / 3600)$$
A run requiring 5 minutes of human debugging at $60/hr incurs $5.00 of labor cost, which instantly erases nominal cent-level token savings.

### 3.3 Acceptance Margins & Decision Outcomes
`OptimizationEvaluator::evaluate()` computes paired comparisons across benchmark slices:
- **`QualityInferior`:** Candidate quality drops by more than `quality_tolerance_margin` (default: 0.05). Optimization claim is strictly rejected.
- **`NotCheaper`:** When retries, recovery costs, and human time are included, total savings fall below `cost_reduction_margin` (default: 0.15). Optimization claim is strictly rejected.
- **`Inconclusive`:** Paired sample size is below `min_sample_size` (default: 3). Pinned baseline remains default.
- **`QualityPreservedAndCheaper`:** Both quality non-inferiority and cost reduction margins are empirically satisfied.

### 3.4 S1 Calibrated Confidence & Abstain
- Confidence is evaluated by `S1Judge` based on structured signals (exit codes, status, output presence).
- Clean execution achieves calibrated confidence $\ge 0.75$.
- Ambiguous or partial execution triggers explicit abstain ($< 0.75$), escalating to S2 without lossy compression.

### 3.5 Authority Invariant in Delta Replanning
- `Replanner::create_brief()` uses DAG dependency reachability to isolate affected nodes while preserving settled upstream nodes.
- `Replanner::apply_delta()` carries forward committed authority obligations (`obligations`), preventing retroactive rewriting of completed work or permissions.
- **INV-03 Guardrail:** If an outbox effect is in `Uncertain` status, automated blind retry is blocked.

### 3.6 Explicit Representation of Uncertainty
- `snapshot.unknowns` adds explicit token buffers (+500/unknown) and latency (+1000ms/unknown).
- Active uncertain or pending side-effects add recovery buffers (+800 tokens, +2500ms).
- Cost, latency, and quality are never estimated as `0` for unknown parameters.

---

## 4. Multi-Objective Pareto Selector & Real Model Pricing

### 4.1 Pareto Multi-Objective Optimization
The updated Selector (`platform/runtime/src/oi/selector.rs`) moves beyond naive first-admissible selection by implementing a multi-objective utility scoring function:
$$U = w_q \cdot Q - w_c \cdot \left(\frac{C}{C_{\text{baseline}}}\right) - w_t \cdot \left(\frac{T}{T_{\text{baseline}}}\right)$$
Where:
- $Q \in [0.0, 1.0]$: Estimated task outcome quality.
- $C$: Estimated total economic cost, normalized against the pinned baseline cost $C_{\text{baseline}}$.
- $T$: Estimated end-to-end latency in milliseconds, normalized against baseline latency $T_{\text{baseline}}$.
- $w_q = 0.50, w_c = 0.30, w_t = 0.20$: Default Pareto weight vector ensuring quality dominates economics while penalizing latency bloat.

### 4.2 Real Model Pricing Catalog
`platform/runtime/src/oi/estimator.rs` embeds standard cloud model pricing to calculate dollar cost per run:
- **Claude 3.5 Sonnet:** \$3.00 / MTok input, \$15.00 / MTok output.
- **Claude 3.5 Haiku:** \$0.80 / MTok input, \$4.00 / MTok output.
- **GPT-4o:** \$2.50 / MTok input, \$10.00 / MTok output.
- **GPT-4o-mini:** \$0.15 / MTok input, \$0.60 / MTok output.
- **Gemini 1.5 Pro:** \$1.25 / MTok input, \$5.00 / MTok output.
- **Gemini 1.5 Flash:** \$0.075 / MTok input, \$0.30 / MTok output.
- **Local Models (Ollama, llama.cpp):** \$0.00 direct API cost.

### 4.3 Prompt Cache Hit Affinity
To account for provider-side KV caching (Anthropic prompt caching, OpenAI prefix caching):
- Cache read operations apply a **0.1x multiplier** to input token cost.
- Cache creation/write operations apply a **1.25x multiplier** to input token cost.
- The context compiler structures prompts into cache-anchored prefixes (system prompt + workspace file tree + project rules) to maximize cache hit probability.

---

## 5. The 7 Execution Topologies Continuum (T0–T6)

Custos classifies execution topologies into a cohesive continuum:

| Topology | Code | Orchestration Model | When Admitted |
|---|:---:|---|---|
| **T0 Direct Pinned** | `T0DirectPinned` | Single sovereign model, zero orchestration overhead | Default baseline for standard interactive tasks |
| **T1 Pass-Through** | `T1PassThrough` | Direct streaming to model without intermediate planning | Ephemeral queries, conversational clarifications |
| **T2 Dual Judge** | `T2DualJudge` | Dual model cross-checking with consensus agreement | Critical security or high-risk domain changes |
| **T3 Speculative Verification** | `T3SpeculativeVerification` | Draft generated by fast model, verified by strong model | High-token generation with strict formal verifiers |
| **T4 Hierarchical Router** | `T4HierarchicalRouter` | Dynamic routing based on task scope and complexity | Multi-domain requests spanning multiple workbenches |
| **T5 Lead-Worker DAG** | `T5LeadWorker` | Lead planner decomposes task into sub-worker DAG with Kahn topological sort and coverage synthesis gate | Large multi-file refactoring, independent sub-modules |
| **T6 Dynamic Replan** | `T6DynamicReplan` | Adaptive runtime DAG replanning with settled node isolation | Long-running workflows with partial failures |

### Topology T5: Lead-Worker DAG Coordinator Deep-Dive
Implemented in `platform/runtime/src/oi/topologies/t5_lead_worker.rs` (271 lines):
1. **Decomposition:** Lead worker receives high-level task and emits a structured `LeadDecomposition` containing $N$ independent sub-tasks and their explicit dependencies.
2. **Kahn's Topological Sort:** Sub-tasks are ordered using Kahn's algorithm. If a cyclic dependency is detected, execution fails immediately with `DomainError::Validation("Cyclic dependency detected in Lead-Worker DAG")`.
3. **Parallel Execution:** Independent DAG tiers run in parallel across isolated worktree workspaces.
4. **Synthesis Gate:** Lead worker gathers all sub-worker outcomes. The synthesis is accepted only if the synthesis coverage score satisfies `CoverageThreshold` ($\ge 0.80$).

---

## 6. 0-Token FastPathFilter

Implemented in `platform/runtime/src/agent/fast_path.rs`:
- High-frequency deterministic requests (e.g. `help`, `ping`, `version`, `clear`, `workbenches`, `git status`, `git diff`, `git log`) bypass LLM inference entirely.
- Executes via high-performance regex matching in **< 1ms** with **\$0.00** API cost.
- Prevents token expenditure on trivial system commands and provides instantaneous terminal responsiveness.

---

## 7. Verification & Test Matrix

The OI engine is exhaustively validated by two core test suites:

### 7.1 Golden Acceptance Suite (G12)
`tests/e2e/tests/oi_measured_optimization_golden_test.rs`:
1. `test_g12_cannot_claim_cheaper_when_human_recovery_exceeds_token_savings`: Asserts rejection with `NotCheaper` when human time (\$5.00) exceeds token savings (\$0.10).
2. `test_g12_cannot_claim_quality_preserved_when_criteria_fail_margin`: Asserts rejection with `QualityInferior` when candidate criteria score drops from 1.0 to 0.70.
3. `test_g12_inconclusive_sample_size_preserves_pinned_baseline_default`: Asserts `Inconclusive` when $N < 3$, keeping baseline default.
4. `test_g12_approves_genuine_optimization_meeting_quality_and_cost_margins`: Asserts approval with `QualityPreservedAndCheaper` on genuine 50% savings and 100% quality.
5. `test_g12_s1_judge_passes_clean_evidence_above_threshold`: Asserts fast pass on exit code 0 and verified token matching.
6. `test_g12_s1_judge_abstains_on_uncertain_signal_preserving_evidence`: Asserts abstain on partial execution and uncompressed escalation to S2.
7. `test_g12_s1_judge_rejects_explicit_violations_with_detailed_breakdown`: Asserts clear failure reasons on non-zero exit codes.
8. `test_g12_replan_preserves_settled_nodes_and_obligations`: Asserts DAG delta reachability and obligation immutability.
9. `test_g12_replan_inv03_blocks_blind_retry_on_uncertain_outbox_effect`: Asserts `[INV-03 Guardrail]` blocks blind retry on Uncertain outbox items.
10. `test_g12_absent_executors_pruned_and_rejected_by_hard_filters`: Asserts unconfigured executors are excluded from candidate generation and rejected by hard filters.
11. `test_g12_hard_filters_enforce_model_pin_and_local_only`: Asserts model pin and local-only constraints.
12. `test_g12_estimator_explicit_uncertainty_representation`: Asserts non-zero token and latency buffers for unknown items and recovery side-effects.

### 7.2 OI Topologies Slice Suite
`tests/e2e/tests/oi_topologies_slice.rs`:
- **Theme 1:** Direct Pinned Topology (single model baseline, fallback behavior).
- **Theme 2:** Dual Judge Topology (consensus, dissenting vote escalation).
- **Theme 3:** Speculative Verification Topology (draft validation, rollback on rejection).
- **Theme 4:** Hierarchical Router Topology (complexity threshold routing).
- **Theme 5:** T5 Lead Worker DAG Topology (Kahn sort, cycle rejection, coverage synthesis gate).

---

## 8. Quality Gate Status

- **Golden Suite (G06–G12):** 40/40 PASS.
- **OI Topologies Slice (Themes 1–5):** PASS (100%).
- **Zero-I/O Domain Boundary:** `bash ./xtask/scripts/check_deps.sh --strict` PASS.
- **Workspace Clippy:** 0 warnings (`-D warnings`).
- **Desktop UI Bundle:** `npm run build` PASS.
- **Canvas Smoke:** `node tests/desktop/canvas-smoke.mjs` 42/42 PASS.
