pub mod annotations;
pub mod artifacts;
pub mod claims;
pub mod executions;
pub mod notebooks;
pub mod notes;
pub mod reviews;
pub mod sources;
pub mod synthesis;

#[allow(unused_imports)]
pub use annotations::*;
#[allow(unused_imports)]
pub use artifacts::*;
#[allow(unused_imports)]
pub use claims::*;
#[allow(unused_imports)]
pub use executions::*;
#[allow(unused_imports)]
pub use notebooks::*;
#[allow(unused_imports)]
pub use notes::*;
#[allow(unused_imports)]
pub use reviews::*;
#[allow(unused_imports)]
pub use sources::*;
#[allow(unused_imports)]
pub use synthesis::*;

use crate::connection::DbConnection;

#[derive(Clone)]
pub struct ResearchRepository {
    pub(crate) db: DbConnection,
}

impl ResearchRepository {
    pub fn new(db: DbConnection) -> Self {
        Self { db }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::DbConnection;
    use custos_domain::{
        AnnotationRecord, AnnotationStatus, ArtifactLineageNode, CellExecutionStatus,
        ClaimEvidenceLink, ClaimGroundingLevel, EnvSnapshot, EnvironmentSpec,
        EvidenceRelation, ExecutionArtifact, ExecutionRecord, ExecutionRecordStatus,
        NoteRecord, NotebookCell, NotebookCellType, PassageAnchor, Recipe, RecipeInput, ResearchClaim,
        ResearchExperimentRun, ResearchSynthesisProposal, ReviewFinding, ReviewMethod,
        ReviewStatus, ReviewTargetType, ReviewerRecord, SourceRecord, SynthesisProposalStatus,
    };
    use rusqlite::params;

    fn protected_execution(recipe_id: &str) -> ExecutionRecord {
        use custos_domain::{
            CheckpointCommitStatus, CheckpointFileEntry, ComputeBackendKind,
            TrainingCheckpointManifest,
        };
        let mut checkpoint = TrainingCheckpointManifest::new(
            "checkpoint400",
            400,
            1.0,
            "s3://test/attempt-a/c400",
            "a".repeat(64),
        )
        .with_full_state("model@immutable", "unsloth@pinned", "sample400")
        .with_commit_status(CheckpointCommitStatus::CommittedProtected);
        for path in [
            "adapter_model.safetensors",
            "optimizer.pt",
            "scheduler.pt",
            "rng_state.pth",
            "trainer_state.json",
        ] {
            checkpoint.files.push(CheckpointFileEntry {
                path: path.into(),
                size_bytes: 1,
                digest: "a".repeat(64),
                required_for_state_resume: true,
            });
        }
        let mut record = ExecutionRecord::new(recipe_id)
            .with_backend(ComputeBackendKind::Colab)
            .with_output_namespace("attempt-a")
            .with_milestones(450, Some(440), None);
        record.commit_protected_checkpoint(400, checkpoint).unwrap();
        record
    }

    #[test]
    fn checkpoint_execution_survives_restart_and_preserves_continuation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("research.db");
        let path = path.to_str().unwrap();
        let (recipe_id, parent_id, child_id);
        {
            let repo = ResearchRepository::new(DbConnection::open(path).unwrap());
            let recipe = Recipe::new("train", "train.py", EnvironmentSpec::new());
            recipe_id = recipe.id.clone();
            repo.save_recipe(&recipe).unwrap();
            let parent = protected_execution(&recipe_id);
            parent_id = parent.id.clone();
            repo.save_execution_record(&parent).unwrap();
            let child = ExecutionRecord::new(&recipe_id)
                .with_backend(custos_domain::ComputeBackendKind::Kaggle)
                .with_resume_from(&parent_id)
                .with_resumed_from_checkpoint("checkpoint400")
                .with_output_namespace("attempt-b");
            child_id = child.id.clone();
            repo.save_execution_record(&child).unwrap();
        }
        let db = DbConnection::open(path).unwrap();
        let repo = ResearchRepository::new(db.clone());
        let mut parent = repo.get_execution_record(&parent_id).unwrap().unwrap();
        assert_eq!(parent.latest_observed_step, Some(450));
        assert_eq!(parent.latest_local_checkpoint_step, Some(440));
        assert_eq!(parent.latest_protected_checkpoint_step, Some(400));
        assert_eq!(
            parent.checkpoint.as_ref().unwrap().checkpoint_id,
            "checkpoint400"
        );
        let child = repo.get_execution_record(&child_id).unwrap().unwrap();
        assert_eq!(
            child.resume_from_attempt_id.as_deref(),
            Some(parent_id.as_str())
        );
        assert_eq!(
            child.resumed_from_checkpoint_id.as_deref(),
            Some("checkpoint400")
        );
        assert_eq!(
            child.backend,
            Some(custos_domain::ComputeBackendKind::Kaggle)
        );
        assert_eq!(
            repo.list_execution_records_for_recipe(&recipe_id)
                .unwrap()
                .len(),
            2
        );
        let count: u64 = db
            .lock()
            .unwrap()
            .query_row(
                "SELECT count(*) FROM outbox_messages WHERE topic = 'research.execution.saved'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
        let mut stale = parent.clone();
        stale.latest_observed_step = Some(449);
        assert!(repo.save_execution_record(&stale).is_err());
        let checkpoint = parent.checkpoint.clone().unwrap();
        assert!(parent.commit_protected_checkpoint(401, checkpoint).is_err());
        parent.latest_protected_checkpoint_step = None;
        parent.last_protected_at = None;
        assert!(repo.save_execution_record(&parent).is_err());
    }

    #[test]
    fn checkpoint_snapshot_and_event_rollback_together() {
        let db = DbConnection::open_in_memory().unwrap();
        let repo = ResearchRepository::new(db.clone());
        let recipe = Recipe::new("train", "train.py", EnvironmentSpec::new());
        repo.save_recipe(&recipe).unwrap();
        let record = protected_execution(&recipe.id);
        db.lock()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER fail_execution_event BEFORE INSERT ON outbox_messages
             WHEN NEW.topic = 'research.execution.saved' BEGIN SELECT RAISE(ABORT, 'injected failure'); END;",
            )
            .unwrap();
        assert!(repo.save_execution_record(&record).is_err());
        assert!(repo.get_execution_record(&record.id).unwrap().is_none());
    }

    #[test]
    fn checkpoint_legacy_rows_decode_but_corrupt_snapshots_do_not_default() {
        let db = DbConnection::open_in_memory().unwrap();
        let repo = ResearchRepository::new(db.clone());
        let recipe = Recipe::new("legacy", "train.py", EnvironmentSpec::new());
        repo.save_recipe(&recipe).unwrap();
        let record = ExecutionRecord::new(&recipe.id);
        repo.save_execution_record(&record).unwrap();
        db.lock()
            .unwrap()
            .execute(
                "UPDATE research_execution_records SET record_json = NULL WHERE id = ?1",
                params![record.id],
            )
            .unwrap();
        assert!(repo
            .get_execution_record(&record.id)
            .unwrap()
            .unwrap()
            .checkpoint
            .is_none());
        db.lock()
            .unwrap()
            .execute(
                "UPDATE research_execution_records SET record_json = '{broken' WHERE id = ?1",
                params![record.id],
            )
            .unwrap();
        assert!(repo.get_execution_record(&record.id).is_err());
        assert!(repo.list_execution_records_for_recipe(&recipe.id).is_err());
    }

    #[test]
    fn checkpoint_lineage_cannot_target_missing_parent_or_shared_namespace() {
        let db = DbConnection::open_in_memory().unwrap();
        let repo = ResearchRepository::new(db);
        let recipe = Recipe::new("train", "train.py", EnvironmentSpec::new());
        repo.save_recipe(&recipe).unwrap();
        let parent = protected_execution(&recipe.id);
        let mut child = ExecutionRecord::new(&recipe.id)
            .with_resume_from(&parent.id)
            .with_resumed_from_checkpoint("checkpoint400")
            .with_output_namespace("attempt-a");
        assert!(repo.save_execution_record(&child).is_err());
        repo.save_execution_record(&parent).unwrap();
        assert!(repo.save_execution_record(&child).is_err());
        child.output_namespace = Some("attempt-b".into());
        repo.save_execution_record(&child).unwrap();
        child.session_id = Some("different-session".into());
        assert!(repo.save_execution_record(&child).is_err());
    }

    #[test]
    fn notebook_replace_is_atomic_scoped_and_deletes_removed_cells() {
        let db = DbConnection::open_in_memory().unwrap();
        let repo = ResearchRepository::new(db);
        let cell = |id: &str, session: &str| NotebookCell {
            id: id.into(),
            session_id: session.into(),
            cell_type: NotebookCellType::Code,
            source: "print(42)".into(),
            cell_index: 0,
            execution_count: None,
            status: CellExecutionStatus::Idle,
            stdout: None,
            stderr: None,
            output_image: None,
            wall_ms: None,
            epoch: 0,
            updated_at: 0,
            ..Default::default()
        };
        repo.replace_notebook_cells("a", &[cell("one", "a"), cell("two", "a")])
            .unwrap();
        repo.replace_notebook_cells("b", &[cell("other", "b")])
            .unwrap();
        repo.replace_notebook_cells("a", &[cell("two", "a")])
            .unwrap();
        assert_eq!(repo.list_notebook_cells("a").unwrap().len(), 1);
        assert_eq!(repo.list_notebook_cells("a").unwrap()[0].id, "two");
        assert!(repo
            .replace_notebook_cells("a", &[cell("other", "a")])
            .is_err());
        assert_eq!(
            repo.list_notebook_cells("a").unwrap()[0].id,
            "two",
            "failed replacement rolls back deletion"
        );
        assert_eq!(repo.list_notebook_cells("b").unwrap()[0].id, "other");
        assert!(repo
            .replace_notebook_cells("a", &[cell("duplicate", "a"), cell("duplicate", "a")])
            .is_err());
        repo.replace_notebook_cells("a", &[]).unwrap();
        assert!(repo.list_notebook_cells("a").unwrap().is_empty());
    }

    #[test]
    fn test_research_repository_full_lifecycle() {
        let db = DbConnection::open_in_memory().expect("open db");
        let repo = ResearchRepository::new(db);

        // 1. Source
        let src = SourceRecord {
            id: "src_123".to_string(),
            source_type: "paper".to_string(),
            title: "Attention Is All You Need".to_string(),
            doi: Some("10.1145/test".to_string()),
            authors: vec!["Vaswani et al.".to_string()],
            year: Some(2017),
            content_hash: "hash_vaswani".to_string(),
            local_path: Some("/papers/transformer.pdf".to_string()),
            verified: true,
            abstract_text: Some("We propose Transformer...".to_string()),
            created_at: 1700000000,
        };
        repo.save_source(&src).expect("save source");
        let sources = repo.list_sources().expect("list sources");
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].title, "Attention Is All You Need");
        assert!(sources[0].verified);

        // 2. Anchor
        let anchor = PassageAnchor {
            id: "anc_01".to_string(),
            source_id: "src_123".to_string(),
            source_title: Some("Attention Is All You Need".to_string()),
            section_title: Some("3.2.2 Multi-Head Attention".to_string()),
            page_number: Some(3),
            start_offset: 120,
            end_offset: 250,
            exact_text: "Multi-Head Attention consists of several attention layers".to_string(),
            passage_hash: "hash_anchor_01".to_string(),
        };
        repo.save_anchor(&anchor).expect("save anchor");
        let anchors = repo
            .list_anchors_for_source("src_123")
            .expect("list anchors");
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].exact_text, anchor.exact_text);

        // 3. Claim
        let claim = ResearchClaim {
            id: "claim_01".to_string(),
            statement: "Multi-Head Attention runs 8 parallel heads in base model".to_string(),
            level: ClaimGroundingLevel::L1Cited,
            confidence_score: 0.95,
            invariants: vec!["head_count == 8".to_string()],
            evidence_links: vec![ClaimEvidenceLink {
                passage_anchor_id: "anc_01".to_string(),
                source_title: Some("Attention Is All You Need".to_string()),
                exact_text: Some(
                    "Multi-Head Attention consists of several attention layers".to_string(),
                ),
                relation: EvidenceRelation::Supports,
                rationale: "Confirmed in section 3.2.2".to_string(),
                verified_by: "expert_review".to_string(),
            }],
            created_at: 1700000200,
            sealed_proof_uri: None,
        };
        repo.save_claim(&claim).expect("save claim");
        let claims = repo.list_claims().expect("list claims");
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].statement, claim.statement);
        assert_eq!(claims[0].level, ClaimGroundingLevel::L1Cited);
        assert_eq!(claims[0].evidence_links.len(), 1);

        // 4. Run
        let run = ResearchExperimentRun {
            run_id: "run_exp_99".to_string(),
            session_id: "sess_01".to_string(),
            command: "python train_transformer.py".to_string(),
            cwd: "/workspace".to_string(),
            status: "ok".to_string(),
            wall_ms: 45000,
            surface: Some("sade_container".to_string()),
            reproducibility: "deterministic".to_string(),
            input_merkle_root: "merkle_in_root".to_string(),
            output_merkle_root: "merkle_out_root".to_string(),
            env_snapshot: EnvSnapshot {
                python_version: "3.11.4".to_string(),
                lockfile_hash: "lock_abc".to_string(),
                package_count: Some(42),
                hardware: "Apple M3 Max".to_string(),
                platform: Some("macos-arm64".to_string()),
            },
            sade_permit_id: Some("permit_01".to_string()),
            cas_log_uri: Some("cas://bafy_log".to_string()),
            ts: 1700000300,
        };
        repo.save_run(&run).expect("save run");
        let runs = repo.list_runs().expect("list runs");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].run_id, "run_exp_99");
        assert_eq!(runs[0].reproducibility, "deterministic");

        // 5. Lineage
        let lineage = ArtifactLineageNode {
            artifact_path: "eval_table.csv".to_string(),
            version: 1,
            content_hash: "hash_csv".to_string(),
            produced_by_run_id: Some("run_exp_99".to_string()),
            parent_version_hash: None,
            timestamp: 1700000400,
        };
        repo.record_artifact_lineage(&lineage)
            .expect("record lineage");
        let lineage_list = repo
            .list_artifact_lineage("eval_table.csv")
            .expect("list lineage");
        assert_eq!(lineage_list.len(), 1);
        assert_eq!(lineage_list[0].version, 1);
        assert_eq!(lineage_list[0].content_hash, "hash_csv");

        // 6. Recipe
        let mut env_spec = EnvironmentSpec::new();
        env_spec.python_version = Some("3.11.4".to_string());
        env_spec.requirements = vec!["torch>=2.0.0".to_string(), "pandas".to_string()];
        let recipe = Recipe::new(
            "Train Ablation Model",
            "python train.py --lr 1e-4",
            env_spec,
        )
        .with_description("Ablation study on learning rate")
        .with_input(RecipeInput::new("train_data", "cas://bafy_data", true))
        .with_output("loss_curve.png");
        let recipe_id = recipe.id.clone();
        repo.save_recipe(&recipe).expect("save recipe");

        let fetched_recipe = repo
            .get_recipe(&recipe_id)
            .expect("get recipe")
            .expect("recipe exists");
        assert_eq!(fetched_recipe.name, "Train Ablation Model");
        assert_eq!(fetched_recipe.inputs.len(), 1);
        assert_eq!(fetched_recipe.outputs, vec!["loss_curve.png"]);

        let recipes = repo.list_recipes().expect("list recipes");
        assert_eq!(recipes.len(), 1);

        // 7. Execution Record
        let mut exec_record = ExecutionRecord::new(&recipe_id).with_session_id("session_res_01");
        exec_record.stdout_cas_uri = Some("cas://bafy_exec_stdout".to_string());
        exec_record.artifacts.push(ExecutionArtifact::new(
            "loss_curve.png",
            "hash_loss_curve",
            10240,
        ));
        let exec_id = exec_record.id.clone();
        repo.save_execution_record(&exec_record)
            .expect("save exec record");

        let fetched_exec = repo
            .get_execution_record(&exec_id)
            .expect("get exec")
            .expect("exec exists");
        assert_eq!(fetched_exec.status, ExecutionRecordStatus::Pending);
        assert_eq!(fetched_exec.artifacts.len(), 1);

        exec_record.transition(ExecutionRecordStatus::Completed);
        exec_record.exit_code = Some(0);
        repo.save_execution_record(&exec_record)
            .expect("update exec record");

        let updated_exec = repo
            .get_execution_record(&exec_id)
            .expect("get exec updated")
            .expect("exec exists");
        assert_eq!(updated_exec.status, ExecutionRecordStatus::Completed);
        assert_eq!(updated_exec.exit_code, Some(0));

        let execs_for_recipe = repo
            .list_execution_records_for_recipe(&recipe_id)
            .expect("list execs for recipe");
        assert_eq!(execs_for_recipe.len(), 1);

        // 8. Annotations
        let ann_img = AnnotationRecord::new_image_pin(
            "figures/loss_curve.png",
            1,
            0.35,
            0.75,
            1,
            "Add error bars at epoch 50",
            "alice",
        )
        .with_side_chat("session_side_01");
        let ann_img_id = ann_img.id.clone();
        repo.save_annotation(&ann_img).expect("save annotation");

        let pending = repo.list_pending_annotations().expect("list pending");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].actor, "alice");

        let artifact_anns = repo
            .list_annotations_for_artifact("figures/loss_curve.png", Some(1))
            .expect("list anns");
        assert_eq!(artifact_anns.len(), 1);
        assert_eq!(
            artifact_anns[0].side_chat_session_id.as_deref(),
            Some("session_side_01")
        );

        repo.update_annotation_status(&ann_img_id, AnnotationStatus::Submitted, Some("turn_101"))
            .expect("update status");
        let updated_ann = repo
            .list_annotations_for_artifact("figures/loss_curve.png", Some(1))
            .expect("list anns updated");
        assert_eq!(updated_ann[0].status, AnnotationStatus::Submitted);
        assert_eq!(
            updated_ann[0].submitted_turn_id.as_deref(),
            Some("turn_101")
        );

        let pending_after = repo.list_pending_annotations().expect("list pending after");
        assert_eq!(pending_after.len(), 0);

        // 9. Artifact List & Lineage DAG
        let all_artifacts = repo.list_all_artifacts().expect("list all artifacts");
        assert_eq!(all_artifacts.len(), 1);
        assert_eq!(all_artifacts[0].artifact_path, "eval_table.csv");
        assert_eq!(all_artifacts[0].latest_version, 1);
        assert_eq!(all_artifacts[0].latest_content_hash, "hash_csv");

        let graph = repo
            .get_artifact_lineage_graph(None)
            .expect("get lineage graph");
        assert!(!graph.nodes.is_empty());
        let csv_node = graph
            .nodes
            .iter()
            .find(|n| n.id == "artifact:eval_table.csv:v1");
        assert!(csv_node.is_some());

        // 10. Notes & Version History
        let mut note = NoteRecord::new(
            "Convergence Architecture Notes",
            "# Custos SADE Convergence\nStep 6 artifact identity verified.",
            Some("sess_res_01".into()),
            None,
            vec!["sade".into(), "architecture".into()],
        )
        .expect("create note");
        let note_id = note.id.clone();
        repo.save_note(&note).expect("save note");

        let fetched_note = repo
            .get_note(&note_id)
            .expect("get note")
            .expect("note exists");
        assert_eq!(fetched_note.title, "Convergence Architecture Notes");
        assert_eq!(fetched_note.version, 1);
        assert_eq!(fetched_note.tags, vec!["sade", "architecture"]);

        let prev_ver = note
            .update(
                Some("Convergence Architecture Notes (Updated)".into()),
                "# Custos SADE Convergence\nStep 6 updated with lineage DAG graph.".into(),
                None,
            )
            .expect("update note");
        repo.save_note(&note).expect("save updated note");
        repo.save_note_version(&prev_ver)
            .expect("save note version");

        let note_versions = repo.list_note_versions(&note_id).expect("list versions");
        assert_eq!(note_versions.len(), 1);
        assert_eq!(note_versions[0].version, 1);

        let notes_for_sess = repo.list_notes(Some("sess_res_01")).expect("list notes");
        assert_eq!(notes_for_sess.len(), 1);
        assert_eq!(notes_for_sess[0].version, 2);

        // 8. Reviewer Records (Step 8)
        let review = ReviewerRecord::new(
            ReviewTargetType::Diff,
            "patch_hash_987",
            "verifier:security-linter",
            ReviewMethod::AutomatedVerifier,
            ReviewStatus::Approved,
            "No path traversal or command injection vulnerabilities found.",
            Some("sha256:fedcba".into()),
            vec![ReviewFinding {
                severity: custos_domain::FindingSeverity::Info,
                criterion: "no_command_injection".to_string(),
                message: "Passed fail-closed validation".into(),
                file_path: Some("crates/custos-runtime/src/lib.rs".into()),
                line_number: Some(42),
            }],
        )
        .expect("create review");

        repo.save_review(&review).expect("save review");
        let fetched_rev = repo
            .get_review(&review.id)
            .expect("get review")
            .expect("review exists");
        assert_eq!(fetched_rev.target_id, "patch_hash_987");
        assert_eq!(fetched_rev.status, ReviewStatus::Approved);
        assert!(fetched_rev.is_fresh);
        assert_eq!(fetched_rev.findings.len(), 1);

        let filtered_revs = repo
            .list_reviews(Some(ReviewTargetType::Diff), Some("patch_hash_987"))
            .expect("list reviews");
        assert_eq!(filtered_revs.len(), 1);

        repo.mark_review_stale(&review.id).expect("mark stale");
        let stale_rev = repo
            .get_review(&review.id)
            .expect("get review")
            .expect("review exists");
        assert!(!stale_rev.is_fresh);

        // 10. Research Synthesis Proposal
        let proposal = ResearchSynthesisProposal::new(
            "Empirical Attention Bounds",
            "Synthesis of multi-head attention claims and verification tasks",
            vec![custos_domain::ClaimHandoffSummary {
                claim_id: "claim_01".into(),
                statement: "Multi-Head Attention runs 8 parallel heads in base model".into(),
                level: ClaimGroundingLevel::L1Cited,
                confidence_score: 0.95,
                has_fresh_review: true,
                sealed_proof_uri: None,
            }],
            vec![custos_domain::RecipeHandoffSummary {
                recipe_id: "recipe_01".into(),
                name: "Train transformer".into(),
                command: "python train.py".into(),
                inputs_count: 1,
                outputs: vec!["weights.pt".into()],
            }],
            vec!["eval_table.csv".into()],
            Some("ws_default".into()),
            Some("feature/attention-bounds".into()),
        )
        .expect("create synthesis proposal");

        repo.save_synthesis_proposal(&proposal)
            .expect("save proposal");
        let fetched_prop = repo
            .get_synthesis_proposal(&proposal.id)
            .expect("get proposal")
            .expect("proposal exists");
        assert_eq!(fetched_prop.title, "Empirical Attention Bounds");
        assert_eq!(fetched_prop.claims.len(), 1);
        assert_eq!(fetched_prop.recipes.len(), 1);
        assert_eq!(fetched_prop.status, SynthesisProposalStatus::Draft);

        let proposals = repo.list_synthesis_proposals().expect("list proposals");
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].id, proposal.id);
    }
}
