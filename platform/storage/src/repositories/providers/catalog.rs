use super::ProviderRepository;
use custos_domain::{DomainError, ModelCatalogOption, ModelPricing};
use rusqlite::params;

impl ProviderRepository {
    pub fn save_catalog_models(
        &self,
        provider_id: &str,
        models: &[ModelCatalogOption],
    ) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let now = chrono::Utc::now().to_rfc3339();
        let payload = serde_json::to_string(
            &serde_json::json!({"provider_id": provider_id, "models": models}),
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        tx.execute(
            "INSERT INTO outbox_messages (id, topic, payload, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                custos_domain::new_id("catalog"),
                "provider.models.probed",
                payload,
                now
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        tx.execute(
            "DELETE FROM provider_model_catalog WHERE provider_id = ?1",
            params![provider_id],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        for m in models {
            let row_id = format!("{}:{}", provider_id, m.id);
            let ctx_int = m.context_window.map(|w| w as i64);
            let fast_mode_int = if m.supports_fast_mode { 1i64 } else { 0i64 };
            let pricing_json = m
                .pricing
                .as_ref()
                .and_then(|p| serde_json::to_string(p).ok());
            let fetched_at = chrono::Utc::now().timestamp_millis();
            let capabilities_json = serde_json::to_string(
                &serde_json::json!({"efforts":m.efforts,"is_default":m.is_default}),
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

            tx.execute(
                "INSERT INTO provider_model_catalog (id, provider_id, model_id, label, description, context_window,
                                                    default_effort, supports_fast_mode, pricing_json, fetched_at, capabilities_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(id) DO UPDATE SET
                    label = excluded.label,
                    description = excluded.description,
                    context_window = excluded.context_window,
                    default_effort = excluded.default_effort,
                    supports_fast_mode = excluded.supports_fast_mode,
                    pricing_json = excluded.pricing_json,
                    fetched_at = excluded.fetched_at,
                    capabilities_json = excluded.capabilities_json",
                params![
                    row_id,
                    provider_id,
                    m.id,
                    m.label,
                    m.description,
                    ctx_int,
                    m.default_effort,
                    fast_mode_int,
                    pricing_json,
                    fetched_at,
                    capabilities_json,
                ],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        }
        tx.commit()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub fn catalog_fetched_at(&self, provider_id: &str) -> Result<i64, DomainError> {
        let conn = self.db.lock()?;
        conn.query_row(
            "SELECT COALESCE(MAX(fetched_at), 0) FROM provider_model_catalog WHERE provider_id = ?1",
            params![provider_id],
            |row| row.get(0),
        )
        .map_err(|e| DomainError::Validation(e.to_string()))
    }

    pub fn list_catalog_models(
        &self,
        provider_id: Option<&str>,
    ) -> Result<Vec<ModelCatalogOption>, DomainError> {
        let conn = self.db.lock()?;
        let query = if provider_id.is_some() {
            "SELECT model_id, label, description, provider_id, context_window, default_effort,
                    supports_fast_mode, pricing_json, capabilities_json
             FROM provider_model_catalog WHERE provider_id = ?1 ORDER BY label ASC"
        } else {
            "SELECT model_id, label, description, provider_id, context_window, default_effort,
                    supports_fast_mode, pricing_json, capabilities_json
             FROM provider_model_catalog ORDER BY label ASC"
        };

        let mut stmt = conn
            .prepare(query)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let map_fn = |row: &rusqlite::Row| {
            let model_id: String = row.get(0)?;
            let label: String = row.get(1)?;
            let description: Option<String> = row.get(2)?;
            let p_type: String = row.get(3)?;
            let context_window: Option<i64> = row.get(4)?;
            let mut default_effort: Option<String> = row.get(5)?;
            let supports_fast_mode: i64 = row.get(6)?;
            let pricing_json: Option<String> = row.get(7)?;

            let pricing: Option<ModelPricing> = pricing_json
                .as_deref()
                .and_then(|j| serde_json::from_str(j).ok());

            let capabilities_json: String = row.get(8)?;
            let capabilities: serde_json::Value = serde_json::from_str(&capabilities_json)
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        8,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
            let efforts: Vec<String> = serde_json::from_value(
                capabilities
                    .get("efforts")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([])),
            )
            .map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    8,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            if default_effort
                .as_ref()
                .is_some_and(|default| !efforts.contains(default))
            {
                default_effort = None;
            }
            let is_default = capabilities
                .get("is_default")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);

            Ok(ModelCatalogOption {
                id: model_id,
                label,
                description,
                provider_type: p_type,
                is_default,
                default_effort,
                efforts,
                supports_fast_mode: supports_fast_mode != 0,
                context_window: context_window.map(|w| w as u64),
                pricing,
            })
        };

        let mut list = Vec::new();
        if let Some(pid) = provider_id {
            let rows = stmt
                .query_map(params![pid], map_fn)
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            for r in rows {
                list.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
            }
        } else {
            let rows = stmt
                .query_map([], map_fn)
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            for r in rows {
                list.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
            }
        }

        Ok(list)
    }
}
