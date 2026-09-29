use crate::compiler::{SourceDocument, TokenAwareContextCompiler};
use crate::traits::ContextBuilder;
use custos_core_domain::{ContextItem, ContextPack, DomainError, new_id};
use custos_repo_intelligence::scanner::WorkspaceScanner;
use sha2::{Digest, Sha256};
use std::path::Path;

pub struct EngineeringRecipe {
    pub workspace_root: std::path::PathBuf,
    pub max_tokens: usize,
}

impl EngineeringRecipe {
    pub fn new(workspace_root: impl AsRef<Path>, max_tokens: usize) -> Self {
        Self {
            workspace_root: workspace_root.as_ref().to_path_buf(),
            max_tokens,
        }
    }

    pub async fn compile_pack(&self, query: &str) -> Result<ContextPack, DomainError> {
        let scanner = WorkspaceScanner::new();
        let inventory = scanner.scan_inventory(&self.workspace_root)?;

        let mut compiler = TokenAwareContextCompiler::new();
        for file_entry in inventory {
            // Only add smaller text files to avoid memory bloat during demo
            if file_entry.size_bytes < 500_000 {
                if let Ok(content) = std::fs::read_to_string(&file_entry.absolute_path) {
                    compiler.add_document(SourceDocument::new(file_entry.relative_path, content));
                }
            }
        }

        let slice = compiler.compile(query, self.max_tokens).await?;

        // Add provenance hash to each item
        let items: Vec<ContextItem> = slice
            .items
            .into_iter()
            .map(|mut item| {
                let mut hasher = Sha256::new();
                hasher.update(item.content.as_bytes());
                let result = hasher.finalize();
                item.provenance_hash = Some(format!("{:x}", result));
                item
            })
            .collect();

        Ok(ContextPack {
            id: new_id("pack"),
            items,
            total_tokens: slice.total_tokens,
        })
    }
}
