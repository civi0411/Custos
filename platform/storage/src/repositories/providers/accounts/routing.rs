use super::super::ProviderRepository;
use custos_domain::DomainError;
use rusqlite::params;

impl ProviderRepository {
    pub fn routing_policy(&self, service: &str) -> Result<serde_json::Value, DomainError> {
        use rusqlite::OptionalExtension;
        let conn = self.db.lock()?;
        let policy: Option<(bool, String)> = conn
            .query_row(
                "SELECT round_robin, account_order FROM provider_account_routing WHERE service_type=?1",
                params![service],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let (enabled, order) = policy.unwrap_or_else(|| {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM providers_config WHERE (service_type=?1 OR (service_type IN ('antigravity','gemini') AND ?1 IN ('antigravity','gemini'))) AND status IN ('active','configured','primary','standby')",
                    params![service],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            (count > 1, "[]".into())
        });
        let order: Vec<String> = serde_json::from_str(&order)
            .map_err(|_| DomainError::Validation("Invalid account routing policy".into()))?;
        Ok(serde_json::json!({"service_type":service,"round_robin":enabled,"account_order":order}))
    }

    pub fn save_routing_policy(
        &self,
        service: &str,
        enabled: bool,
        order: &[String],
    ) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let mut seen = std::collections::HashSet::new();
        for account in order {
            let matches: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM providers_config WHERE id=?1 AND (service_type=?2 OR (service_type IN ('antigravity','gemini') AND ?2 IN ('antigravity','gemini'))))",
                    params![account, service],
                    |row| row.get(0),
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            if !matches || !seen.insert(account) {
                return Err(DomainError::Validation(
                    "Account order must contain distinct accounts belonging to this provider"
                        .into(),
                ));
            }
        }
        let encoded =
            serde_json::to_string(order).map_err(|e| DomainError::Validation(e.to_string()))?;
        tx.execute(
            "INSERT INTO provider_account_routing(service_type,round_robin,account_order,cursor) VALUES(?1,?2,?3,0) ON CONFLICT(service_type) DO UPDATE SET round_robin=excluded.round_robin,account_order=excluded.account_order,cursor=0",
            params![service, enabled, encoded],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Self::record_provider_event(
            &tx,
            "provider.routing.updated",
            service,
            service,
            chrono::Utc::now().timestamp_millis(),
        )?;
        tx.commit()
            .map_err(|e| DomainError::Validation(e.to_string()))
    }

    pub fn select_account(
        &self,
        service: &str,
        eligible: &[String],
    ) -> Result<Option<String>, DomainError> {
        use rusqlite::OptionalExtension;
        if eligible.is_empty() {
            return Ok(None);
        }
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let row: Option<(bool, String, i64)> = tx
            .query_row(
                "SELECT round_robin,account_order,cursor FROM provider_account_routing WHERE service_type=?1",
                params![service],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let mut pool = Vec::new();
        for id in eligible {
            let available: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM providers_config p WHERE p.id=?1 AND (p.service_type=?2 OR (p.service_type IN ('antigravity','gemini') AND ?2 IN ('antigravity','gemini'))) AND p.status IN ('active','configured','primary','standby') AND (?2='local' OR EXISTS(SELECT 1 FROM provider_credentials c WHERE c.provider_id=p.id AND c.state='active')))",
                    params![id, service],
                    |r| r.get(0),
                )
                .map_err(|e| DomainError::Validation(e.to_string()))?;
            if available && !pool.contains(id) {
                pool.push(id.clone());
            }
        }
        if pool.is_empty() {
            return Err(DomainError::Validation(
                "No eligible provider account remains available".into(),
            ));
        }
        let (enabled, encoded, cursor) = row.unwrap_or_else(|| (pool.len() > 1, "[]".into(), 0));
        let order: Vec<String> = serde_json::from_str(&encoded)
            .map_err(|_| DomainError::Validation("Invalid account routing policy".into()))?;
        pool.sort_by_key(|id| {
            order
                .iter()
                .position(|candidate| candidate == id)
                .unwrap_or(usize::MAX)
        });
        let index = if enabled {
            (cursor.max(0) as usize) % pool.len()
        } else {
            0
        };
        let selected = pool[index].clone();
        if enabled {
            tx.execute(
                "INSERT INTO provider_account_routing(service_type,round_robin,account_order,cursor) VALUES(?1,1,'[]',?2) ON CONFLICT(service_type) DO UPDATE SET cursor=?2",
                params![service, ((index + 1) % pool.len()) as i64],
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        }
        Self::record_provider_event(
            &tx,
            "provider.account.selected",
            &selected,
            service,
            chrono::Utc::now().timestamp_millis(),
        )?;
        tx.commit()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(Some(selected))
    }
}

