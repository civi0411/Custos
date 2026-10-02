//! Sandbox Path Containment Guard
//!
//! Enforces workspace containment and prevents directory traversal attacks
//! (e.g. `../` escapes, symlink traversal, or arbitrary absolute paths).

use custos_domain::DomainError;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PathSandbox {
    workspace_root: PathBuf,
}

impl PathSandbox {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Result<Self, DomainError> {
        let root = workspace_root.into();
        let canonical_root = std::fs::canonicalize(&root).unwrap_or(root);
        Ok(Self {
            workspace_root: canonical_root,
        })
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    /// Resolves and validates that the requested path strictly resides within the sandbox workspace.
    /// Traversal attempts (such as `../` escaping workspace or absolute paths outside workspace)
    /// return `DomainError::Unauthorized`.
    pub fn resolve_and_contain(&self, path: impl AsRef<Path>) -> Result<PathBuf, DomainError> {
        let path = path.as_ref();

        // Disallow null bytes
        if path.to_string_lossy().contains('\0') {
            return Err(DomainError::Unauthorized(
                "Sandbox containment violation: Path contains null byte".into(),
            ));
        }

        let full_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace_root.join(path)
        };

        // Normalize path components to check traversal before canonicalization
        let mut normalized = PathBuf::new();
        for component in full_path.components() {
            match component {
                Component::Prefix(p) => normalized.push(p.as_os_str()),
                Component::RootDir => normalized.push(Component::RootDir.as_os_str()),
                Component::CurDir => {}
                Component::ParentDir => {
                    if !normalized.pop() {
                        return Err(DomainError::Unauthorized(format!(
                            "Sandbox containment violation: Path '{}' attempts parent traversal above filesystem root",
                            path.display()
                        )));
                    }
                }
                Component::Normal(c) => normalized.push(c),
            }
        }

        // Canonicalize if target exists
        let target_path = if let Ok(canonical) = std::fs::canonicalize(&normalized) {
            canonical
        } else {
            normalized
        };

        if !target_path.starts_with(&self.workspace_root) {
            return Err(DomainError::Unauthorized(format!(
                "Sandbox containment violation: Path '{}' resolves to '{}' which is outside workspace root '{}'",
                path.display(),
                target_path.display(),
                self.workspace_root.display()
            )));
        }

        Ok(target_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_path_within_workspace() {
        let temp_dir = std::env::temp_dir();
        let sandbox = PathSandbox::new(&temp_dir).unwrap();

        let res = sandbox.resolve_and_contain("subfolder/file.txt");
        assert!(res.is_ok());
        let resolved = res.unwrap();
        assert!(resolved.starts_with(sandbox.workspace_root()));
    }

    #[test]
    fn test_parent_traversal_denied() {
        let temp_dir = std::env::temp_dir().join("custos_test_sandbox_sub");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let sandbox = PathSandbox::new(&temp_dir).unwrap();

        let res = sandbox.resolve_and_contain("../../etc/passwd");
        assert!(res.is_err());
        match res.unwrap_err() {
            DomainError::Unauthorized(msg) => {
                assert!(msg.contains("Sandbox containment violation"));
            }
            other => panic!("Expected Unauthorized, got: {other:?}"),
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_absolute_path_outside_denied() {
        let temp_dir = std::env::temp_dir().join("custos_test_sandbox_sub2");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let sandbox = PathSandbox::new(&temp_dir).unwrap();

        let res = sandbox.resolve_and_contain("/etc/passwd");
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), DomainError::Unauthorized(_)));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
