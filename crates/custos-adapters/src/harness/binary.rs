//! Binary resolution and availability probing across platform PATH and standard locations.

use std::path::{Path, PathBuf};

pub fn resolve_binary_path(binary_name: &str) -> Option<PathBuf> {
    if binary_name == "internal" || binary_name.is_empty() {
        return None;
    }
    for sub in binary_name.split([',', '|']).map(str::trim) {
        let bin_name = sub.split_whitespace().next().unwrap_or(sub);
        let bin = Path::new(bin_name);
        if bin.exists() {
            return Some(bin.to_path_buf());
        }
        if let Ok(path) = which::which(bin) {
            return Some(path);
        }
        let candidates = [
            "/opt/homebrew/bin",
            "/usr/local/bin",
            "/usr/bin",
            "/bin",
        ];
        for dir in candidates {
            let candidate = Path::new(dir).join(bin);
            if candidate.exists() {
                return Some(candidate);
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            let candidate1 = Path::new(&home).join(".cargo/bin").join(bin);
            if candidate1.exists() {
                return Some(candidate1);
            }
            let candidate2 = Path::new(&home).join(".local/bin").join(bin);
            if candidate2.exists() {
                return Some(candidate2);
            }
        }
        #[cfg(windows)]
        {
            if let Ok(userprofile) = std::env::var("USERPROFILE") {
                let cargo_bin = Path::new(&userprofile).join(".cargo/bin");
                for candidate in [
                    cargo_bin.join(bin),
                    cargo_bin.join(format!("{bin_name}.exe")),
                    cargo_bin.join(format!("{bin_name}.cmd")),
                ] {
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
            }
            if let Ok(appdata) = std::env::var("APPDATA") {
                let npm_bin = Path::new(&appdata).join("npm");
                for candidate in [
                    npm_bin.join(bin),
                    npm_bin.join(format!("{bin_name}.cmd")),
                    npm_bin.join(format!("{bin_name}.exe")),
                ] {
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
            }
            if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
                let pnpm_bin = Path::new(&localappdata).join("pnpm");
                for candidate in [
                    pnpm_bin.join(bin),
                    pnpm_bin.join(format!("{bin_name}.cmd")),
                    pnpm_bin.join(format!("{bin_name}.exe")),
                ] {
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
            }
        }
    }
    None
}

pub fn probe_binary_available(binary_name: &str) -> bool {
    binary_name == "internal" || binary_name.is_empty() || resolve_binary_path(binary_name).is_some()
}
