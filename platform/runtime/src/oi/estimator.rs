//! OI Cost Estimator with Model Pricing Catalog (RFC 004 §3.2)
//!
//! Estimates token count, cost (USD), and latency (ms) for each candidate harness,
//! using per-token API pricing and prompt-cache affinity discounts.
//!
//! Pricing catalog (as of Q4 2025 — update when providers change rates):
//!   claude-code / claude-sonnet-4-5: $3/MTok input, $15/MTok output, 80% cache discount
//!   claude-haiku-3-5:               $0.80/MTok input, $4/MTok output, 80% cache discount
//!   gpt-4o:                         $2.50/MTok input, $10/MTok output, 50% cache discount
//!   gpt-4o-mini:                    $0.15/MTok input, $0.60/MTok output, 50% cache discount
//!   gemini-2-flash:                 $0.075/MTok input, $0.30/MTok output, 0% cache discount
//!   goose / custos-agent:           $3.00/MTok input (delegates to claude-sonnet by default)
//!   local / mlx / llama / ollama:   $0 (compute cost not tracked)
//!
//! shortcut: actual prompt length is unknown at planning time; est_tokens is derived from
//!   snapshot complexity heuristics + harness baseline. Upgrade to real tokenizer when
//!   the runtime has access to the prompt text before dispatch.

use custos_domain::oi::{Candidate, DecisionSnapshot};

/// Per-model pricing constants (USD per million tokens).
struct ModelPricing {
    input_per_mtok: f32,
    output_per_mtok: f32,
    /// Fraction of input tokens eligible for cache discount (0.0 = none, 0.8 = Anthropic cache).
    cache_discount_fraction: f32,
    /// Baseline token budget (input + output) for a typical short task with no snapshot pressure.
    baseline_tokens: u64,
    /// Baseline latency ms for a typical short task.
    baseline_latency_ms: u64,
}

impl ModelPricing {
    fn estimate_cost(&self, tokens: u64, cache_hit_rate: f32) -> (f32, f32) {
        let tok_m = tokens as f32 / 1_000_000.0;
        // Assume 60% of budget is input, 40% output (heuristic).
        let input_tok_m = tok_m * 0.6;
        let output_tok_m = tok_m * 0.4;

        // Apply cache affinity discount on eligible input tokens
        let effective_input_rate = self.input_per_mtok
            * (1.0 - self.cache_discount_fraction * cache_hit_rate);

        let min_cost = effective_input_rate * input_tok_m * 0.5; // optimistic: short output
        let max_cost =
            effective_input_rate * input_tok_m + self.output_per_mtok * output_tok_m;

        (min_cost, max_cost)
    }
}

fn pricing_for(harness_id: &str) -> Option<ModelPricing> {
    // Match by prefix/substring — harness IDs may have suffixes like "-latest"
    let h = harness_id.to_lowercase();
    if h.contains("local") || h.contains("mlx") || h.contains("llama") || h.contains("ollama") {
        return Some(ModelPricing {
            input_per_mtok: 0.0,
            output_per_mtok: 0.0,
            cache_discount_fraction: 0.0,
            baseline_tokens: 2_000,
            baseline_latency_ms: 5_000,
        });
    }
    if h.contains("haiku") {
        return Some(ModelPricing {
            input_per_mtok: 0.80,
            output_per_mtok: 4.00,
            cache_discount_fraction: 0.80,
            baseline_tokens: 3_000,
            baseline_latency_ms: 3_500,
        });
    }
    if h.contains("claude") || h.contains("goose") || h.contains("custos-agent") {
        // claude-code, claude-sonnet-*, goose (which routes to Anthropic by default)
        return Some(ModelPricing {
            input_per_mtok: 3.00,
            output_per_mtok: 15.00,
            cache_discount_fraction: 0.80,
            baseline_tokens: 5_000,
            baseline_latency_ms: 8_000,
        });
    }
    if h.contains("gpt-4o-mini") || h.contains("gpt4o-mini") {
        return Some(ModelPricing {
            input_per_mtok: 0.15,
            output_per_mtok: 0.60,
            cache_discount_fraction: 0.50,
            baseline_tokens: 3_000,
            baseline_latency_ms: 3_000,
        });
    }
    if h.contains("gpt-4o") || h.contains("gpt4o") {
        return Some(ModelPricing {
            input_per_mtok: 2.50,
            output_per_mtok: 10.00,
            cache_discount_fraction: 0.50,
            baseline_tokens: 4_000,
            baseline_latency_ms: 6_000,
        });
    }
    if h.contains("gemini") {
        return Some(ModelPricing {
            input_per_mtok: 0.075,
            output_per_mtok: 0.30,
            cache_discount_fraction: 0.0,
            baseline_tokens: 3_000,
            baseline_latency_ms: 4_000,
        });
    }
    None
}

/// Fallback pricing for unrecognised harnesses (conservative estimate).
fn fallback_pricing() -> ModelPricing {
    ModelPricing {
        input_per_mtok: 2.00,
        output_per_mtok: 8.00,
        cache_discount_fraction: 0.0,
        baseline_tokens: 2_000,
        baseline_latency_ms: 4_000,
    }
}

pub struct Estimator;

impl Estimator {
    /// Estimates without snapshot context (used in tests and simple planners).
    pub fn estimate(candidates: &mut [Candidate]) {
        Self::estimate_with_snapshot(candidates, None);
    }

    /// Full estimation including snapshot-driven complexity surcharges and cache affinity.
    pub fn estimate_with_snapshot(
        candidates: &mut [Candidate],
        snapshot: Option<&DecisionSnapshot>,
    ) {
        for cand in candidates.iter_mut() {
            let pricing = pricing_for(&cand.harness_id).unwrap_or_else(fallback_pricing);

            // Base token budget from pricing catalog
            let mut tokens: u64 = pricing.baseline_tokens;
            let mut latency_ms: u64 = pricing.baseline_latency_ms;

            // Snapshot-driven complexity surcharges
            if let Some(snap) = snapshot {
                // Each unknown adds ~500 tokens and 1s latency (must be investigated)
                let unknowns = snap.unknowns.len() as u64;
                tokens += unknowns * 500;
                latency_ms += unknowns * 1_000;

                // Pending/uncertain effects: recovery buffer (agent must check state)
                let uncertain = (snap.uncertain_effects_count + snap.pending_effects_count) as u64;
                tokens += uncertain * 800;
                latency_ms += uncertain * 2_500;

                // Each acceptance criterion adds verification tokens
                let criteria = snap.criteria.len() as u64;
                tokens += criteria * 300;
                latency_ms += criteria * 600;
            }

            // Cache affinity: derive expected hit rate from snapshot predictability.
            // High unknown count → low cache reuse; low unknown count → high cache reuse.
            let cache_hit_rate = match snapshot {
                None => 0.5, // assume moderate cache hit without snapshot info
                Some(snap) => {
                    let pressure = snap.unknowns.len()
                        + snap.uncertain_effects_count
                        + snap.pending_effects_count;
                    // 0 pressure → 0.9 hit rate; 10+ pressure → 0.1 hit rate
                    (1.0_f32 - (pressure as f32 / 10.0).min(1.0)) * 0.8 + 0.1
                }
            };

            let (cost_min, cost_max) = pricing.estimate_cost(tokens, cache_hit_rate);

            cand.est_tokens = tokens;
            cand.est_latency_ms = latency_ms;
            cand.est_cost_usd_min = cost_min;
            cand.est_cost_usd_max = cost_max;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::oi::{Candidate, ExecutionTopology};

    fn cand(id: &str) -> Candidate {
        Candidate {
            topology: ExecutionTopology::T1SingleWorker,
            harness_id: id.into(),
            est_cost_usd_min: 0.0,
            est_cost_usd_max: 0.0,
            est_tokens: 0,
            est_latency_ms: 0,
            risks: vec![],
        }
    }

    #[test]
    fn local_model_has_zero_cost() {
        let mut candidates = vec![cand("local-llama-3")];
        Estimator::estimate(&mut candidates);
        assert_eq!(candidates[0].est_cost_usd_max, 0.0);
        assert_eq!(candidates[0].est_cost_usd_min, 0.0);
    }

    #[test]
    fn claude_code_more_expensive_than_haiku() {
        let mut candidates = vec![cand("claude-code"), cand("claude-haiku-3-5")];
        Estimator::estimate(&mut candidates);
        assert!(
            candidates[0].est_cost_usd_max > candidates[1].est_cost_usd_max,
            "claude-code should cost more than haiku"
        );
    }

    #[test]
    fn more_unknowns_raises_tokens_and_latency() {
        use custos_domain::oi::DecisionSnapshot;
        let snap_empty = DecisionSnapshot {
            task_id: "t".into(),
            unknowns: vec![],
            uncertain_effects_count: 0,
            pending_effects_count: 0,
            criteria: vec![],
            ..Default::default()
        };
        let snap_complex = DecisionSnapshot {
            unknowns: vec!["u1".into(), "u2".into(), "u3".into()],
            ..snap_empty.clone()
        };

        let mut a = vec![cand("goose")];
        let mut b = vec![cand("goose")];
        Estimator::estimate_with_snapshot(&mut a, Some(&snap_empty));
        Estimator::estimate_with_snapshot(&mut b, Some(&snap_complex));

        assert!(b[0].est_tokens > a[0].est_tokens);
        assert!(b[0].est_latency_ms > a[0].est_latency_ms);
    }

    #[test]
    fn cache_affinity_reduces_cost_for_simple_tasks() {
        use custos_domain::oi::DecisionSnapshot;
        let snap_simple = DecisionSnapshot {
            task_id: "t".into(),
            unknowns: vec![],
            uncertain_effects_count: 0,
            pending_effects_count: 0,
            criteria: vec![],
            ..Default::default()
        };
        let snap_complex = DecisionSnapshot {
            unknowns: (0..8).map(|i| format!("u{i}")).collect(),
            ..snap_simple.clone()
        };

        let mut a = vec![cand("claude-code")];
        let mut b = vec![cand("claude-code")];
        Estimator::estimate_with_snapshot(&mut a, Some(&snap_simple));
        Estimator::estimate_with_snapshot(&mut b, Some(&snap_complex));

        // Complex tasks have more tokens AND lower cache hit rate → higher cost
        assert!(b[0].est_cost_usd_max > a[0].est_cost_usd_max);
    }
}
