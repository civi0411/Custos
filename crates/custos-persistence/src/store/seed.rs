//! Canonical test data seeding for in-memory and verification tests.

use super::SqliteTaskStore;
use custos_domain::{DomainError, Task};

impl SqliteTaskStore {
    #[cfg(test)]
    pub fn seed_canonical_data_if_empty(&self) -> Result<(), DomainError> {
        // 1. Seed tasks if empty
        if self.task_repo.list_tasks()?.is_empty() {
            let mut task1 = Task::new(
                "task_inv_verify_01".to_string(),
                "Invariant Formal Verification Suite".to_string(),
            );
            task1.status = custos_domain::TaskStatus::Running;
            task1.contract = Some(custos_domain::TaskContract {
                pack_id: "engineering".to_string(),
                name: "Verification Contract".to_string(),
                description: "Verify invariants on multi-agent execution".to_string(),
                required_capabilities: vec!["analysis".to_string()],
                evidence_requirements: vec![],
            });
            self.task_repo.save_task(&task1)?;

            let mut task2 = Task::new(
                "task_worktree_02".to_string(),
                "Autonomous Worktree Pipeline".to_string(),
            );
            task2.status = custos_domain::TaskStatus::Draft;
            self.task_repo.save_task(&task2)?;

            let session_id = custos_domain::SessionId::new("sess_inv_01");
            let mut session = custos_domain::Session::new(
                session_id.clone(),
                custos_domain::SessionMode::Attached {
                    task_id: "task_inv_verify_01".to_string(),
                },
            );
            session.attached_to = Some("task_inv_verify_01".to_string());
            self.session_repo.save_session(&session)?;

            let now = chrono::Utc::now().to_rfc3339();
            self.session_repo
                .append_journal(&custos_domain::SessionJournalEntry {
                    entry_id: None,
                    session_id: session_id.clone(),
                    entry_type: "user".to_string(),
                    entry_data: "Initialize invariant verification bounds for multi-agent swarm"
                        .to_string(),
                    occurred_at: now.clone(),
                })?;
            self.session_repo
                .append_journal(&custos_domain::SessionJournalEntry {
                    entry_id: None,
                    session_id,
                    entry_type: "assistant".to_string(),
                    entry_data:
                        "Verification bounds initialized with MERKLE-CAS-SEALED invariant contract."
                            .to_string(),
                    occurred_at: now,
                })?;
        }

        // 2. Seed research sources & claims if empty
        if self.research_repo.list_sources()?.is_empty() {
            let now_ts = chrono::Utc::now().timestamp_millis();
            let src1 = custos_domain::SourceRecord {
                id: "src_nature_2024_01".to_string(),
                source_type: "paper".to_string(),
                title: "Self-Organizing Invariant Architectures in Deterministic Multi-Agent Swarms".to_string(),
                doi: Some("10.1038/s41586-024-07821-x".to_string()),
                authors: vec!["V. Pham".into(), "M. Chen".into(), "E. Vance".into()],
                year: Some(2024),
                content_hash: "blake3_9941a8e2f7b11c".to_string(),
                local_path: None,
                verified: true,
                abstract_text: Some("We present a zero-trust consensus mechanism bounding stochastic agent divergence using Merkle-sealed invariant contracts.".to_string()),
                created_at: now_ts - 7200000,
            };
            let src2 = custos_domain::SourceRecord {
                id: "src_arxiv_2025_02".to_string(),
                source_type: "paper".to_string(),
                title: "On the Convergence Rates of Cryptographic Capability Tickets under Asymmetric Latency".to_string(),
                doi: Some("10.48550/arXiv.2501.09912".to_string()),
                authors: vec!["T. Lindholm".into(), "K. S. Rao".into()],
                year: Some(2025),
                content_hash: "blake3_7718c091ad4e22".to_string(),
                local_path: None,
                verified: true,
                abstract_text: Some("This study provides lower bounds for atomic ticket acquisition across distributed authority gates.".to_string()),
                created_at: now_ts - 3600000,
            };
            self.research_repo.save_source(&src1)?;
            self.research_repo.save_source(&src2)?;

            let anchor = custos_domain::PassageAnchor {
                id: "anc_01".to_string(),
                source_id: "src_nature_2024_01".to_string(),
                source_title: Some("Self-Organizing Invariant Architectures".to_string()),
                section_title: Some("Section 4.2 Invariant Bounding".to_string()),
                page_number: Some(8),
                start_offset: 1240,
                end_offset: 1485,
                exact_text: "Phantom state execution was reduced by 99.8% across 10,000 runs."
                    .to_string(),
                passage_hash: "blake3_anc_4491c".to_string(),
            };
            self.research_repo.save_anchor(&anchor)?;

            let claim = custos_domain::ResearchClaim {
                id: "claim_01".to_string(),
                statement: "Phantom state execution in unconstrained LLM loops can be reduced by 99.8% using Merkle-sealed state invariants.".to_string(),
                level: custos_domain::ClaimGroundingLevel::L3Sealed,
                confidence_score: 0.99,
                invariants: vec!["INV-PHANTOM-STATE-BOUND".to_string(), "INV-CAS-SEALED".to_string()],
                sealed_proof_uri: Some("cas://bafy2bzace4v3k99a77x1198".to_string()),
                evidence_links: vec![
                    custos_domain::ClaimEvidenceLink {
                        passage_anchor_id: "anc_01".to_string(),
                        source_title: Some("Self-Organizing Invariant Architectures".to_string()),
                        exact_text: Some("Phantom state execution was reduced by 99.8% across 10,000 runs.".to_string()),
                        relation: custos_domain::EvidenceRelation::Supports,
                        rationale: "Empirically proven with deterministic clean-room replays across 10,000 runs.".to_string(),
                        verified_by: "deterministic_engine".to_string(),
                    }
                ],
                created_at: now_ts - 3600000,
            };
            self.research_repo.save_claim(&claim)?;
        }

        // 3. Seed default providers if empty
        if self.provider_repo.list_providers()?.is_empty() {
            let now = chrono::Utc::now().timestamp_millis();
            let p1 = custos_domain::ProviderConfig {
                id: "p_anthropic".to_string(),
                name: "Anthropic Claude (Sonnet 3.7)".to_string(),
                service_type: "anthropic".to_string(),
                api_key_masked: "sk-ant-••••••••".to_string(),
                status: "unconfigured".to_string(),
                endpoint_url: None,
                default_model: Some("claude-3-7-sonnet".to_string()),
                context_window: Some(200_000),
                fast_mode: Some(true),
                created_at: now,
                updated_at: now,
            };
            let p2 = custos_domain::ProviderConfig {
                id: "p_openai".to_string(),
                name: "OpenAI Codex / GPT-4o".to_string(),
                service_type: "openai".to_string(),
                api_key_masked: "sk-proj-••••••••".to_string(),
                status: "unconfigured".to_string(),
                endpoint_url: None,
                default_model: Some("gpt-4o".to_string()),
                context_window: Some(128_000),
                fast_mode: Some(true),
                created_at: now,
                updated_at: now,
            };
            let p3 = custos_domain::ProviderConfig {
                id: "p_gemini".to_string(),
                name: "Google Gemini 2.5 Flash".to_string(),
                service_type: "gemini".to_string(),
                api_key_masked: "AIzaSy••••••••".to_string(),
                status: "unconfigured".to_string(),
                endpoint_url: None,
                default_model: Some("gemini-2.5-flash".to_string()),
                context_window: Some(1_048_576),
                fast_mode: Some(true),
                created_at: now,
                updated_at: now,
            };
            let p4 = custos_domain::ProviderConfig {
                id: "p_local".to_string(),
                name: "Local Model (Ollama / llama.cpp)".to_string(),
                service_type: "local".to_string(),
                api_key_masked: "none".to_string(),
                status: "unconfigured".to_string(),
                endpoint_url: Some("http://127.0.0.1:11434".to_string()),
                default_model: Some("qwen2.5-coder:32b".to_string()),
                context_window: Some(32_768),
                fast_mode: Some(false),
                created_at: now,
                updated_at: now,
            };
            self.provider_repo.save_provider(&p1)?;
            self.provider_repo.save_provider(&p2)?;
            self.provider_repo.save_provider(&p3)?;
            self.provider_repo.save_provider(&p4)?;
        }

        // 4. Seed default client key if empty
        if self.provider_repo.list_client_keys()?.is_empty() {
            let k1 = custos_domain::ClientApiKeyRecord {
                id: "key_gateway_01".to_string(),
                name: "Custos Desktop Client Token".to_string(),
                token: "custos_live_sec_9941a8e2".to_string(),
                created_at: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                revoked: false,
            };
            self.provider_repo.create_client_key(&k1)?;
        }

        Ok(())
    }
}
