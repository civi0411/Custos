use std::time::{Duration, SystemTime};

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use reqwest::header::{HeaderMap, RETRY_AFTER};
use serde_json::Value;

/// Hard cap on retry delays we'll honor from remote responses. A malformed
/// 429 with `retry_after_seconds: 1e30` (or a far-future HTTP-date) should
/// degrade to "no retry hint" rather than freeze the agent or panic when
/// converting to `Duration`. One hour is well past any legitimate
/// rate-limit window.
pub const MAX_RETRY_AFTER_SECS: f64 = 3600.0;

/// Extract a retry delay from a 429 response. Prefers the body's
/// `error.metadata.retry_after_seconds` (OpenRouter shape, more precise than
/// the integer header) and falls back to the RFC 7231 `Retry-After` header
/// in either its delay-seconds form or its HTTP-date form.
pub fn extract_retry_after(headers: &HeaderMap, payload: Option<&Value>) -> Option<Duration> {
    if let Some(secs) = payload
        .and_then(|p| p.get("error"))
        .and_then(|e| e.get("metadata"))
        .and_then(|m| m.get("retry_after_seconds"))
        .and_then(|v| v.as_f64())
    {
        if let Some(d) = duration_from_finite_secs(secs) {
            return Some(d);
        }
    }

    headers
        .get(RETRY_AFTER)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| parse_retry_after_header(s.trim()))
}

/// Convert a finite, non-negative, in-range seconds value to a `Duration`.
/// Returns `None` for NaN, negative, infinite, or absurdly large inputs —
/// `Duration::from_secs_f64` panics on the latter.
pub fn duration_from_finite_secs(secs: f64) -> Option<Duration> {
    if !secs.is_finite() || secs < 0.0 {
        return None;
    }
    let clamped = secs.min(MAX_RETRY_AFTER_SECS);
    Some(Duration::from_secs_f64(clamped))
}

/// Parse `Retry-After` per RFC 7231 §7.1.3: either a non-negative integer
/// number of seconds, or an HTTP-date (interpreted as the absolute time at
/// which the request may be retried). A past date is honored as "retry
/// now" (`Duration::ZERO`) rather than dropped — clock skew or near-now
/// timestamps plus network latency commonly produce an HTTP-date that is
/// already in the past, and falling back to exponential backoff would
/// add unnecessary delay against an explicit server hint.
pub fn parse_retry_after_header(value: &str) -> Option<Duration> {
    if let Ok(secs) = value.parse::<u64>() {
        return duration_from_finite_secs(secs as f64);
    }
    let target = parse_http_date(value)?;
    let delay = target
        .duration_since(SystemTime::now())
        .unwrap_or(Duration::ZERO);
    duration_from_finite_secs(delay.as_secs_f64())
}

/// Parse the three HTTP-date forms RFC 7231 §7.1.1.1 requires recipients to
/// accept: IMF-fixdate (`Sun, 06 Nov 1994 08:49:37 GMT`), the obsolete RFC 850
/// form (`Sunday, 06-Nov-94 08:49:37 GMT`), and asctime (`Sun Nov  6 08:49:37
/// 1994`). All three are interpreted as GMT.
pub fn parse_http_date(value: &str) -> Option<SystemTime> {
    let value = value.trim();
    if let Ok(dt) = DateTime::parse_from_rfc2822(value) {
        return Some(SystemTime::from(dt));
    }
    if let Some(body) = value.strip_suffix(" GMT") {
        if let Ok(naive) = NaiveDateTime::parse_from_str(body, "%A, %d-%b-%y %H:%M:%S") {
            return Some(SystemTime::from(Utc.from_utc_datetime(&naive)));
        }
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(value, "%a %b %e %H:%M:%S %Y") {
        return Some(SystemTime::from(Utc.from_utc_datetime(&naive)));
    }
    None
}
