//! S1 Calibration & Confidence Scoring (Brier Score, ECE, Abstain Threshold)
//!
//! Enforces RFC 005 calibration: S1 judgments must possess calibrated confidence.
//! If confidence is below 0.75, the S1 fabric must abstain and escalate to S2.

/// Default confidence threshold below which S1 abstains and escalates to S2.
pub const ABSTAIN_CONFIDENCE_THRESHOLD: f64 = 0.75;

/// Computes the Brier score for a set of probability predictions and binary outcomes.
///
/// Brier Score = (1 / N) * sum((f_i - o_i)^2)
/// Lower is better; 0.0 represents perfect calibration and accuracy.
pub fn calculate_brier_score(predictions: &[(f64, bool)]) -> f64 {
    if predictions.is_empty() {
        return 0.0;
    }
    let sum_sq_err: f64 = predictions
        .iter()
        .map(|(conf, outcome)| {
            let actual = if *outcome { 1.0 } else { 0.0 };
            let clamped = conf.clamp(0.0, 1.0);
            (clamped - actual).powi(2)
        })
        .sum();
    sum_sq_err / (predictions.len() as f64)
}

/// Computes Expected Calibration Error (ECE) across M equal-width bins.
///
/// ECE = sum_m (|B_m| / N) * |acc(B_m) - conf(B_m)|
pub fn calculate_ece(predictions: &[(f64, bool)], num_bins: usize) -> f64 {
    if predictions.is_empty() || num_bins == 0 {
        return 0.0;
    }

    let n = predictions.len() as f64;
    let bin_width = 1.0 / (num_bins as f64);
    let mut ece = 0.0;

    for i in 0..num_bins {
        let lower = i as f64 * bin_width;
        let upper = if i == num_bins - 1 {
            1.0001 // Include 1.0 in last bin
        } else {
            (i + 1) as f64 * bin_width
        };

        let in_bin: Vec<&(f64, bool)> = predictions
            .iter()
            .filter(|(conf, _)| {
                let c = conf.clamp(0.0, 1.0);
                c >= lower && c < upper
            })
            .collect();

        if in_bin.is_empty() {
            continue;
        }

        let bin_size = in_bin.len() as f64;
        let avg_conf = in_bin.iter().map(|(c, _)| c.clamp(0.0, 1.0)).sum::<f64>() / bin_size;
        let avg_acc = in_bin
            .iter()
            .map(|(_, o)| if *o { 1.0 } else { 0.0 })
            .sum::<f64>()
            / bin_size;

        ece += (bin_size / n) * (avg_acc - avg_conf).abs();
    }

    ece
}

/// Calibrator for S1 confidence ratings.
#[derive(Debug, Clone)]
pub struct ConfidenceCalibrator {
    threshold: f64,
}

impl Default for ConfidenceCalibrator {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfidenceCalibrator {
    /// Creates a calibrator with the default threshold (0.75).
    pub fn new() -> Self {
        Self {
            threshold: ABSTAIN_CONFIDENCE_THRESHOLD,
        }
    }

    /// Creates a calibrator with a custom threshold.
    pub fn with_threshold(threshold: f64) -> Self {
        Self {
            threshold: threshold.clamp(0.0, 1.0),
        }
    }

    /// Returns true if confidence is below the threshold and S1 must abstain.
    pub fn should_abstain(&self, confidence: f64) -> bool {
        confidence < self.threshold
    }

    /// Returns the active abstain threshold.
    pub fn threshold(&self) -> f64 {
        self.threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brier_score_perfect() {
        let sample = vec![(1.0, true), (0.0, false), (1.0, true)];
        let score = calculate_brier_score(&sample);
        assert!((score - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_brier_score_worst() {
        let sample = vec![(0.0, true), (1.0, false)];
        let score = calculate_brier_score(&sample);
        assert!((score - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_ece_calculation() {
        let sample = vec![
            (0.9, true),
            (0.85, true),
            (0.2, false),
            (0.1, false),
        ];
        let ece = calculate_ece(&sample, 5);
        assert!(ece >= 0.0 && ece <= 1.0);
    }

    #[test]
    fn test_abstain_threshold() {
        let cal = ConfidenceCalibrator::new();
        assert!(cal.should_abstain(0.50));
        assert!(cal.should_abstain(0.749));
        assert!(!cal.should_abstain(0.750));
        assert!(!cal.should_abstain(0.90));
    }
}
