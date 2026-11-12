//! Client-side daemon connection helpers and re-exported `ProfileResolver` from `custos-protocol`.

use crate::local_api::LocalApiClient;
pub use custos_protocol::profile::{DaemonLifecycleState, ProfileResolver};
use std::sync::Arc;

pub fn spawn_daemon_process() {
    let daemon_bin =
        std::env::var("CUSTOS_DAEMON_BIN").unwrap_or_else(|_| "custos-daemon".to_string());
    let mut cmd = std::process::Command::new(&daemon_bin);

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            let candidate = parent.join("custos-daemon");
            let candidate_exe = parent.join("custos-daemon.exe");
            if candidate_exe.exists() {
                cmd = std::process::Command::new(candidate_exe);
            } else if candidate.exists() {
                cmd = std::process::Command::new(candidate);
            }
        }
    }

    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::inherit());

    match cmd.spawn() {
        Ok(_) => eprintln!("[Client] Spawned background custos-daemon process"),
        Err(e) => eprintln!("[Client] Notice: unable to spawn daemon binary directly: {e}"),
    }
}

pub async fn ensure_daemon_client_async(
    profile: &ProfileResolver,
) -> Result<Arc<LocalApiClient>, String> {
    if let Some(port) = profile.read_port() {
        let client = LocalApiClient::from_port(port);
        if client.ping().await.is_ok() {
            return Ok(Arc::new(client));
        }
    }

    spawn_daemon_process();

    let start = tokio::time::Instant::now();
    while start.elapsed() < std::time::Duration::from_secs(30) {
        if let Some(port) = profile.read_port() {
            let client = LocalApiClient::from_port(port);
            if client.ping().await.is_ok() {
                return Ok(Arc::new(client));
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    Err(format!(
        "Failed to connect to custos-daemon for profile '{}' (port {:?})",
        profile.profile_id(),
        profile.read_port()
    ))
}

pub fn ensure_daemon_client(profile: &ProfileResolver) -> Result<Arc<LocalApiClient>, String> {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        tokio::task::block_in_place(|| handle.block_on(ensure_daemon_client_async(profile)))
    } else {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        rt.block_on(ensure_daemon_client_async(profile))
    }
}
