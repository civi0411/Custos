pub mod accounts;
pub mod catalog;
pub mod local_profiles;
pub mod usage;

#[allow(unused_imports)]
pub use accounts::*;
#[allow(unused_imports)]
pub use catalog::*;
#[allow(unused_imports)]
pub use local_profiles::*;
#[allow(unused_imports)]
pub use usage::*;

use crate::connection::DbConnection;

#[derive(Clone)]
pub struct ProviderRepository {
    pub(crate) db: DbConnection,
}

impl ProviderRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }
}

#[cfg(test)]
mod catalog_snapshot_tests {
    use custos_domain::{
        LocalModelDescriptor, LocalModelFolder, LocalModelLoadProfile, ModelCatalogOption,
        ModelCompatibility, OAuthTokenRecord, ProviderConfig,
    };

    #[test]
    fn oauth_connection_is_atomic_and_outbox_is_redacted() {
        let store = crate::SqliteTaskStore::new_in_memory().unwrap();
        let repo = store.providers();
        let token = OAuthTokenRecord {
            provider_id: "antigravity".into(),
            service_type: "antigravity".into(),
            access_token: "secret-access".into(),
            refresh_token: Some("secret-refresh".into()),
            expires_at: 10000,
            token_type: "Bearer".into(),
            scope: Some("cloud-platform".into()),
            created_at: 1,
            updated_at: 1,
        };
        let config = ProviderConfig {
            id: "antigravity".into(),
            service_type: "antigravity".into(),
            name: "person@example.com".into(),
            api_key_masked: "oauth".into(),
            status: "active".into(),
            endpoint_url: Some("https://cloudcode-pa.googleapis.com".into()),
            default_model: None,
            context_window: None,
            fast_mode: None,
            created_at: 1,
            updated_at: 1,
        };
        repo.save_oauth_connection(&token, &config).unwrap();
        assert_eq!(
            repo.get_provider("antigravity").unwrap().unwrap().name,
            "person@example.com"
        );
        {
            let conn = repo.db.lock().unwrap();
            let payload: String = conn
                .query_row(
                    "SELECT payload FROM outbox_messages WHERE topic='provider.oauth.connected'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert!(!payload.contains("secret-access"));
            assert!(!payload.contains("secret-refresh"));
            conn.execute_batch("CREATE TRIGGER reject_provider BEFORE INSERT ON providers_config BEGIN SELECT RAISE(ABORT, 'test rejection'); END;").unwrap();
        }
        let mut changed = token.clone();
        changed.access_token = "replacement".into();
        assert!(repo.save_oauth_connection(&changed, &config).is_err());
        assert_eq!(
            repo.get_oauth_token("antigravity")
                .unwrap()
                .unwrap()
                .access_token,
            "secret-access"
        );
    }

    #[test]
    fn verified_snapshot_replaces_models_and_records_outbox_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let store =
            crate::SqliteTaskStore::new(dir.path().join("catalog.db").to_str().unwrap()).unwrap();
        let repo = store.providers();
        let model = |id: &str| ModelCatalogOption {
            id: id.into(),
            label: id.into(),
            description: None,
            provider_type: "openai".into(),
            is_default: false,
            default_effort: None,
            efforts: vec![],
            supports_fast_mode: false,
            context_window: None,
            pricing: None,
        };
        repo.save_catalog_models("verified-probe:account", &[model("removed"), model("kept")])
            .unwrap();
        let timestamp = repo.catalog_fetched_at("verified-probe:account").unwrap();
        assert!(timestamp > 0);
        repo.save_catalog_models("verified-probe:account", &[model("kept")])
            .unwrap();
        let cached = repo
            .list_catalog_models(Some("verified-probe:account"))
            .unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].id, "kept");
        assert!(repo
            .list_catalog_models(Some("other-account"))
            .unwrap()
            .is_empty());
        let conn = repo.db.lock().unwrap();
        let events: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM outbox_messages WHERE topic = 'provider.models.probed'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(events, 2);
    }

    #[test]
    fn routing_policy_and_cursor_survive_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("routing.db");
        {
            let store = crate::SqliteTaskStore::new(path.to_str().unwrap()).unwrap();
            let repo = store.providers();
            for id in ["A", "B"] {
                let config = ProviderConfig {
                    id: id.into(),
                    name: id.into(),
                    service_type: "local".into(),
                    api_key_masked: "none".into(),
                    status: "active".into(),
                    endpoint_url: None,
                    default_model: None,
                    context_window: None,
                    fast_mode: None,
                    created_at: 0,
                    updated_at: 0,
                };
                repo.save_provider(&config).unwrap();
            }
            repo.save_routing_policy("local", true, &["B".into(), "A".into()])
                .unwrap();
            assert_eq!(
                repo.select_account("local", &["A".into(), "B".into()])
                    .unwrap()
                    .as_deref(),
                Some("B")
            );
        }
        let store = crate::SqliteTaskStore::new(path.to_str().unwrap()).unwrap();
        let repo = store.providers();
        assert_eq!(repo.routing_policy("local").unwrap()["round_robin"], true);
        assert_eq!(
            repo.select_account("local", &["A".into(), "B".into()])
                .unwrap()
                .as_deref(),
            Some("A")
        );
        repo.save_routing_policy("local", false, &["A".into(), "B".into()])
            .unwrap();
        assert_eq!(
            repo.select_account("local", &["A".into(), "B".into()])
                .unwrap()
                .as_deref(),
            Some("A")
        );
        let conn = repo.db.lock().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM outbox_messages WHERE topic='provider.account.selected'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn local_model_folders_and_cache_crud() {
        let store = crate::SqliteTaskStore::new_in_memory().unwrap();
        let repo = store.providers();
        let folder = LocalModelFolder {
            id: "fld-1".into(),
            folder_path: "/tmp/models".into(),
            label: Some("Test Folder".into()),
            is_active: true,
            auto_scan: true,
            created_at: 100,
            updated_at: 100,
        };
        repo.save_local_folder(&folder).unwrap();
        let list = repo.list_local_folders().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "fld-1");
        assert_eq!(list[0].folder_path, "/tmp/models");

        let model = LocalModelDescriptor {
            file_path: "/tmp/models/qwen2.5-7b.gguf".into(),
            folder_id: "fld-1".into(),
            file_name: "qwen2.5-7b.gguf".into(),
            file_size_bytes: 4_500_000_000,
            file_mtime: 200,
            architecture: "qwen2".into(),
            model_name: "Qwen2.5-7B-Instruct".into(),
            context_length: 32768,
            quantization: "Q4_K_M".into(),
            chat_template: Some("<chat_template>".into()),
            total_memory_required: 5_200_000_000,
            compatibility: ModelCompatibility::Green,
            scanned_at: 300,
        };
        repo.upsert_cached_model(&model).unwrap();
        let cached = repo.list_cached_models(None).unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].model_name, "Qwen2.5-7B-Instruct");
        assert_eq!(cached[0].compatibility, ModelCompatibility::Green);

        let by_path = repo
            .get_cached_model_by_path("/tmp/models/qwen2.5-7b.gguf")
            .unwrap()
            .unwrap();
        assert_eq!(by_path.file_size_bytes, 4_500_000_000);

        let profile = LocalModelLoadProfile {
            model_path: model.file_path.clone(),
            revision: 1,
            context_size: 4096,
            gpu_mode: "auto".into(),
            gpu_layers: None,
            eval_batch_size: Some(512),
            cpu_threads: Some(8),
            flash_attention: Some(true),
            mmap: true,
            mlock: false,
            idle_timeout_seconds: Some(900),
            updated_at: 400,
        };
        repo.save_local_model_load_profile(&profile).unwrap();
        assert_eq!(
            repo.get_local_model_load_profile(&model.file_path)
                .unwrap()
                .unwrap(),
            profile
        );

        repo.delete_local_folder("fld-1").unwrap();
        assert!(repo.list_local_folders().unwrap().is_empty());
        assert!(repo.list_cached_models(None).unwrap().is_empty());
        assert!(repo
            .get_local_model_load_profile(&model.file_path)
            .unwrap()
            .is_none());
    }
}

#[cfg(test)]
mod catalog_capability_tests {
    use custos_domain::ModelCatalogOption;

    #[test]
    fn exact_capabilities_survive_restart_and_empty_refresh() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("catalog-capabilities.db");
        let model = ModelCatalogOption {
            id: "gpt-6.1-sol".into(),
            label: "GPT-6.1 Sol".into(),
            provider_type: "openai".into(),
            description: None,
            is_default: true,
            default_effort: Some("low".into()),
            efforts: vec![
                "low".into(),
                "xhigh".into(),
                "max".into(),
                "ultra".into(),
            ],
            supports_fast_mode: false,
            context_window: Some(272000),
            pricing: None,
        };
        {
            let store = crate::SqliteTaskStore::new(path.to_str().unwrap()).unwrap();
            store
                .providers()
                .save_catalog_models("account-scope", std::slice::from_ref(&model))
                .unwrap();
        }
        let store = crate::SqliteTaskStore::new(path.to_str().unwrap()).unwrap();
        let saved = store
            .providers()
            .list_catalog_models(Some("account-scope"))
            .unwrap();
        assert_eq!(saved[0].efforts, model.efforts);
        assert_eq!(saved[0].default_effort, model.default_effort);
        assert!(saved[0].is_default);
        store
            .providers()
            .save_catalog_models("account-scope", &[])
            .unwrap();
        assert!(store
            .providers()
            .list_catalog_models(Some("account-scope"))
            .unwrap()
            .is_empty());
    }

    #[test]
    fn legacy_catalog_migration_is_idempotent_and_does_not_invent_efforts() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!(
            "../../../migrations/0025_provider_model_catalog.sql"
        ))
        .unwrap();
        conn.execute("INSERT INTO provider_model_catalog(id,provider_id,model_id,label,default_effort,fetched_at) VALUES('legacy','A','old','Old','medium',1)",[]).unwrap();
        crate::migrations::run_migrations(&conn).unwrap();
        crate::migrations::run_migrations(&conn).unwrap();
        let metadata: String = conn
            .query_row(
                "SELECT capabilities_json FROM provider_model_catalog WHERE id='legacy'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(metadata, "{}");
    }
}
