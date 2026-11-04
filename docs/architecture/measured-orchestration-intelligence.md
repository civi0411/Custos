# Measured Orchestration Intelligence & S1 vs. Pinned Baseline Optimization (F11)

> SADE Master Specification — Phase 10 / F11 Architectural Blueprint
> Governs: `custos-domain`, `custos-core`, `custos-runtime`, and Golden Acceptance Test `G12`.

---

## 1. Context & Motivation

In naive multi-agent frameworks, complex orchestration graphs and small-model routing are often marketed as "cost-reducing optimizations" without empirical verification. In practice:
1. **Hidden Economic Cost:** A smaller, cheaper model might use fewer direct API tokens, but frequently fails subtle requirements, requiring human interventions, manual triage, and multiple retries that result in net economic losses.
2. **Quality Degradation:** Routing to lightweight models without strict quality non-inferiority margins leads to degraded accuracy, broken tests, and regressions on edge cases.
3. **Premature Convergence / Guessing:** Fast heuristic judges that guess on ambiguous or partial outputs corrupt evidence ledgers and invalidate downstream deliberation.
4. **Retroactive Authority Escalation:** Naive DAG replanners rewrite committed nodes or grant extra permits on failed steps, violating authority invariants.
5. **Phantom Candidates & Zero Uncertainty:** Planners suggest absent or unconfigured executors, and represent unknown factors as `0` rather than explicit uncertainty ranges.

Phase 10 (F11) transforms Orchestration Intelligence (OI) and S1/S2 deliberation from scaffold into an honest, measured optimization substrate anchored by Golden Acceptance Test `G12`:
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
        Estimator                             ▼
(Explicit Uncertainty Ranges:          Admissibility Gate
 Unknowns + Recovery + Criteria)
            │
            ▼
        [ Deliberation & Selection ]
        - Direct Pinned Baseline (Default)
        - Multi-Node DAG Candidate
            │
            ├─────────────────────────────────────────────┐
            ▼                                             ▼
   [ S1 Fast Judge ]                             [ Replanner ]
- Calibrated Confidence (>= 0.75)             - Isolates Failed Subgraph
- Clean Evidence -> Fast Pass                 - Preserves Settled Nodes
- Ambiguous / Partial -> Abstain to S2        - Preserves Committed Authority
- Uncompressed Evidence Preserved             - INV-03: Blocks Uncertain Retry
            │                                             │
            └──────────────────────┬──────────────────────┘
                                   ▼
                    [ OptimizationEvaluator ]
                    - Pinned Baseline vs. Candidate Runs
                    - Total Economic Cost = API + Retries + Human Time
                    - Quality Non-Inferiority Margin Check
                    - Cost Reduction Margin Check
                    - Inconclusive on Small Sample Sizes
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

## 4. Verification & Golden Acceptance Suite (G12)

Implemented in `tests/e2e/tests/oi_measured_optimization_golden_test.rs`:
1. `test_g12_cannot_claim_cheaper_when_human_recovery_exceeds_token_savings`: Asserts rejection with `NotCheaper` when human time ($5.00) exceeds token savings ($0.10).
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

---

## 5. Quality Gate Status

- **Golden Suite (G06, G07, G08, G09, G10, G11, G12):** 40/40 PASS.
- **Zero-I/O Domain Boundary:** `bash ./scripts/check_deps.sh --strict` PASS.
- **Workspace Clippy:** 0 warnings (`-D warnings`).
- **Desktop UI Bundle:** `npm run build` PASS (992ms).
- **Canvas Smoke:** `node tools/canvas-smoke.mjs` 42/42 PASS.
