//! Non-destructive OSC 7501 stateful PTY scanner and status precedence resolver.
//!
//! Tracks `\x1b]7501;<payload>\x07` (or ST `\x1b\\`) across chunk boundaries,
//! responds to `OSC 7501 ; ?` probes, and resets on OSC 133 A or RIS (`\x1bc`).

use std::time::{Duration, Instant};

const MAX_BODY_BYTES: usize = 4096;
const MAX_CARRY_BYTES: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Osc7501Status {
    Idle,
    Working,
    WaitingApproval,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Osc7501Event {
    Probe,
    StatusChanged(Osc7501Status),
    PromptReset,
}

#[derive(Debug, Default)]
pub struct Osc7501StatefulScanner {
    carry: Vec<u8>,
    in_osc: bool,
    osc_buf: Vec<u8>,
    pub current_status: Option<Osc7501Status>,
}

impl Osc7501StatefulScanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn scan_chunk(&mut self, chunk: &[u8]) -> Vec<Osc7501Event> {
        let mut events = Vec::new();
        let mut data = Vec::with_capacity(self.carry.len() + chunk.len());
        data.extend_from_slice(&self.carry);
        data.extend_from_slice(chunk);
        self.carry.clear();

        let mut i = 0;
        while i < data.len() {
            if self.in_osc {
                if data[i] == 0x07 {
                    // BEL terminator
                    self.finish_osc(&mut events);
                    i += 1;
                    continue;
                }
                if data[i] == 0x1b {
                    if i + 1 >= data.len() {
                        self.carry.push(0x1b);
                        break;
                    }
                    if data[i + 1] == b'\\' {
                        // ST terminator (\x1b\\)
                        self.finish_osc(&mut events);
                        i += 2;
                        continue;
                    }
                }
                if self.osc_buf.len() < MAX_BODY_BYTES {
                    self.osc_buf.push(data[i]);
                } else {
                    // Drop oversized OSC sequence safely
                    self.in_osc = false;
                    self.osc_buf.clear();
                }
                i += 1;
                continue;
            }

            if data[i] == 0x1b {
                if i + 1 >= data.len() {
                    self.carry.push(0x1b);
                    break;
                }
                if data[i + 1] == b'c' {
                    // RIS full terminal reset (\x1bc)
                    self.current_status = Some(Osc7501Status::Idle);
                    events.push(Osc7501Event::PromptReset);
                    i += 2;
                    continue;
                }
                if data[i + 1] == b']' {
                    self.in_osc = true;
                    self.osc_buf.clear();
                    i += 2;
                    continue;
                }
            }
            i += 1;
        }

        if self.carry.len() > MAX_CARRY_BYTES {
            self.carry.clear();
        }
        events
    }

    fn finish_osc(&mut self, events: &mut Vec<Osc7501Event>) {
        self.in_osc = false;
        let raw = String::from_utf8_lossy(&self.osc_buf).trim().to_string();
        self.osc_buf.clear();

        if raw == "133;A" || raw.starts_with("133;A;") {
            self.current_status = Some(Osc7501Status::Idle);
            events.push(Osc7501Event::PromptReset);
            return;
        }

        if let Some(payload) = raw.strip_prefix("7501;") {
            let trimmed = payload.trim();
            if trimmed == "?" {
                events.push(Osc7501Event::Probe);
                return;
            }
            let status = match trimmed.to_ascii_lowercase().as_str() {
                "idle" => Osc7501Status::Idle,
                "working" | "busy" | "running" => Osc7501Status::Working,
                "approval" | "waiting_approval" | "permission" => Osc7501Status::WaitingApproval,
                other => Osc7501Status::Unknown(other.to_string()),
            };
            if self.current_status.as_ref() != Some(&status) {
                self.current_status = Some(status.clone());
                events.push(Osc7501Event::StatusChanged(status));
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StatusPrecedencePolicy {
    pub stale_hook_secs: u64,
}

impl Default for StatusPrecedencePolicy {
    fn default() -> Self {
        Self {
            stale_hook_secs: 120,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedAgentStatusSource {
    ManagedHook,
    Osc7501,
}

pub struct StatusPrecedenceResolver;

impl StatusPrecedenceResolver {
    /// `ManagedHook` takes precedence over `OSC 7501` unless silent past `stale_hook_secs`.
    pub fn resolve(
        policy: StatusPrecedencePolicy,
        hook_status: Option<(Osc7501Status, Instant)>,
        osc_status: Option<Osc7501Status>,
        now: Instant,
    ) -> Option<(Osc7501Status, ResolvedAgentStatusSource)> {
        if let Some((status, updated_at)) = hook_status {
            if now.duration_since(updated_at) <= Duration::from_secs(policy.stale_hook_secs) {
                return Some((status, ResolvedAgentStatusSource::ManagedHook));
            }
        }
        osc_status.map(|s| (s, ResolvedAgentStatusSource::Osc7501))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanner_handles_chunk_boundaries_probes_and_resets() {
        let mut scanner = Osc7501StatefulScanner::new();
        // Split ESC ] 7501;working BEL across two chunks
        assert!(scanner.scan_chunk(b"hello \x1b").is_empty());
        let ev1 = scanner.scan_chunk(b"]7501;working\x07world");
        assert_eq!(
            ev1,
            vec![Osc7501Event::StatusChanged(Osc7501Status::Working)]
        );

        // Probe via ST (\x1b\\) split at ESC
        assert!(scanner.scan_chunk(b"\x1b]7501;?\x1b").is_empty());
        let ev2 = scanner.scan_chunk(b"\\");
        assert_eq!(ev2, vec![Osc7501Event::Probe]);

        // OSC 133;A resets to Idle
        let ev3 = scanner.scan_chunk(b"\x1b]133;A\x07");
        assert_eq!(ev3, vec![Osc7501Event::PromptReset]);
        assert_eq!(scanner.current_status, Some(Osc7501Status::Idle));

        // RIS (\x1bc) also resets
        let _ = scanner.scan_chunk(b"\x1b]7501;approval\x07");
        let ev4 = scanner.scan_chunk(b"\x1bc");
        assert_eq!(ev4, vec![Osc7501Event::PromptReset]);
        assert_eq!(scanner.current_status, Some(Osc7501Status::Idle));
    }

    #[test]
    fn precedence_prefers_fresh_hook_and_falls_back_when_stale() {
        let policy = StatusPrecedencePolicy::default();
        let now = Instant::now();
        let fresh = now - Duration::from_secs(10);
        let stale = now - Duration::from_secs(180);

        let res = StatusPrecedenceResolver::resolve(
            policy,
            Some((Osc7501Status::WaitingApproval, fresh)),
            Some(Osc7501Status::Working),
            now,
        );
        assert_eq!(
            res,
            Some((
                Osc7501Status::WaitingApproval,
                ResolvedAgentStatusSource::ManagedHook
            ))
        );

        let fallback = StatusPrecedenceResolver::resolve(
            policy,
            Some((Osc7501Status::WaitingApproval, stale)),
            Some(Osc7501Status::Working),
            now,
        );
        assert_eq!(
            fallback,
            Some((Osc7501Status::Working, ResolvedAgentStatusSource::Osc7501))
        );
    }
}

