//! Host OS Binary Resolution Utilities
//!
//! Resolves paths to external CLI binaries on the host machine (`agent-browser`,
//! `ssh`, `gh`, `git`) using environment overrides, standard installation paths,
//! and PATH lookups.

use std::path::PathBuf;

pub fn resolve_agent_browser_binary() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("AGENT_BROWSER_BINARY") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }
    if let Some(home) = dirs::home_dir() {
        let local_bin = home.join(".local/bin/agent-browser");
        if local_bin.exists() {
            return Some(local_bin);
        }
    }
    if let Ok(output) = std::process::Command::new("which")
        .arg("agent-browser")
        .output()
    {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let p = PathBuf::from(path_str);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

pub fn resolve_ssh_binary() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("SSH_BINARY") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }
    for candidate in [
        "/usr/bin/ssh",
        "/usr/local/bin/ssh",
        "/opt/homebrew/bin/ssh",
    ] {
        let p = PathBuf::from(candidate);
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(output) = std::process::Command::new("which").arg("ssh").output() {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let p = PathBuf::from(path_str);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

pub fn resolve_gh_binary() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("GH_BINARY") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }
    for candidate in ["/opt/homebrew/bin/gh", "/usr/local/bin/gh", "/usr/bin/gh"] {
        let p = PathBuf::from(candidate);
        if p.exists() {
            return Some(p);
        }
    }
    if let Some(home) = dirs::home_dir() {
        let local_bin = home.join(".local/bin/gh");
        if local_bin.exists() {
            return Some(local_bin);
        }
    }
    if let Ok(output) = std::process::Command::new("which").arg("gh").output() {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let p = PathBuf::from(path_str);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

pub fn resolve_git_binary() -> PathBuf {
    if let Ok(path) = std::env::var("GIT_BINARY") {
        let p = PathBuf::from(path);
        if p.exists() {
            return p;
        }
    }
    for candidate in [
        "/opt/homebrew/bin/git",
        "/usr/local/bin/git",
        "/usr/bin/git",
    ] {
        let p = PathBuf::from(candidate);
        if p.exists() {
            return p;
        }
    }
    PathBuf::from("git")
}

