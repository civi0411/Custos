//! Host OS Caffeinate Execution Adapter (SADE R8 §6.3)
//!
//! Encapsulates OS process management for preventing system sleep,
//! eliminating naked global static mutable tuples and direct Command spawns in the API layer.

use std::sync::Mutex;

/// Manages system sleep inhibition via the host `caffeinate` utility.
pub static CAFFEINATE_MANAGER: CaffeinateManager = CaffeinateManager::new();

#[derive(Default)]
pub struct CaffeinateManager {
    state: Mutex<(String, Option<std::process::Child>)>,
}

impl CaffeinateManager {
    pub const fn new() -> Self {
        Self {
            state: Mutex::new((String::new(), None)),
        }
    }

    /// Returns current mode ("off", "on", "always") and whether the child process is active.
    pub fn status(&self) -> (String, bool) {
        let mut lock = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let (mode, child) = &mut *lock;
        if mode.is_empty() {
            *mode = "off".to_string();
        }
        let is_active = child
            .as_mut()
            .map(|c| c.try_wait().ok().flatten().is_none())
            .unwrap_or(false);
        (mode.clone(), is_active)
    }

    /// Sets the caffeinate mode ("on", "always", or "off"), spawning or killing the child process as necessary.
    pub fn set_mode(&self, mode: &str) -> (String, bool) {
        let mut lock = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let (current_mode, child) = &mut *lock;
        let normalized = mode.trim().to_ascii_lowercase();
        *current_mode = normalized.clone();

        let should_run = matches!(normalized.as_str(), "always" | "on" | "agent");
        if should_run {
            let alive = child
                .as_mut()
                .map(|c| c.try_wait().ok().flatten().is_none())
                .unwrap_or(false);
            if !alive {
                *child = spawn_awake_assertion();
            }
        } else if let Some(mut c) = child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }

        let is_active = child
            .as_mut()
            .map(|c| c.try_wait().ok().flatten().is_none())
            .unwrap_or(false);
        (current_mode.clone(), is_active)
    }
}

fn spawn_awake_assertion() -> Option<std::process::Child> {
    #[cfg(target_os = "macos")]
    {
        let pid = std::process::id().to_string();
        return std::process::Command::new("caffeinate")
            .args(["-i", "-s", "-w", &pid])
            .spawn()
            .ok();
    }
    #[cfg(target_os = "linux")]
    {
        return std::process::Command::new("systemd-inhibit")
            .args([
                "--what=sleep:handle-lid-switch",
                "--mode=block",
                "--why=Custos active agent run",
                "cat",
            ])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .ok();
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caffeinate_recognizes_agent_mode_and_off() {
        let mgr = CaffeinateManager::new();
        let (mode, _) = mgr.set_mode("agent");
        assert_eq!(mode, "agent");
        let (off_mode, active) = mgr.set_mode("off");
        assert_eq!(off_mode, "off");
        assert!(!active);
    }
}
