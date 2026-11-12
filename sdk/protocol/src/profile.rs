//! Profile path resolution and daemon lifecycle state shared by daemon and clients.

use std::path::{Path, PathBuf};

/// Client-visible daemon connectivity and attach lifecycle state (Phase D3 / F07).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DaemonLifecycleState {
    Connecting,
    Ready,
    Incompatible,
    Degraded,
    Recovering,
}

/// Resolves standard directory and file paths for a given Custos profile.
#[derive(Debug, Clone)]
pub struct ProfileResolver {
    profile_id: String,
    profile_dir: PathBuf,
}

impl ProfileResolver {
    /// Resolve profile configuration based on environment variables or defaults.
    pub fn from_env() -> Self {
        let profile_id = std::env::var("CUSTOS_PROFILE").unwrap_or_else(|_| "default".to_string());

        let profile_dir = if let Ok(custom_dir) = std::env::var("CUSTOS_PROFILE_DIR") {
            PathBuf::from(custom_dir)
        } else {
            let root = if let Ok(custom_home) = std::env::var("CUSTOS_HOME") {
                PathBuf::from(custom_home).join("profiles")
            } else if let Some(home) = dirs::home_dir() {
                home.join(".custos").join("profiles")
            } else {
                PathBuf::from(".").join(".custos").join("profiles")
            };
            root.join(&profile_id)
        };

        Self::with_dir(profile_id, profile_dir)
    }

    /// Resolve paths for a specific profile ID using standard home root.
    pub fn for_profile(profile_id: impl Into<String>) -> Self {
        let profile_id = profile_id.into();
        let root = if let Ok(custom_home) = std::env::var("CUSTOS_HOME") {
            PathBuf::from(custom_home).join("profiles")
        } else if let Some(home) = dirs::home_dir() {
            home.join(".custos").join("profiles")
        } else {
            PathBuf::from(".").join(".custos").join("profiles")
        };
        let profile_dir = root.join(&profile_id);
        Self::with_dir(profile_id, profile_dir)
    }

    /// Construct with an explicit directory.
    pub fn with_dir(profile_id: impl Into<String>, profile_dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&profile_dir);
        Self {
            profile_id: profile_id.into(),
            profile_dir,
        }
    }

    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }

    pub fn profile_dir(&self) -> &Path {
        &self.profile_dir
    }

    pub fn database_path(&self) -> PathBuf {
        if let Ok(path) = std::env::var("CUSTOS_DATABASE") {
            return PathBuf::from(path);
        }
        if let Ok(path) = std::env::var("CUSTOS_DB_PATH") {
            return PathBuf::from(path);
        }
        self.profile_dir.join("custos.db")
    }

    pub fn lock_path(&self) -> PathBuf {
        self.profile_dir.join("daemon.lock")
    }

    pub fn port_path(&self) -> PathBuf {
        self.profile_dir.join("daemon.port")
    }

    pub fn http_port_path(&self) -> PathBuf {
        self.profile_dir.join("daemon.http_port")
    }

    pub fn read_port(&self) -> Option<u16> {
        let path = self.port_path();
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(path).ok()?;
        content.trim().parse::<u16>().ok()
    }

    pub fn read_http_port(&self) -> Option<u16> {
        let path = self.http_port_path();
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(path).ok()?;
        content.trim().parse::<u16>().ok()
    }

    pub fn write_port(&self, port: u16) -> std::io::Result<()> {
        std::fs::write(self.port_path(), port.to_string())
    }

    pub fn write_http_port(&self, port: u16) -> std::io::Result<()> {
        std::fs::write(self.http_port_path(), port.to_string())
    }

    pub fn remove_port(&self) -> std::io::Result<()> {
        let path = self.port_path();
        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
        let http_path = self.http_port_path();
        if http_path.exists() {
            let _ = std::fs::remove_file(http_path);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_resolver_paths() {
        let temp_dir = tempfile::tempdir().unwrap();
        let profile_dir = temp_dir.path().join("test_profile");

        let resolver = ProfileResolver::with_dir("test", profile_dir.clone());
        assert_eq!(resolver.profile_id(), "test");
        assert_eq!(resolver.profile_dir(), profile_dir.as_path());
        assert_eq!(resolver.database_path(), profile_dir.join("custos.db"));
        assert_eq!(resolver.lock_path(), profile_dir.join("daemon.lock"));
        assert_eq!(resolver.port_path(), profile_dir.join("daemon.port"));

        resolver.write_port(4242).unwrap();
        assert_eq!(resolver.read_port(), Some(4242));
        resolver.remove_port().unwrap();
        assert_eq!(resolver.read_port(), None);
    }
}

