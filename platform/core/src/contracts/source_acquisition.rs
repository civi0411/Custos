//! Source Acquisition Port Contract
//!
//! Abstracts physical retrieval of source content (local files, URLs, etc.)
//! for citation verification, claim lineage, and fact checking.

use async_trait::async_trait;
use custos_domain::DomainError;

#[async_trait]
pub trait SourceAcquisitionPort: Send + Sync {
    /// Reads or fetches the text content of a source given a relative path or URI and optional base path.
    async fn fetch_source_text(
        &self,
        path_or_uri: &str,
        base_path: Option<&str>,
    ) -> Result<String, DomainError>;
}
