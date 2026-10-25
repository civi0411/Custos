//! Sandbox Path Containment Guard
//!
//! Enforces workspace containment and prevents directory traversal attacks
//! (e.g. `../` escapes, symlink traversal, or arbitrary absolute paths).
//! Pure policy validation; concrete OS stats/symlinks are injected via observations.

use custos_domain::DomainError;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PathSandbox {
    workspace_root: PathBuf,
}

impl PathSandbox {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Result<Self, DomainError> {
        let root = workspace_root.into();
        Ok(Self {
            workspace_root: root,
        })
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    /// Pure policy validator: checks containment against `workspace_root`.
    /// `observed_canonical` allows injecting the OS-observed canonicalized symlink resolution
    /// from an adapter without doing direct filesystem I/O in core.
    pub fn validate_with_observations(
        &self,
        path: impl AsRef<Path>,
        observed_canonical: Option<PathBuf>,
    ) -> Result<PathBuf, DomainError> {
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

        // Normalize path components to check traversal
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

        let target_path = observed_canonical.unwrap_or(normalized);

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

    /// Resolves and validates that the requested path strictly resides within the sandbox workspace.
    /// Traversal attempts (such as `../` escaping workspace or absolute paths outside workspace)
    /// return `DomainError::Unauthorized`.
    pub fn resolve_and_contain(&self, path: impl AsRef<Path>) -> Result<PathBuf, DomainError> {
        self.validate_with_observations(path, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_path_within_workspace() {
        let root = PathBuf::from("/mock/workspace");
        let sandbox = PathSandbox::new(&root).unwrap();

        let res = sandbox.resolve_and_contain("subfolder/file.txt");
        assert!(res.is_ok());
        let resolved = res.unwrap();
        assert!(resolved.starts_with(sandbox.workspace_root()));
    }

    #[test]
    fn test_parent_traversal_denied() {
        let root = PathBuf::from("/mock/workspace");
        let sandbox = PathSandbox::new(&root).unwrap();

        let res = sandbox.resolve_and_contain("../../etc/passwd");
        assert!(res.is_err());
        match res.unwrap_err() {
            DomainError::Unauthorized(msg) => {
                assert!(msg.contains("Sandbox containment violation"));
            }
            other => panic!("Expected Unauthorized, got: {other:?}"),
        }
    }

    #[test]
    fn test_absolute_path_outside_denied() {
        let root = PathBuf::from("/mock/workspace");
        let sandbox = PathSandbox::new(&root).unwrap();

        let res = sandbox.resolve_and_contain("/etc/passwd");
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), DomainError::Unauthorized(_)));
    }

    #[test]
    fn test_symlink_observation_rejection() {
        let root = PathBuf::from("/mock/workspace");
        let sandbox = PathSandbox::new(&root).unwrap();

        let observed_symlink = Some(PathBuf::from("/etc/target"));
        let res = sandbox.validate_with_observations("symlink_to_etc", observed_symlink);
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), DomainError::Unauthorized(_)));
    }
}
