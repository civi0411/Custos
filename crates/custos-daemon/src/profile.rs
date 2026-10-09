//! Profile Resolution and Singleton Daemon Process Locking
//!
//! Enforces SADE-OrCa single backend owner constraints (Wave H0, H1):
//! 1. Canonical profile directory tree: `~/.custos/profiles/<profile_id>/`
//! 2. Exclusive filesystem lock (`daemon.lock`) preventing concurrent daemon processes
//!    from corrupting SQLite WAL databases.

use fs2::FileExt;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DaemonLockError {
    #[error("Daemon is already running (pid: {pid:?}) on lock file: {path}")]
    AlreadyRunning {
        pid: Option<u32>,
        path: PathBuf,
    },
    #[error("I/O error handling daemon lock at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Resolves standard directory and file paths for a given Custos profile.
#[derive(Debug, Clone)]
pub struct ProfileResolver {
    profile_id: String,
    profile_dir: PathBuf,
}

impl ProfileResolver {
    /// Resolve profile configuration based on environment variables or defaults.
    ///
    /// Resolution order:
    /// - Profile dir: `CUSTOS_PROFILE_DIR` -> `~/.custos/profiles/<profile_id>`
    /// - Profile id: `CUSTOS_PROFILE` -> `"default"`
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

    /// Returns path to the SQLite database file.
    ///
    /// Respects `CUSTOS_DATABASE` or `CUSTOS_DB_PATH` environment overrides if set.
    pub fn database_path(&self) -> PathBuf {
        if let Ok(path) = std::env::var("CUSTOS_DATABASE") {
            return PathBuf::from(path);
        }
        if let Ok(path) = std::env::var("CUSTOS_DB_PATH") {
            return PathBuf::from(path);
        }
        self.profile_dir.join("custos.db")
    }

    /// Path to the exclusive daemon lock file.
    pub fn lock_path(&self) -> PathBuf {
        self.profile_dir.join("daemon.lock")
    }

    /// Path to the local API TCP/IPC port discovery file.
    pub fn port_path(&self) -> PathBuf {
        self.profile_dir.join("daemon.port")
    }

    /// Path to the local API HTTP port discovery file.
    pub fn http_port_path(&self) -> PathBuf {
        self.profile_dir.join("daemon.http_port")
    }

    /// Read the port recorded by a running daemon if available.
    pub fn read_port(&self) -> Option<u16> {
        let path = self.port_path();
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(path).ok()?;
        content.trim().parse::<u16>().ok()
    }

    /// Read the HTTP port recorded by a running daemon if available.
    pub fn read_http_port(&self) -> Option<u16> {
        let path = self.http_port_path();
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(path).ok()?;
        content.trim().parse::<u16>().ok()
    }

    /// Record the active port number for client discovery.
    pub fn write_port(&self, port: u16) -> std::io::Result<()> {
        let path = self.port_path();
        std::fs::write(path, port.to_string())
    }

    /// Record the active HTTP port number for client discovery.
    pub fn write_http_port(&self, port: u16) -> std::io::Result<()> {
        let path = self.http_port_path();
        std::fs::write(path, port.to_string())
    }

    /// Remove the port files on daemon shutdown.
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

/// Ensure a daemon client connection is ready for the given profile.
///
/// If daemon is already running, attaches to its local TCP port.
/// If not running, attempts to spawn `custos-daemon` and waits for it to become ready.
pub fn ensure_daemon_client(
    profile: &ProfileResolver,
) -> Result<std::sync::Arc<crate::local_api::LocalApiClient>, String> {
    use crate::local_api::LocalApiClient;

    // 1. If port file exists, try to connect and ping
    if let Ok(client) = LocalApiClient::from_profile(profile) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        if rt.block_on(client.ping()).is_ok() {
            return Ok(std::sync::Arc::new(client));
        }
    }

    // 2. If daemon not reachable, attempt to spawn daemon binary
    let daemon_bin = std::env::var("CUSTOS_DAEMON_BIN").unwrap_or_else(|_| "custos-daemon".to_string());
    let mut cmd = std::process::Command::new(&daemon_bin);

    // Look for daemon next to current executable or in target/debug
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

    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::inherit());

    match cmd.spawn() {
        Ok(_) => eprintln!("[Client] Spawned background custos-daemon process"),
        Err(e) => eprintln!("[Client] Notice: unable to spawn daemon binary directly: {e}"),
    }

    // 3. Poll for port file and successful ping for up to 3 seconds
    let start = std::time::Instant::now();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;

    while start.elapsed() < std::time::Duration::from_secs(3) {
        if let Ok(client) = LocalApiClient::from_profile(profile) {
            if rt.block_on(client.ping()).is_ok() {
                return Ok(std::sync::Arc::new(client));
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    Err(format!(
        "Failed to connect to custos-daemon for profile '{}' (port {:?})",
        profile.profile_id(),
        profile.read_port()
    ))
}


/// Exclusive process-level lock held by the running daemon.
///
/// Uses OS kernel advisory locking (`flock`/`LockFileEx` via `fs2`).
/// Automatically released by the kernel if the process terminates or crashes.
pub struct DaemonLock {
    _file: File,
    lock_path: PathBuf,
}

impl DaemonLock {
    /// Acquire exclusive lock on the given file path.
    ///
    /// Returns `Err(DaemonLockError::AlreadyRunning)` if another process holds the lock.
    pub fn acquire(lock_path: impl AsRef<Path>) -> Result<Self, DaemonLockError> {
        let lock_path = lock_path.as_ref().to_path_buf();

        if let Some(parent) = lock_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|source| DaemonLockError::Io {
                path: lock_path.clone(),
                source,
            })?;

        // Attempt non-blocking exclusive lock
        match file.try_lock_exclusive() {
            Ok(()) => {
                // Lock acquired. Write current PID into file for diagnostic inspection.
                let _ = file.set_len(0);
                let pid = std::process::id();
                let _ = file.write_all(format!("{pid}\n").as_bytes());
                let _ = file.flush();

                Ok(Self {
                    _file: file,
                    lock_path,
                })
            }
            Err(e) => {
                // Another process holds the lock or an I/O error occurred
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.raw_os_error() == Some(35) // EWOULDBLOCK on unix
                    || e.raw_os_error() == Some(11) // EAGAIN on linux
                    || e.raw_os_error() == Some(33) // ERROR_LOCK_VIOLATION on Windows
                {
                    // Read existing PID if present
                    let mut content = String::new();
                    let pid = file
                        .read_to_string(&mut content)
                        .ok()
                        .and_then(|_| content.trim().parse::<u32>().ok());

                    Err(DaemonLockError::AlreadyRunning {
                        pid,
                        path: lock_path,
                    })
                } else {
                    Err(DaemonLockError::Io {
                        path: lock_path,
                        source: e,
                    })
                }
            }
        }
    }

    /// Check if a lock file is currently locked without retaining the lock.
    pub fn is_locked(lock_path: impl AsRef<Path>) -> bool {
        let lock_path = lock_path.as_ref();
        if !lock_path.exists() {
            return false;
        }
        match OpenOptions::new().read(true).write(true).open(lock_path) {
            Ok(file) => match file.try_lock_exclusive() {
                Ok(()) => {
                    let _ = file.unlock();
                    false
                }
                Err(_) => true,
            },
            Err(_) => false,
        }
    }

    /// Explicitly release the lock and clean up the lock file.
    pub fn release(self) {
        let _ = std::fs::remove_file(&self.lock_path);
    }
}

impl Drop for DaemonLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.lock_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_lock_acquisition_and_exclusivity() {
        let temp_dir = tempfile::tempdir().unwrap();
        let lock_file = temp_dir.path().join("daemon.lock");

        // First acquisition must succeed
        let lock1 = DaemonLock::acquire(&lock_file).expect("First lock should succeed");
        assert!(DaemonLock::is_locked(&lock_file));

        // Second acquisition must fail with AlreadyRunning
        let lock2_res = DaemonLock::acquire(&lock_file);
        assert!(matches!(
            lock2_res,
            Err(DaemonLockError::AlreadyRunning { .. })
        ));

        // Release first lock
        drop(lock1);

        // Third acquisition must now succeed
        let lock3 = DaemonLock::acquire(&lock_file).expect("Lock should succeed after drop");
        drop(lock3);
    }

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

        // Test port write and read
        resolver.write_port(4242).unwrap();
        assert_eq!(resolver.read_port(), Some(4242));
        resolver.remove_port().unwrap();
        assert_eq!(resolver.read_port(), None);
    }
}
