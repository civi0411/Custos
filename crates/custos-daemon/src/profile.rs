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
    AlreadyRunning { pid: Option<u32>, path: PathBuf },
    #[error("I/O error handling daemon lock at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

pub use custos_sdk::profile::{
    ensure_daemon_client, ensure_daemon_client_async, spawn_daemon_process, DaemonLifecycleState,
    ProfileResolver,
};


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
                    || e.raw_os_error() == Some(33)
                // ERROR_LOCK_VIOLATION on Windows
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
