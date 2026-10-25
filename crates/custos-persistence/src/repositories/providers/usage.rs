use super::ProviderRepository;
use custos_domain::DomainError;

impl ProviderRepository {
    /// Queries the total recorded events for a given provider without exposing credentials or tokens.
    pub fn provider_event_counts(&self, provider_id: &str) -> Result<i64, DomainError> {
        let conn = self.db.lock()?;
        conn.query_row(
            "SELECT COUNT(*) FROM outbox_messages WHERE topic LIKE 'provider.%' AND payload LIKE ?1",
            rusqlite::params![format!("%\"provider_id\":\"{}\"%", provider_id)],
            |row| row.get(0),
        )
        .map_err(|e| DomainError::Validation(e.to_string()))
    }
}
