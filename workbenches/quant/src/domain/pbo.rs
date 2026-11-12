//! Probability of Backtest Overfitting (PBO) via CSCV
//!
//! Implements Combinatorially Symmetric Cross-Validation (CSCV) to assess the
//! probability that the highest-ranking in-sample strategy variant underperforms
//! the median strategy out-of-sample (Bailey, Borwein, López de Prado & Zhu, 2014).

use custos_domain::DomainError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PboResult {
    pub available: bool,
    pub probability: f64,
    pub samples: usize,
    pub median_logit: f64,
    pub logits: Vec<f64>,
    pub is_overfitted: bool,
    pub status: String,
}

/// Helper to generate combinations of indices k out of n
fn generate_combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut result = Vec::new();
    let mut current = Vec::new();

    fn backtrack(
        start: usize,
        n: usize,
        k: usize,
        current: &mut Vec<usize>,
        result: &mut Vec<Vec<usize>>,
    ) {
        if current.len() == k {
            result.push(current.clone());
            return;
        }
        for i in start..n {
            current.push(i);
            backtrack(i + 1, n, k, current, result);
            current.pop();
        }
    }

    backtrack(0, n, k, &mut current, &mut result);
    result
}

/// Computes the Probability of Backtest Overfitting (PBO) from a candidate score matrix.
///
/// Matrix dimensions: M strategy candidates (rows) x T sub-periods (columns).
/// Requires at least 2 candidates and at least 4 evaluation sub-periods.
pub fn calculate_pbo_cscv(
    score_matrix: &[Vec<f64>],
    max_combinations: usize,
) -> Result<PboResult, DomainError> {
    if score_matrix.len() < 2 {
        return Ok(PboResult {
            available: false,
            probability: 0.0,
            samples: 0,
            median_logit: 0.0,
            logits: Vec::new(),
            is_overfitted: false,
            status: "insufficient_candidates".into(),
        });
    }

    let min_periods = score_matrix.iter().map(|row| row.len()).min().unwrap_or(0);
    if min_periods < 4 {
        return Ok(PboResult {
            available: false,
            probability: 0.0,
            samples: 0,
            median_logit: 0.0,
            logits: Vec::new(),
            is_overfitted: false,
            status: "insufficient_periods".into(),
        });
    }

    let num_candidates = score_matrix.len();
    let half_periods = min_periods / 2;

    let mut combinations = generate_combinations(min_periods, half_periods);
    if combinations.len() > max_combinations && max_combinations > 0 {
        // Deterministic sub-sampling of combinations
        let step = combinations.len() / max_combinations;
        let mut subsampled = Vec::with_capacity(max_combinations);
        for i in 0..max_combinations {
            subsampled.push(combinations[i * step].clone());
        }
        combinations = subsampled;
    }

    let mut logits = Vec::with_capacity(combinations.len());

    for train_indices in &combinations {
        let mut train_scores = vec![0.0; num_candidates];
        let mut test_scores = vec![0.0; num_candidates];

        let train_len = train_indices.len() as f64;
        let test_len = (min_periods - train_indices.len()) as f64;

        // Fast lookup for train membership
        let mut is_train = vec![false; min_periods];
        for &idx in train_indices {
            is_train[idx] = true;
        }

        for (cand_idx, row) in score_matrix.iter().enumerate() {
            let mut train_sum = 0.0;
            let mut test_sum = 0.0;
            for t in 0..min_periods {
                if is_train[t] {
                    train_sum += row[t];
                } else {
                    test_sum += row[t];
                }
            }
            train_scores[cand_idx] = train_sum / train_len;
            test_scores[cand_idx] = test_sum / test_len;
        }

        // Find the in-sample (train) winning candidate
        let mut is_winner = 0;
        let mut best_train_score = train_scores[0];
        for (i, &score) in train_scores.iter().enumerate().skip(1) {
            if score > best_train_score {
                best_train_score = score;
                is_winner = i;
            }
        }

        // Calculate rank of the IS winner in out-of-sample (test) scores
        let winner_test_score = test_scores[is_winner];
        let mut rank = 1;
        for (i, &score) in test_scores.iter().enumerate() {
            if i != is_winner && score < winner_test_score {
                rank += 1;
            }
        }

        let percentile = (rank as f64 / (num_candidates as f64 + 1.0)).clamp(1e-6, 1.0 - 1e-6);
        let logit = (percentile / (1.0 - percentile)).ln();
        logits.push(logit);
    }

    if logits.is_empty() {
        return Ok(PboResult {
            available: false,
            probability: 0.0,
            samples: 0,
            median_logit: 0.0,
            logits: Vec::new(),
            is_overfitted: false,
            status: "empty_logits".into(),
        });
    }

    let underperformed_count = logits.iter().filter(|&&l| l <= 0.0).count();
    let probability = (underperformed_count as f64 / logits.len() as f64).clamp(0.0, 1.0);

    let mut sorted_logits = logits.clone();
    sorted_logits.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_logit = sorted_logits[sorted_logits.len() / 2];

    let is_overfitted = probability > 0.50;

    Ok(PboResult {
        available: true,
        probability: (probability * 10000.0).round() / 10000.0,
        samples: logits.len(),
        median_logit: (median_logit * 10000.0).round() / 10000.0,
        logits: logits.into_iter().take(200).collect(),
        is_overfitted,
        status: if is_overfitted {
            "overfitted"
        } else {
            "robust"
        }
        .into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pbo_cscv_insufficient_inputs() {
        let matrix = vec![vec![1.0, 2.0]];
        let result = calculate_pbo_cscv(&matrix, 100).expect("handles small matrix");
        assert!(!result.available);
        assert_eq!(result.status, "insufficient_candidates");

        let matrix_few_periods = vec![vec![1.0, 2.0], vec![2.0, 3.0]];
        let result2 = calculate_pbo_cscv(&matrix_few_periods, 100).expect("handles few periods");
        assert!(!result2.available);
        assert_eq!(result2.status, "insufficient_periods");
    }

    #[test]
    fn test_pbo_cscv_calculation() {
        // 4 strategy candidates across 6 sub-periods
        let matrix = vec![
            vec![1.5, 2.0, 1.8, 1.2, 0.9, 1.4],    // Robust performer
            vec![2.5, 2.2, 2.9, -1.0, -0.5, -0.8], // Overfitted: good first half, bad second
            vec![0.5, 0.6, 0.4, 0.5, 0.7, 0.6],    // Consistent low performer
            vec![1.2, 1.1, 1.3, 1.4, 1.2, 1.3],    // Steady benchmark
        ];

        let result = calculate_pbo_cscv(&matrix, 500).expect("pbo calculated");
        assert!(result.available);
        assert_eq!(result.samples, 20); // 6 choose 3 = 20
        assert!(result.probability >= 0.0 && result.probability <= 1.0);
    }
}
