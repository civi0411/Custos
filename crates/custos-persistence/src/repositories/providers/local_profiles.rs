use super::ProviderRepository;
use custos_domain::{
    DomainError, LocalModelDescriptor, LocalModelFolder, LocalModelLoadProfile, ModelCompatibility,
};
use rusqlite::params;

impl ProviderRepository {
    pub fn save_local_folder(&self, folder: &LocalModelFolder) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute(
            "INSERT INTO local_model_folders(id, folder_path, label, is_active, auto_scan, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                folder_path = excluded.folder_path,
                label = excluded.label,
                is_active = excluded.is_active,
                auto_scan = excluded.auto_scan,
                updated_at = excluded.updated_at",
            params![
                folder.id,
                folder.folder_path,
                folder.label,
                folder.is_active as i32,
                folder.auto_scan as i32,
                folder.created_at,
                folder.updated_at,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub fn list_local_folders(&self) -> Result<Vec<LocalModelFolder>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare("SELECT id, folder_path, label, is_active, auto_scan, created_at, updated_at FROM local_model_folders ORDER BY created_at ASC")
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(LocalModelFolder {
                    id: row.get(0)?,
                    folder_path: row.get(1)?,
                    label: row.get(2)?,
                    is_active: row.get::<_, i32>(3)? != 0,
                    auto_scan: row.get::<_, i32>(4)? != 0,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(list)
    }

    pub fn get_local_folder(&self, id: &str) -> Result<Option<LocalModelFolder>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare("SELECT id, folder_path, label, is_active, auto_scan, created_at, updated_at FROM local_model_folders WHERE id = ?1")
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let mut rows = stmt
            .query_map(params![id], |row| {
                Ok(LocalModelFolder {
                    id: row.get(0)?,
                    folder_path: row.get(1)?,
                    label: row.get(2)?,
                    is_active: row.get::<_, i32>(3)? != 0,
                    auto_scan: row.get::<_, i32>(4)? != 0,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        if let Some(r) = rows.next() {
            Ok(Some(r.map_err(|e| DomainError::Validation(e.to_string()))?))
        } else {
            Ok(None)
        }
    }

    pub fn delete_local_folder(&self, id: &str) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute(
            "DELETE FROM local_model_folders WHERE id = ?1",
            params![id],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub fn upsert_cached_model(&self, model: &LocalModelDescriptor) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute(
            "INSERT INTO local_model_cache(
                file_path, folder_id, file_name, file_size_bytes, file_mtime,
                architecture, model_name, context_length, quantization,
                chat_template, total_memory_required, compatibility, scanned_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(file_path) DO UPDATE SET
                folder_id = excluded.folder_id,
                file_name = excluded.file_name,
                file_size_bytes = excluded.file_size_bytes,
                file_mtime = excluded.file_mtime,
                architecture = excluded.architecture,
                model_name = excluded.model_name,
                context_length = excluded.context_length,
                quantization = excluded.quantization,
                chat_template = excluded.chat_template,
                total_memory_required = excluded.total_memory_required,
                compatibility = excluded.compatibility,
                scanned_at = excluded.scanned_at",
            params![
                model.file_path,
                model.folder_id,
                model.file_name,
                model.file_size_bytes as i64,
                model.file_mtime,
                model.architecture,
                model.model_name,
                model.context_length as i64,
                model.quantization,
                model.chat_template,
                model.total_memory_required as i64,
                match model.compatibility {
                    ModelCompatibility::Green => "green",
                    ModelCompatibility::Yellow => "yellow",
                    ModelCompatibility::Red => "red",
                },
                model.scanned_at,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }

    pub fn list_cached_models(
        &self,
        folder_id: Option<&str>,
    ) -> Result<Vec<LocalModelDescriptor>, DomainError> {
        let conn = self.db.lock()?;
        let (query, has_folder) = if folder_id.is_some() {
            (
                "SELECT file_path, folder_id, file_name, file_size_bytes, file_mtime,
                        architecture, model_name, context_length, quantization,
                        chat_template, total_memory_required, compatibility, scanned_at
                 FROM local_model_cache WHERE folder_id = ?1 ORDER BY model_name ASC",
                true,
            )
        } else {
            (
                "SELECT file_path, folder_id, file_name, file_size_bytes, file_mtime,
                        architecture, model_name, context_length, quantization,
                        chat_template, total_memory_required, compatibility, scanned_at
                 FROM local_model_cache ORDER BY model_name ASC",
                false,
            )
        };
        let mut stmt = conn
            .prepare(query)
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let mapper = |row: &rusqlite::Row| {
            let compat_str: String = row.get(11)?;
            let compatibility = match compat_str.as_str() {
                "green" => ModelCompatibility::Green,
                "yellow" => ModelCompatibility::Yellow,
                _ => ModelCompatibility::Red,
            };
            Ok(LocalModelDescriptor {
                file_path: row.get(0)?,
                folder_id: row.get(1)?,
                file_name: row.get(2)?,
                file_size_bytes: row.get::<_, i64>(3)? as u64,
                file_mtime: row.get(4)?,
                architecture: row.get(5)?,
                model_name: row.get(6)?,
                context_length: row.get::<_, i64>(7)? as u64,
                quantization: row.get(8)?,
                chat_template: row.get(9)?,
                total_memory_required: row.get::<_, i64>(10)? as u64,
                compatibility,
                scanned_at: row.get(12)?,
            })
        };
        let rows = if has_folder {
            stmt.query_map(params![folder_id.unwrap()], mapper)
        } else {
            stmt.query_map([], mapper)
        }
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(|e| DomainError::Validation(e.to_string()))?);
        }
        Ok(list)
    }

    pub fn get_cached_model_by_path(
        &self,
        file_path: &str,
    ) -> Result<Option<LocalModelDescriptor>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT file_path, folder_id, file_name, file_size_bytes, file_mtime,
                        architecture, model_name, context_length, quantization,
                        chat_template, total_memory_required, compatibility, scanned_at
                 FROM local_model_cache WHERE file_path = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let mut rows = stmt
            .query_map(params![file_path], |row| {
                let compat_str: String = row.get(11)?;
                let compatibility = match compat_str.as_str() {
                    "green" => ModelCompatibility::Green,
                    "yellow" => ModelCompatibility::Yellow,
                    _ => ModelCompatibility::Red,
                };
                Ok(LocalModelDescriptor {
                    file_path: row.get(0)?,
                    folder_id: row.get(1)?,
                    file_name: row.get(2)?,
                    file_size_bytes: row.get::<_, i64>(3)? as u64,
                    file_mtime: row.get(4)?,
                    architecture: row.get(5)?,
                    model_name: row.get(6)?,
                    context_length: row.get::<_, i64>(7)? as u64,
                    quantization: row.get(8)?,
                    chat_template: row.get(9)?,
                    total_memory_required: row.get::<_, i64>(10)? as u64,
                    compatibility,
                    scanned_at: row.get(12)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        if let Some(r) = rows.next() {
            Ok(Some(r.map_err(|e| DomainError::Validation(e.to_string()))?))
        } else {
            Ok(None)
        }
    }

    pub fn get_local_model_load_profile(
        &self,
        model_path: &str,
    ) -> Result<Option<LocalModelLoadProfile>, DomainError> {
        let conn = self.db.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT model_path, revision, context_size, gpu_mode, gpu_layers,
                        eval_batch_size, cpu_threads, flash_attention, mmap, mlock,
                        idle_timeout_seconds, updated_at
                 FROM local_model_load_profiles WHERE model_path = ?1",
            )
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        let mut rows = stmt
            .query_map(params![model_path], |row| {
                let flash_attention_i: Option<i32> = row.get(7)?;
                Ok(LocalModelLoadProfile {
                    model_path: row.get(0)?,
                    revision: row.get::<_, i64>(1)? as u64,
                    context_size: row.get::<_, i64>(2)? as u64,
                    gpu_mode: row.get(3)?,
                    gpu_layers: row.get::<_, Option<i64>>(4)?.map(|v| v as u32),
                    eval_batch_size: row.get::<_, Option<i64>>(5)?.map(|v| v as u32),
                    cpu_threads: row.get::<_, Option<i64>>(6)?.map(|v| v as u32),
                    flash_attention: flash_attention_i.map(|v| v != 0),
                    mmap: row.get::<_, i32>(8)? != 0,
                    mlock: row.get::<_, i32>(9)? != 0,
                    idle_timeout_seconds: row.get::<_, Option<i64>>(10)?.map(|v| v as u64),
                    updated_at: row.get(11)?,
                })
            })
            .map_err(|e| DomainError::Validation(e.to_string()))?;
        if let Some(r) = rows.next() {
            Ok(Some(r.map_err(|e| DomainError::Validation(e.to_string()))?))
        } else {
            Ok(None)
        }
    }

    pub fn save_local_model_load_profile(
        &self,
        profile: &LocalModelLoadProfile,
    ) -> Result<(), DomainError> {
        let conn = self.db.lock()?;
        conn.execute(
            "INSERT INTO local_model_load_profiles(model_path, revision, context_size, gpu_mode,
                gpu_layers, eval_batch_size, cpu_threads, flash_attention, mmap, mlock,
                idle_timeout_seconds, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(model_path) DO UPDATE SET revision=excluded.revision,
                context_size=excluded.context_size, gpu_mode=excluded.gpu_mode,
                gpu_layers=excluded.gpu_layers, eval_batch_size=excluded.eval_batch_size,
                cpu_threads=excluded.cpu_threads, flash_attention=excluded.flash_attention,
                mmap=excluded.mmap, mlock=excluded.mlock,
                idle_timeout_seconds=excluded.idle_timeout_seconds, updated_at=excluded.updated_at",
            params![
                profile.model_path,
                profile.revision as i64,
                profile.context_size as i64,
                profile.gpu_mode,
                profile.gpu_layers.map(i64::from),
                profile.eval_batch_size.map(i64::from),
                profile.cpu_threads.map(i64::from),
                profile.flash_attention.map(i32::from),
                profile.mmap as i32,
                profile.mlock as i32,
                profile.idle_timeout_seconds.map(|v| v as i64),
                profile.updated_at,
            ],
        )
        .map_err(|e| DomainError::Validation(e.to_string()))?;
        Ok(())
    }
}
