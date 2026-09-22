//! Workspace Scanner & Inventory Engine
//!
//! Recursively scans repositories, builds file inventories, and extracts basic symbols.

use custos_core_domain::DomainError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub relative_path: String,
    pub absolute_path: PathBuf,
    pub extension: Option<String>,
    pub size_bytes: u64,
    pub line_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolRef {
    pub name: String,
    pub kind: String, // "struct", "enum", "trait", "fn", "mod"
    pub file_path: String,
    pub line_number: usize,
}

#[derive(Debug, Clone, Default)]
pub struct WorkspaceScanner {
    ignored_patterns: Vec<String>,
}

impl WorkspaceScanner {
    pub fn new() -> Self {
        Self {
            ignored_patterns: vec![
                ".git".into(),
                "target".into(),
                "node_modules".into(),
                ".custos".into(),
                ".idea".into(),
                ".vscode".into(),
            ],
        }
    }

    pub fn with_ignored_patterns(patterns: Vec<String>) -> Self {
        Self {
            ignored_patterns: patterns,
        }
    }

    /// Recursively scans root directory and returns list of source files.
    pub fn scan_inventory(&self, root: &Path) -> Result<Vec<FileEntry>, DomainError> {
        if !root.exists() {
            return Err(DomainError::Validation(format!(
                "Path does not exist: {}",
                root.display()
            )));
        }

        let mut entries = Vec::new();
        self.walk_dir(root, root, &mut entries)?;
        entries.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        Ok(entries)
    }

    fn walk_dir(
        &self,
        base_dir: &Path,
        current_dir: &Path,
        results: &mut Vec<FileEntry>,
    ) -> Result<(), DomainError> {
        let read_dir = std::fs::read_dir(current_dir)
            .map_err(|e| DomainError::Validation(format!("Cannot read dir: {e}")))?;

        for entry_res in read_dir {
            let entry = entry_res
                .map_err(|e| DomainError::Validation(format!("Error accessing entry: {e}")))?;
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            if self.ignored_patterns.iter().any(|ig| ig == &file_name) {
                continue;
            }

            if path.is_dir() {
                self.walk_dir(base_dir, &path, results)?;
            } else if path.is_file() {
                let metadata = entry
                    .metadata()
                    .map_err(|e| DomainError::Validation(format!("Cannot read metadata: {e}")))?;
                let relative_path = path
                    .strip_prefix(base_dir)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();

                let extension = path
                    .extension()
                    .map(|ext| ext.to_string_lossy().to_string());

                // Count lines if utf-8 text
                let line_count = if let Ok(content) = std::fs::read_to_string(&path) {
                    content.lines().count()
                } else {
                    0
                };

                results.push(FileEntry {
                    relative_path,
                    absolute_path: path,
                    extension,
                    size_bytes: metadata.len(),
                    line_count,
                });
            }
        }

        Ok(())
    }

    /// Simple heuristic symbol extraction for Rust/TypeScript/Python
    pub fn extract_symbols(&self, file: &FileEntry) -> Result<Vec<SymbolRef>, DomainError> {
        let content = std::fs::read_to_string(&file.absolute_path)
            .map_err(|e| DomainError::Validation(format!("Cannot read file: {e}")))?;

        let mut symbols = Vec::new();

        for (line_idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            let line_num = line_idx + 1;

            // Rust symbols
            if trimmed.starts_with("pub struct ") || trimmed.starts_with("struct ") {
                if let Some(name) = extract_name(trimmed, "struct") {
                    symbols.push(SymbolRef {
                        name,
                        kind: "struct".into(),
                        file_path: file.relative_path.clone(),
                        line_number: line_num,
                    });
                }
            } else if trimmed.starts_with("pub trait ") || trimmed.starts_with("trait ") {
                if let Some(name) = extract_name(trimmed, "trait") {
                    symbols.push(SymbolRef {
                        name,
                        kind: "trait".into(),
                        file_path: file.relative_path.clone(),
                        line_number: line_num,
                    });
                }
            } else if trimmed.starts_with("pub enum ") || trimmed.starts_with("enum ") {
                if let Some(name) = extract_name(trimmed, "enum") {
                    symbols.push(SymbolRef {
                        name,
                        kind: "enum".into(),
                        file_path: file.relative_path.clone(),
                        line_number: line_num,
                    });
                }
            } else if trimmed.starts_with("pub fn ") || trimmed.starts_with("fn ") {
                if let Some(name) = extract_name(trimmed, "fn") {
                    symbols.push(SymbolRef {
                        name,
                        kind: "fn".into(),
                        file_path: file.relative_path.clone(),
                        line_number: line_num,
                    });
                }
            }
        }

        Ok(symbols)
    }
}

fn extract_name(line: &str, keyword: &str) -> Option<String> {
    let after_kw = line.split(keyword).nth(1)?;
    let clean = after_kw.trim();
    let name_part = clean
        .split(|c: char| c == '<' || c == '(' || c == '{' || c == ':' || c.is_whitespace())
        .next()?;
    if name_part.is_empty() {
        None
    } else {
        Some(name_part.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_and_symbol_extraction() {
        let temp_dir = std::env::temp_dir().join(format!("custos_repo_int_{}", custos_core_domain::new_id("test")));
        std::fs::create_dir_all(temp_dir.join("src")).unwrap();
        std::fs::create_dir_all(temp_dir.join(".git")).unwrap();

        // Ignored file in .git
        std::fs::write(temp_dir.join(".git").join("config"), "git config").unwrap();

        // Real code file
        let code = r#"
pub struct TaskManager {
    pub id: String,
}

pub trait Worker {
    fn execute(&self);
}

pub fn run_worker() {
}
"#;
        std::fs::write(temp_dir.join("src").join("lib.rs"), code).unwrap();

        let scanner = WorkspaceScanner::new();
        let inventory = scanner.scan_inventory(&temp_dir).unwrap();

        // .git is ignored, only src/lib.rs is found
        assert_eq!(inventory.len(), 1);
        assert_eq!(inventory[0].relative_path, "src/lib.rs");
        assert_eq!(inventory[0].extension.as_deref(), Some("rs"));

        let symbols = scanner.extract_symbols(&inventory[0]).unwrap();
        assert_eq!(symbols.len(), 4);
        assert_eq!(symbols[0].name, "TaskManager");
        assert_eq!(symbols[0].kind, "struct");
        assert_eq!(symbols[1].name, "Worker");
        assert_eq!(symbols[1].kind, "trait");
        assert_eq!(symbols[2].name, "execute");
        assert_eq!(symbols[2].kind, "fn");
        assert_eq!(symbols[3].name, "run_worker");
        assert_eq!(symbols[3].kind, "fn");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
