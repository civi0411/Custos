//! 3-tier Caller Scope Policy & Public Source Refusal for remote RPC surfaces.

use std::net::{IpAddr, Ipv4Addr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallerScope {
    Owner,
    RuntimePaired { allow_desktop_control: bool },
    Mobile,
}

impl CallerScope {
    pub fn from_header(raw: Option<&str>) -> Self {
        match raw.map(str::trim).map(str::to_ascii_lowercase).as_deref() {
            Some("mobile") => Self::Mobile,
            Some("runtime-paired") | Some("runtime_paired") => Self::RuntimePaired {
                allow_desktop_control: false,
            },
            _ => Self::Owner,
        }
    }
}

pub struct CallerScopePolicy;

impl CallerScopePolicy {
    pub fn allows(scope: CallerScope, method: &str) -> bool {
        match scope {
            CallerScope::Owner => true,
            CallerScope::RuntimePaired {
                allow_desktop_control,
            } => {
                if method.starts_with("v1.roaming.trust.") || method.starts_with("v1.pairing.") {
                    return false;
                }
                if !allow_desktop_control && method.starts_with("v1.desktop_control.") {
                    return false;
                }
                true
            }
            CallerScope::Mobile => {
                const ALLOWED_PREFIXES: &[&str] = &[
                    "v1.ping",
                    "v1.capabilities.",
                    "v1.tasks.",
                    "v1.workflow.",
                    "v1.assistant.outbox.",
                    "v1.quant.kill_switch",
                    "v1.experiment.observe",
                    "v1.memory.proposals.",
                    "v1.hooks.permission.",
                    "v1.terminal.",
                ];
                ALLOWED_PREFIXES
                    .iter()
                    .any(|prefix| method == *prefix || method.starts_with(prefix))
            }
        }
    }
}

/// Admits loopback, RFC1918 private LAN, and Tailscale CGNAT (`100.64.0.0/10`) by default.
pub fn is_allowed_remote_ip(ip: IpAddr, allow_public: bool) -> bool {
    if allow_public || ip.is_loopback() {
        return true;
    }
    match ip {
        IpAddr::V4(v4) => is_private_or_tailscale_v4(v4),
        IpAddr::V6(v6) => (v6.segments()[0] & 0xfe00) == 0xfc00, // ULA fc00::/7
    }
}

fn is_private_or_tailscale_v4(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    ip.is_private() || (o[0] == 100 && (o[1] & 0b1100_0000) == 64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caller_scope_enforces_mobile_allowlist_and_paired_admin_blocks() {
        assert!(CallerScopePolicy::allows(CallerScope::Owner, "v1.roaming.trust.revoke"));
        assert!(!CallerScopePolicy::allows(
            CallerScope::RuntimePaired {
                allow_desktop_control: false
            },
            "v1.roaming.trust.revoke"
        ));
        assert!(CallerScopePolicy::allows(
            CallerScope::Mobile,
            "v1.quant.kill_switch"
        ));
        assert!(!CallerScopePolicy::allows(
            CallerScope::Mobile,
            "v1.providers.save"
        ));

        assert!(is_allowed_remote_ip("100.80.1.2".parse().unwrap(), false));
        assert!(!is_allowed_remote_ip("8.8.8.8".parse().unwrap(), false));
        assert!(is_allowed_remote_ip("8.8.8.8".parse().unwrap(), true));
    }
}

