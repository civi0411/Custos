//! Deterministic Market Evidence Quality Gate (Zero-Mock Data Truthfulness)
//!
//! Assesses coverage, freshness, and cross-source conflict (<= 2% tolerance)
//! before quantitative evidence enters an Agent's reasoning context or trade gate.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceObservation {
    pub evidence_id: String,
    pub metric: String,
    pub source: String,
    pub value: Option<f64>,
    pub as_of_ts: i64,
    pub freshness_limit_secs: i64,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub quality_flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceQualitySummary {
    pub coverage_ratio: f64,
    pub freshness_ratio: f64,
    pub conflict_ratio: f64,
    pub quality_score: f64,
    pub readiness: String,
    pub confidence_cap: f64,
    pub covered_metrics: Vec<String>,
    pub missing_metrics: Vec<String>,
    pub conflicting_metrics: Vec<String>,
    pub stale_evidence_ids: Vec<String>,
    pub invalid_evidence_ids: Vec<String>,
}

fn has_flag(flags: &[String], targets: &[&str]) -> bool {
    flags
        .iter()
        .any(|f| targets.contains(&f.trim().to_ascii_lowercase().as_str()))
}

fn is_valid(obs: &EvidenceObservation) -> bool {
    if has_flag(
        &obs.quality_flags,
        &["invalid", "missing", "error", "unavailable"],
    ) {
        return false;
    }
    matches!(obs.value, Some(v) if v.is_finite())
}

fn is_fresh(obs: &EvidenceObservation, retrieved_at_ts: i64) -> bool {
    if has_flag(&obs.quality_flags, &["stale", "expired"]) {
        return false;
    }
    let age = (retrieved_at_ts - obs.as_of_ts).max(0);
    age <= obs.freshness_limit_secs.max(0)
}

fn has_cross_source_conflict(items: &[&EvidenceObservation], tolerance: f64) -> bool {
    if items
        .iter()
        .any(|o| has_flag(&o.quality_flags, &["conflict", "conflicting"]))
    {
        return true;
    }
    let valid: Vec<&EvidenceObservation> = items.iter().copied().filter(|o| is_valid(o)).collect();
    let distinct_sources: BTreeSet<String> = valid
        .iter()
        .map(|o| o.source.trim().to_ascii_lowercase())
        .collect();
    if distinct_sources.len() < 2 {
        return false;
    }
    let nums: Vec<f64> = valid.iter().filter_map(|o| o.value).collect();
    if nums.len() < 2 {
        return false;
    }
    let low = nums.iter().copied().fold(f64::INFINITY, f64::min);
    let high = nums.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let denom = low.abs().max(high.abs()).max(1e-12);
    (high - low).abs() / denom > tolerance
}

/// Deterministic quality assessment:
/// `quality_score = 0.50 * coverage + 0.35 * freshness + 0.15 * (1 - conflict)`
/// Missing keys/metrics never synthesize values; they reduce readiness and cap confidence.
pub fn assess_evidence_quality(
    observations: &[EvidenceObservation],
    required_metrics: &[String],
    retrieved_at_ts: i64,
) -> EvidenceQualitySummary {
    let mut by_metric: BTreeMap<String, Vec<&EvidenceObservation>> = BTreeMap::new();
    for obs in observations {
        by_metric.entry(obs.metric.clone()).or_default().push(obs);
    }

    let required: Vec<String> = if !required_metrics.is_empty() {
        required_metrics.to_vec()
    } else {
        let flagged: Vec<String> = observations
            .iter()
            .filter(|o| o.required)
            .map(|o| o.metric.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if !flagged.is_empty() {
            flagged
        } else {
            by_metric.keys().cloned().collect()
        }
    };

    let mut invalid_evidence_ids: Vec<String> = observations
        .iter()
        .filter(|o| !is_valid(o))
        .map(|o| o.evidence_id.clone())
        .collect();
    invalid_evidence_ids.sort();

    let mut covered_metrics = Vec::new();
    let mut missing_metrics = Vec::new();
    let mut fresh_count = 0usize;
    let mut stale_evidence_ids = Vec::new();
    let mut conflicting_metrics = Vec::new();

    for metric in &required {
        let items = by_metric.get(metric).map(Vec::as_slice).unwrap_or(&[]);
        let valid_items: Vec<&EvidenceObservation> =
            items.iter().copied().filter(|o| is_valid(o)).collect();
        if valid_items.is_empty() {
            missing_metrics.push(metric.clone());
            continue;
        }
        covered_metrics.push(metric.clone());
        let mut any_fresh = false;
        for item in &valid_items {
            if is_fresh(item, retrieved_at_ts) {
                any_fresh = true;
            } else {
                stale_evidence_ids.push(item.evidence_id.clone());
            }
        }
        if any_fresh {
            fresh_count += 1;
        }
        if has_cross_source_conflict(items, 0.02) {
            conflicting_metrics.push(metric.clone());
        }
    }
    stale_evidence_ids.sort();

    let denom = required.len() as f64;
    let coverage_ratio = if denom > 0.0 {
        covered_metrics.len() as f64 / denom
    } else {
        0.0
    };
    let freshness_ratio = if !covered_metrics.is_empty() {
        fresh_count as f64 / covered_metrics.len() as f64
    } else {
        0.0
    };
    let conflict_ratio = if denom > 0.0 {
        conflicting_metrics.len() as f64 / denom
    } else {
        0.0
    };

    let quality_score =
        (0.50 * coverage_ratio + 0.35 * freshness_ratio + 0.15 * (1.0 - conflict_ratio))
            .clamp(0.0, 1.0);

    let (readiness, confidence_cap) = if denom == 0.0 || coverage_ratio < 0.35 {
        ("none".to_string(), 35.0)
    } else if quality_score < 0.60 || !conflicting_metrics.is_empty() {
        ("low".to_string(), 55.0)
    } else if quality_score < 0.85 {
        ("medium".to_string(), 75.0)
    } else {
        ("high".to_string(), 100.0)
    };

    EvidenceQualitySummary {
        coverage_ratio,
        freshness_ratio,
        conflict_ratio,
        quality_score,
        readiness,
        confidence_cap,
        covered_metrics,
        missing_metrics,
        conflicting_metrics,
        stale_evidence_ids,
        invalid_evidence_ids,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evidence_quality_conflict_caps_confidence() {
        let obs = vec![
            EvidenceObservation {
                evidence_id: "e1".into(),
                metric: "btc_price".into(),
                source: "binance".into(),
                value: Some(65_000.0),
                as_of_ts: 1000,
                freshness_limit_secs: 60,
                required: true,
                quality_flags: vec![],
            },
            EvidenceObservation {
                evidence_id: "e2".into(),
                metric: "btc_price".into(),
                source: "coinbase".into(),
                value: Some(67_500.0), // >2% divergence
                as_of_ts: 1000,
                freshness_limit_secs: 60,
                required: true,
                quality_flags: vec![],
            },
        ];
        let summary = assess_evidence_quality(&obs, &["btc_price".into()], 1010);
        assert_eq!(summary.conflicting_metrics, vec!["btc_price"]);
        assert_eq!(summary.readiness, "low");
        assert_eq!(summary.confidence_cap, 55.0);
    }

    #[test]
    fn test_evidence_quality_missing_metric_lowers_coverage() {
        let obs = vec![EvidenceObservation {
            evidence_id: "e1".into(),
            metric: "price".into(),
            source: "yahoo".into(),
            value: Some(500.0),
            as_of_ts: 1000,
            freshness_limit_secs: 300,
            required: true,
            quality_flags: vec![],
        }];
        let summary =
            assess_evidence_quality(&obs, &["price".into(), "short_interest".into()], 1050);
        assert!((summary.coverage_ratio - 0.5).abs() < 1e-6);
        assert_eq!(summary.missing_metrics, vec!["short_interest"]);
        assert_eq!(summary.readiness, "medium");
        assert_eq!(summary.confidence_cap, 75.0);
    }
}
