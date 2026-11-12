use super::*;
use custos_persistence::SqliteTaskStore;
use custos_provider::routing::router::CredentialResolver;
use std::sync::Arc;

fn token(oauth: bool) -> custos_domain::OAuthTokenRecord {
    custos_domain::OAuthTokenRecord {
        provider_id: "openai".into(),
        service_type: "openai".into(),
        access_token: if oauth { "opaque-oauth" } else { "eyJ-api-key" }.into(),
        refresh_token: if oauth { Some("refresh".into()) } else { None },
        expires_at: i64::MAX / 2,
        token_type: "bearer".into(),
        scope: None,
        created_at: 1,
        updated_at: 1,
    }
}

#[tokio::test]
async fn credential_metadata_controls_authentication_not_token_prefix() {
    let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
    let resolver = DaemonCredentialResolver {
        availability: Default::default(),
        store: store.clone(),
        credentials: crate::runtime::CustosRuntime::credential_service(&store),
    };
    resolver
        .credentials
        .save_oauth_token(&token(true))
        .await
        .unwrap();
    assert!(resolver.uses_oauth("openai").await.unwrap());
    assert_eq!(
        resolver.get_token("openai").await.unwrap().unwrap(),
        "opaque-oauth"
    );
    resolver
        .credentials
        .save_oauth_token(&token(false))
        .await
        .unwrap();
    assert!(!resolver.uses_oauth("openai").await.unwrap());
    assert_eq!(
        resolver.get_token("openai").await.unwrap().unwrap(),
        "eyJ-api-key"
    );
}

#[tokio::test]
async fn expired_oauth_without_refresh_fails_closed() {
    let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
    let resolver = DaemonCredentialResolver {
        availability: Default::default(),
        store: store.clone(),
        credentials: crate::runtime::CustosRuntime::credential_service(&store),
    };
    let mut expired = token(false);
    expired.scope = Some("openid".into());
    expired.expires_at = 0;
    resolver
        .credentials
        .save_oauth_token(&expired)
        .await
        .unwrap();
    assert!(resolver.uses_oauth("openai").await.unwrap());
    assert!(resolver
        .get_token("openai")
        .await
        .unwrap_err()
        .to_string()
        .contains("expired"));
}

#[tokio::test]
async fn account_pool_priority_rotation_catalog_binding_and_delete() {
    let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
    let service = Arc::new(crate::credentials::ProviderCredentials::new(
        Arc::new(store.providers().clone()),
        Arc::new(crate::credential_tests::FakeVault::default()),
    ));
    let repo = store.providers();
    for id in ["A", "B", "C"] {
        let config = custos_domain::ProviderConfig {
            id: id.into(),
            name: id.into(),
            service_type: "openai".into(),
            api_key_masked: "none".into(),
            status: "active".into(),
            endpoint_url: Some("https://api.openai.com/v1".into()),
            default_model: None,
            context_window: None,
            fast_mode: None,
            created_at: 0,
            updated_at: 0,
        };
        service
            .save_provider(&config, Some(&format!("fixture-{id}")))
            .await
            .unwrap();
        let meta = repo.credential_metadata(id).unwrap().unwrap();
        let key = format!(
            "verified-probe:{id}:https://api.openai.com/v1:api_key:{}",
            meta.created_at
        );
        let model = custos_domain::ModelCatalogOption {
            id: if id == "C" { "gpt-other" } else { "gpt-test" }.into(),
            label: id.into(),
            provider_type: "openai".into(),
            description: None,
            is_default: false,
            default_effort: None,
            efforts: vec![],
            supports_fast_mode: false,
            context_window: None,
            pricing: None,
        };
        repo.save_catalog_models(&key, &[model]).unwrap();
    }
    assert_eq!(
        reserve_provider_account(repo, "openai", "gpt-test")
            .unwrap()
            .as_deref(),
        Some("A")
    );
    repo.save_routing_policy("openai", false, &["B".into(), "A".into(), "C".into()])
        .unwrap();
    for _ in 0..3 {
        assert_eq!(
            reserve_provider_account(repo, "openai", "gpt-test")
                .unwrap()
                .as_deref(),
            Some("B")
        );
    }
    repo.save_routing_policy("openai", true, &["B".into(), "A".into(), "C".into()])
        .unwrap();
    for expected in ["B", "A", "B", "A"] {
        assert_eq!(
            reserve_provider_account(repo, "openai", "gpt-test")
                .unwrap()
                .as_deref(),
            Some(expected)
        );
    }
    assert!(repo
        .save_routing_policy("openai", true, &["A".into(), "A".into()])
        .is_err());
    assert!(repo
        .save_routing_policy("copilot", true, &["A".into()])
        .is_err());
    assert!(reserve_provider_account(repo, "openai", "missing").is_err());
    service.delete("B", true).await.unwrap();
    assert_eq!(
        reserve_provider_account(repo, "openai", "gpt-test")
            .unwrap()
            .as_deref(),
        Some("A")
    );
    assert_eq!(
        service
            .get_oauth_token("A")
            .await
            .unwrap()
            .unwrap()
            .access_token,
        "fixture-A"
    );
    let mut config = repo.get_provider("A").unwrap().unwrap();
    config.endpoint_url = Some("https://different.invalid/v1".into());
    repo.save_provider(&config).unwrap();
    assert!(reserve_provider_account(repo, "openai", "gpt-test").is_err());
}

#[tokio::test]
async fn account_pool_concurrent_reservations_are_balanced_and_locked_selection_fails_closed() {
    let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
    let vault = Arc::new(crate::credential_tests::FakeVault::default());
    let service = Arc::new(crate::credentials::ProviderCredentials::new(
        Arc::new(store.providers().clone()),
        vault.clone(),
    ));
    let repo = store.providers();
    for id in ["A", "B"] {
        let config = custos_domain::ProviderConfig {
            id: id.into(),
            name: id.into(),
            service_type: "copilot".into(),
            api_key_masked: "none".into(),
            status: "active".into(),
            endpoint_url: Some("https://api.githubcopilot.com".into()),
            default_model: None,
            context_window: None,
            fast_mode: None,
            created_at: 0,
            updated_at: 0,
        };
        let mut tok = token(true);
        tok.provider_id = id.into();
        tok.service_type = "copilot".into();
        service
            .save_oauth_connection(&tok, Some(&config))
            .await
            .unwrap();
        let key = format!(
            "verified-probe:{id}:https://api.githubcopilot.com:oauth:{}",
            tok.created_at
        );
        let model = custos_domain::ModelCatalogOption {
            id: "copilot/claude-test".into(),
            label: "Claude".into(),
            provider_type: "copilot".into(),
            description: None,
            is_default: false,
            default_effort: None,
            efforts: vec![],
            supports_fast_mode: false,
            context_window: None,
            pricing: None,
        };
        repo.save_catalog_models(&key, &[model]).unwrap();
    }
    repo.save_routing_policy("copilot", true, &["A".into(), "B".into()])
        .unwrap();
    let mut handles = vec![];
    for _ in 0..20 {
        let repo = (*repo).clone();
        handles.push(tokio::task::spawn_blocking(move || {
            reserve_provider_account(&repo, "copilot", "copilot/claude-test")
                .unwrap()
                .unwrap()
        }));
    }
    let mut a = 0;
    for handle in handles {
        if handle.await.unwrap() == "A" {
            a += 1;
        }
    }
    assert_eq!(a, 10);
    vault
        .fail_read
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let resolver = DaemonCredentialResolver {
        store,
        credentials: service,
        availability: Default::default(),
    };
    let selected = resolver
        .select_account("copilot", "copilot/claude-test")
        .await
        .unwrap()
        .unwrap();
    assert!(resolver.get_token(&selected).await.is_err());
    assert!(resolver
        .validate_account(&selected, "anthropic")
        .await
        .is_err());
}

#[tokio::test]
async fn reasoning_admission_uses_the_selected_account_catalog_and_scope() {
    let store = Arc::new(SqliteTaskStore::new_in_memory().unwrap());
    let credentials = Arc::new(crate::credentials::ProviderCredentials::new(
        Arc::new(store.providers().clone()),
        Arc::new(crate::credential_tests::FakeVault::default()),
    ));
    let availability = custos_provider::routing::availability::ModelAvailability::default();
    let resolver = DaemonCredentialResolver {
        store: store.clone(),
        credentials: credentials.clone(),
        availability: availability.clone(),
    };
    for account in ["A", "B"] {
        let config = custos_domain::ProviderConfig {
            id: account.into(),
            name: account.into(),
            service_type: "openai".into(),
            api_key_masked: "none".into(),
            status: "active".into(),
            endpoint_url: Some("https://api.openai.com/v1".into()),
            default_model: None,
            context_window: None,
            fast_mode: None,
            created_at: 0,
            updated_at: 0,
        };
        credentials
            .save_provider(&config, Some("fixture-key"))
            .await
            .unwrap();
        let scope = provider_catalog_scope(store.providers(), account).unwrap();
        let model = custos_domain::ModelCatalogOption {
            id: "gpt-6.1-sol".into(),
            label: "GPT-6.1 Sol".into(),
            provider_type: "openai".into(),
            description: None,
            is_default: true,
            default_effort: Some("low".into()),
            efforts: if account == "A" {
                vec!["low".into(), "xhigh".into(), "max".into(), "ultra".into()]
            } else {
                vec!["low".into()]
            },
            supports_fast_mode: false,
            context_window: None,
            pricing: None,
        };
        store
            .providers()
            .save_catalog_models(&scope, &[model])
            .unwrap();
    }
    store
        .providers()
        .save_routing_policy("openai", false, &["B".into(), "A".into()])
        .unwrap();
    assert_eq!(
        reserve_available_provider_account(
            store.providers(),
            "openai",
            "gpt-6.1-sol",
            Some("low"),
            &availability
        )
        .unwrap()
        .as_deref(),
        Some("B")
    );
    assert_eq!(
        reserve_available_provider_account(
            store.providers(),
            "openai",
            "gpt-6.1-sol",
            Some("ultra"),
            &availability
        )
        .unwrap()
        .as_deref(),
        Some("A")
    );
    let b_scope = provider_catalog_scope(store.providers(), "B").unwrap();
    availability
        .observe(
            &b_scope,
            "gpt-6.1-sol",
            403,
            r#"{"error":{"code":"model_not_found"}}"#,
            None,
        )
        .unwrap();
    assert_eq!(
        reserve_available_provider_account(
            store.providers(),
            "openai",
            "gpt-6.1-sol",
            Some("low"),
            &availability
        )
        .unwrap()
        .as_deref(),
        Some("A")
    );
    availability.refresh(&b_scope).unwrap();
    assert!(resolver
        .validate_model("A", "gpt-6.1-sol", Some("ultra"))
        .await
        .is_ok());
    assert!(resolver
        .validate_model("B", "gpt-6.1-sol", Some("ultra"))
        .await
        .is_err());
    assert!(resolver
        .validate_model("B", "gpt-6.1-sol", None)
        .await
        .is_ok());
    assert!(resolver.validate_model("A", "missing", None).await.is_err());
    resolver
        .observe_response(
            "A",
            "gpt-6.1-sol",
            429,
            r#"{"error":{"code":"insufficient_quota"}}"#,
            None,
        )
        .await
        .unwrap();
    assert!(resolver
        .validate_model("A", "gpt-6.1-sol", Some("low"))
        .await
        .is_err());
    assert!(resolver
        .validate_model("B", "gpt-6.1-sol", Some("low"))
        .await
        .is_ok());
    assert!(reserve_available_provider_account(
        store.providers(),
        "openai",
        "gpt-6.1-sol",
        Some("ultra"),
        &availability
    )
    .is_err());
    assert_eq!(
        reserve_available_provider_account(
            store.providers(),
            "openai",
            "gpt-6.1-sol",
            Some("low"),
            &availability
        )
        .unwrap()
        .as_deref(),
        Some("B")
    );
    availability
        .observe(
            &b_scope,
            "gpt-6.1-sol",
            429,
            r#"{"error":{"code":"insufficient_quota"}}"#,
            None,
        )
        .unwrap();
    assert!(reserve_available_provider_account(
        store.providers(),
        "openai",
        "gpt-6.1-sol",
        Some("low"),
        &availability
    )
    .is_err());
    availability.quota_recovered(&b_scope).unwrap();
    let scope = provider_catalog_scope(store.providers(), "A").unwrap();
    availability.refresh(&scope).unwrap();
    assert!(resolver
        .validate_model("A", "gpt-6.1-sol", None)
        .await
        .is_err());
    availability.quota_recovered(&scope).unwrap();
    assert!(resolver
        .validate_model("A", "gpt-6.1-sol", None)
        .await
        .is_ok());
    let mut record = store.providers().get_provider("A").unwrap().unwrap();
    record.endpoint_url = Some("https://different.invalid/v1".into());
    store.providers().save_provider(&record).unwrap();
    assert!(resolver
        .validate_model("A", "gpt-6.1-sol", None)
        .await
        .is_err());
}
