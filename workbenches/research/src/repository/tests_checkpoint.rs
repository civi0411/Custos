use super::*;
use custos_domain::{ExecutionRecord, Recipe, RecipeEnvironmentSpec};
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
        let recipe = Recipe::new("train", "train.py", RecipeEnvironmentSpec::new());
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
    let parent = repo.get_execution_record(&parent_id).unwrap().unwrap();
    assert_eq!(parent.latest_protected_checkpoint_step, Some(400));
    assert_eq!(
        parent.checkpoint.as_ref().unwrap().commit_status,
        custos_domain::CheckpointCommitStatus::CommittedProtected
    );
    {
        let conn = db.lock().unwrap();
        let parent_payload: String = conn
            .query_row(
                "SELECT record_json FROM research_execution_records WHERE id = ?1",
                params![parent_id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(parent_payload.contains("\"commit_status\":\"committed_protected\""));
    }
    let mut child = repo.get_execution_record(&child_id).unwrap().unwrap();
    assert_eq!(
        child.resumed_from_checkpoint_id.as_deref(),
        Some("checkpoint400")
    );
    let mut next_ckpt = custos_domain::TrainingCheckpointManifest::new(
        "checkpoint450",
        450,
        1.1,
        "s3://test/attempt-b/c450",
        "b".repeat(64),
    )
    .with_full_state("model@immutable", "unsloth@pinned", "sample450")
    .with_commit_status(custos_domain::CheckpointCommitStatus::CommittedProtected);
    next_ckpt.files.push(custos_domain::CheckpointFileEntry {
        path: "adapter_model.safetensors".into(),
        size_bytes: 1,
        digest: "b".repeat(64),
        required_for_state_resume: true,
    });
    child.commit_protected_checkpoint(450, next_ckpt).unwrap();
    repo.save_execution_record(&child).unwrap();
    let reloaded_child = repo.get_execution_record(&child_id).unwrap().unwrap();
    assert_eq!(reloaded_child.latest_protected_checkpoint_step, Some(450));
    assert!(reloaded_child.checkpoint.is_some());
}
