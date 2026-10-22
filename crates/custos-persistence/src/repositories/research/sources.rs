use super::ResearchRepository;
use custos_domain::{DomainError, PassageAnchor, SourceRecord};
use rusqlite::params;

impl ResearchRepository {
    // Sources
    pub(crate) fn save_source_conn(
        conn: &rusqlite::Connection,
        src: &SourceRecord,
    ) -> Result<(), DomainError> {
        let authors_json = serde_json::to_string(&src.authors)
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        conn.execute(
            "INSERT INTO research_sources (
                id, source_type, title, doi, authors_json, year, content_hash, local_path, verified, abstract_text, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                doi = excluded.doi,
                authors_json = excluded.authors_json,
                year = excluded.year,
                content_hash = excluded.content_hash,
                local_path = excluded.local_path,
                verified = excluded.verified,
                abstract_text = excluded.abstract_text",
            params![
                src.id,
                src.source_type,
                src.title,
                src.doi,
                authors_json,
                src.year,
                src.content_hash,
                src.local_path,
                if src.verified { 1 } else { 0 },
                src.abstract_text,
                src.created_at,
            ],
        ).map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn save_source(&self, src: &SourceRecord) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        Self::save_source_conn(&conn, src)
    }

    pub fn ingest_source_and_claim_atomic(
        &self,
        src: &SourceRecord,
        claim: &custos_domain::ResearchClaim,
    ) -> Result<(), DomainError> {
        let mut conn = self.db.lock()?;
        let tx = conn
            .transaction()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Self::save_source_conn(&tx, src)?;
        Self::save_claim_conn(&tx, claim)?;
        tx.commit()
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub fn list_sources(&self) -> Result<Vec<SourceRecord>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, source_type, title, doi, authors_json, year, content_hash, local_path, verified, abstract_text, created_at
                 FROM research_sources ORDER BY created_at DESC",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let authors_json: String = row.get(4)?;
                let authors: Vec<String> = serde_json::from_str(&authors_json).unwrap_or_default();
                let verified_int: i32 = row.get(8)?;

                Ok(SourceRecord {
                    id: row.get(0)?,
                    source_type: row.get(1)?,
                    title: row.get(2)?,
                    doi: row.get(3)?,
                    authors,
                    year: row.get(5)?,
                    content_hash: row.get(6)?,
                    local_path: row.get(7)?,
                    verified: verified_int == 1,
                    abstract_text: row.get(9)?,
                    created_at: row.get(10)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut res = Vec::new();
        for r in rows {
            res.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(res)
    }

    // Passage Anchors
    pub fn save_anchor(&self, anchor: &PassageAnchor) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute(
            "INSERT INTO research_passage_anchors (
                id, source_id, source_title, section_title, page_number, start_offset, end_offset, exact_text, passage_hash
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(id) DO UPDATE SET
                exact_text = excluded.exact_text,
                passage_hash = excluded.passage_hash",
            params![
                anchor.id,
                anchor.source_id,
                anchor.source_title,
                anchor.section_title,
                anchor.page_number,
                anchor.start_offset as i64,
                anchor.end_offset as i64,
                anchor.exact_text,
                anchor.passage_hash,
            ],
        ).map_err(|e| DomainError::Validation(e.to_string()))?;

        Ok(())
    }

    pub fn list_anchors_for_source(
        &self,
        source_id: &str,
    ) -> Result<Vec<PassageAnchor>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, source_id, source_title, section_title, page_number, start_offset, end_offset, exact_text, passage_hash
                 FROM research_passage_anchors WHERE source_id = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let rows = stmt
            .query_map(params![source_id], |row| {
                let start: i64 = row.get(5)?;
                let end: i64 = row.get(6)?;
                Ok(PassageAnchor {
                    id: row.get(0)?,
                    source_id: row.get(1)?,
                    source_title: row.get(2)?,
                    section_title: row.get(3)?,
                    page_number: row.get(4)?,
                    start_offset: start as usize,
                    end_offset: end as usize,
                    exact_text: row.get(7)?,
                    passage_hash: row.get(8)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;

        let mut res = Vec::new();
        for r in rows {
            res.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(res)
    }
}
