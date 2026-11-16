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
        *current_mode = mode.to_string();

        let should_run = mode == "always" || mode == "on";
        if should_run {
            if child.is_none() {
                *child = std::process::Command::new("caffeinate")
                    .args(["-i", "-s"])
                    .spawn()
                    .ok();
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
