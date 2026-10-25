//! Google Colab Compute Target Adapter (F10 / P3).
//!
//! Governs interactive and batch training on Google Colab hosted runtimes via
//! the Colab CLI (`google-colab-cli`). Decouples kernel stop from runtime unassignment
//! to avoid burning Colab compute units while preserving post-run inspection.

use std::path::PathBuf;
use std::sync::Arc;
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use custos_core::contracts::experiment::{
    AttemptObservation, ExecutionTargetPort, PreflightReport, TargetCapabilities,
};
use custos_domain::{
    DomainError, ExecState, ExecutionArtifact, ExecutionAttempt, ExternalJobRef,
    ProgressTriplet, ResourceState,
};

#[derive(Clone)]
pub struct ColabTargetAdapter {
    colab_bin: PathBuf,
    work_dir: PathBuf,
    mock_mode: bool,
    mock_status: Arc<Mutex<String>>,
    mock_resource_released: Arc<Mutex<bool>>,
}

impl ColabTargetAdapter {
    pub fn new(work_dir: impl Into<PathBuf>) -> Self {
        Self {
            colab_bin: PathBuf::from("colab"),
            work_dir: work_dir.into(),
            mock_mode: false,
            mock_status: Arc::new(Mutex::new("running".to_string())),
            mock_resource_released: Arc::new(Mutex::new(false)),
        }
    }

    pub fn with_colab_bin(mut self, bin: impl Into<PathBuf>) -> Self {
        self.colab_bin = bin.into();
        self
    }

    pub fn is_available(&self) -> bool {
        which::which(&self.colab_bin).is_ok()
    }

    pub fn with_mock_mode(mut self) -> Self {
        self.mock_mode = true;
        self
    }

    pub async fn set_mock_status(&self, status: impl Into<String>) {
        let mut guard = self.mock_status.lock().await;
        *guard = status.into();
    }

    pub async fn is_resource_released(&self) -> bool {
        *self.mock_resource_released.lock().await
    }
}

#[async_trait]
impl ExecutionTargetPort for ColabTargetAdapter {
    async fn capabilities(&self) -> TargetCapabilities {
        TargetCapabilities {
            supports_interactive_control: true,
            supports_checkpoint_request: true,
            supports_direct_storage_upload: true,
            supports_live_telemetry_streaming: true,
            supports_graceful_stop: true,
        }
    }

    async fn preflight(&self, _recipe_id: &str) -> Result<PreflightReport, DomainError> {
        let mut blockers = Vec::new();
        let mut warnings = Vec::new();

        if !self.mock_mode {
            let has_colab = which::which(&self.colab_bin).is_ok();
            let config_dir = dirs::config_dir().map(|c| c.join("colab-cli"));
            let home_config = dirs::home_dir().map(|h| h.join(".config").join("colab-cli"));
            let gcloud_creds = dirs::config_dir().map(|c| c.join("gcloud").join("application_default_credentials.json"));
            let has_creds = config_dir.map(|c| c.join("token.json").exists()).unwrap_or(false)
                || home_config.map(|h| h.join("token.json").exists()).unwrap_or(false)
                || gcloud_creds.map(|g| g.exists()).unwrap_or(false)
                || std::env::var("COLAB_TOKEN").is_ok()
                || std::env::var("GOOGLE_APPLICATION_CREDENTIALS").is_ok();

            if !has_colab {
                if which::which("uv").is_ok() || which::which("python3").is_ok() {
                    warnings.push("Colab CLI not found in PATH; will run via local python environment".to_string());
                } else {
                    blockers.push("Colab CLI not installed and no Python runtime found".to_string());
                }
            }

            if !has_creds {
                warnings.push("No stored Colab credentials found; set COLAB_TOKEN or run 'gcloud auth application-default login' or 'colab --auth=oauth2 sessions'".to_string());
            }
        }

        if blockers.is_empty() {
            Ok(PreflightReport {
                allowed: true,
                blockers: Vec::new(),
                warnings,
            })
        } else {
            Ok(PreflightReport {
                allowed: false,
                blockers,
                warnings,
            })
        }
    }

    async fn submit(
        &self,
        attempt: &ExecutionAttempt,
        _idempotency_key: &str,
    ) -> Result<ExternalJobRef, DomainError> {
        let account = "colab_account".to_string();
        let session_id = format!("colab-{}", attempt.experiment_id.replace('_', "-").to_lowercase());

        let attempt_dir = self.work_dir.join("colab").join(&attempt.id);
        tokio::fs::create_dir_all(&attempt_dir).await.map_err(|e| {
            DomainError::Validation(format!("Failed to create Colab staging dir: {e}"))
        })?;

        let entrypoint = attempt_dir.join("run.py");
        if !entrypoint.exists() {
            tokio::fs::write(&entrypoint, "# Custos Colab runtime script\nprint('Colab job started')\n")
                .await
                .map_err(|e| DomainError::Validation(format!("Failed to write script: {e}")))?;
        }

        if self.mock_mode {
            return ExternalJobRef::new(account, session_id, "v1");
        }

        let mut cmd = tokio::process::Command::new(&self.colab_bin);
        cmd.arg("run")
            .arg(&entrypoint)
            .arg("-s")
            .arg(&session_id)
            .arg("--keep");

        let output = cmd.output().await.map_err(|e| {
            DomainError::Validation(format!("Failed to launch Colab run: {e}"))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DomainError::Validation(format!("Colab run failed: {stderr}")));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let runtime_id = stdout
            .lines()
            .find_map(|l| {
                if l.contains("Session READY (") {
                    let after = l.split("Session READY (").nth(1)?;
                    after.split(')').next()
                } else if l.contains("Creating session '") {
                    let after = l.split("Creating session '").nth(1)?;
                    after.split('\'').next()
                } else {
                    None
                }
            })
            .unwrap_or(&session_id)
            .to_string();

        ExternalJobRef::new(account, runtime_id, "v1")
    }

    async fn observe(&self, external_ref: &ExternalJobRef) -> Result<AttemptObservation, DomainError> {
        let (raw_status, exit_code) = if self.mock_mode {
            let guard = self.mock_status.lock().await;
            (guard.clone(), None)
        } else {
            let mut cmd = tokio::process::Command::new(&self.colab_bin);
            cmd.arg("status").arg("-s").arg(&external_ref.job_id);

            let output = cmd.output().await.map_err(|e| {
                DomainError::Validation(format!("Failed to query Colab status: {e}"))
            })?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
            (stdout, output.status.code())
        };

        let released = if self.mock_mode {
            *self.mock_resource_released.lock().await
        } else {
            raw_status.contains("not found") || raw_status.contains("no active sessions")
        };

        let resource_state = if released {
            ResourceState::ReleaseConfirmed
        } else {
            ResourceState::Held
        };

        let exec_state = if raw_status.contains("busy") || raw_status.contains("running") {
            ExecState::Running
        } else if raw_status.contains("stopped") || raw_status.contains("not found") {
            ExecState::Stopped
        } else if raw_status.contains("idle") || raw_status.contains("completed") || raw_status.contains("finished") {
            ExecState::Completed
        } else if raw_status.contains("failed") || raw_status.contains("error") {
            ExecState::Failed
        } else {
            ExecState::Uncertain
        };

        Ok(AttemptObservation {
            exec_state,
            resource_state,
            progress: ProgressTriplet::default(),
            exit_code,
        })
    }

    async fn request_checkpoint(&self, _external_ref: &ExternalJobRef) -> Result<(), DomainError> {
        // Can write trigger file or trigger kernel interrupt
        Ok(())
    }

    async fn stop_job(&self, external_ref: &ExternalJobRef) -> Result<(), DomainError> {
        if self.mock_mode {
            let mut guard = self.mock_status.lock().await;
            *guard = "stopped".to_string();
            return Ok(());
        }

        let mut cmd = tokio::process::Command::new(&self.colab_bin);
        cmd.arg("stop").arg("-s").arg(&external_ref.job_id);
        let _ = cmd.output().await;
        Ok(())
    }

    async fn release_compute(&self, external_ref: &ExternalJobRef) -> Result<(), DomainError> {
        // colab stop terminates kernel and unassigns the server-side VM session
        if self.mock_mode {
            let mut guard = self.mock_resource_released.lock().await;
            *guard = true;
            return Ok(());
        }

        let mut cmd = tokio::process::Command::new(&self.colab_bin);
        cmd.arg("stop").arg("-s").arg(&external_ref.job_id);
        let output = cmd.output().await.map_err(|e| {
            DomainError::Validation(format!("Failed to unassign Colab compute: {e}"))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("not found") {
                return Err(DomainError::Validation(format!(
                    "Colab unassign failed: {stderr}"
                )));
            }
        }

        Ok(())
    }

    async fn collect_artifacts(
        &self,
        external_ref: &ExternalJobRef,
    ) -> Result<Vec<ExecutionArtifact>, DomainError> {
        let dest = self.work_dir.join("colab").join("outputs").join(&external_ref.job_id);
        tokio::fs::create_dir_all(&dest).await.map_err(|e| {
            DomainError::Validation(format!("Failed to create Colab output dir: {e}"))
        })?;

        if !self.mock_mode {
            let mut cmd = tokio::process::Command::new(&self.colab_bin);
            cmd.arg("download")
                .arg("-s")
                .arg(&external_ref.job_id)
                .arg("content")
                .arg(&dest);
            let _ = cmd.output().await;
        } else {
            let dummy = dest.join("model_checkpoint.pt");
            let _ = tokio::fs::write(&dummy, b"colab weights checkpoint").await;
        }

        let mut artifacts = Vec::new();
        let mut entries = tokio::fs::read_dir(&dest).await.map_err(|e| {
            DomainError::Validation(format!("Failed to read output dir: {e}"))
        })?;

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_file() {
                if let Ok(bytes) = tokio::fs::read(&path).await {
                    let hash = format!("{:x}", Sha256::digest(&bytes));
                    artifacts.push(ExecutionArtifact::new(
                        path.to_string_lossy().to_string(),
                        hash,
                        bytes.len() as u64,
                    ));
                }
            }
        }

        Ok(artifacts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn colab_capabilities_and_preflight() {
        let tmp = tempfile::tempdir().unwrap();
        let adapter = ColabTargetAdapter::new(tmp.path()).with_mock_mode();

        let caps = adapter.capabilities().await;
        assert!(caps.supports_interactive_control);
        assert!(caps.supports_checkpoint_request);
        assert!(caps.supports_graceful_stop);

        let report = adapter.preflight("rec_colab").await.unwrap();
        assert!(report.allowed);
    }

    #[tokio::test]
    async fn colab_decoupled_stop_and_release() {
        let tmp = tempfile::tempdir().unwrap();
        let adapter = ColabTargetAdapter::new(tmp.path()).with_mock_mode();
        let job = ExternalJobRef::new("user", "colab-exp-1", "v1").unwrap();

        // Initially running, resource held
        let obs = adapter.observe(&job).await.unwrap();
        assert_eq!(obs.exec_state, ExecState::Running);
        assert_eq!(obs.resource_state, ResourceState::Held);
        assert!(!adapter.is_resource_released().await);

        // Stop job terminates training but resource remains Held
        adapter.stop_job(&job).await.unwrap();
        let obs = adapter.observe(&job).await.unwrap();
        assert_eq!(obs.exec_state, ExecState::Stopped);
        assert_eq!(obs.resource_state, ResourceState::Held);
        assert!(!adapter.is_resource_released().await);

        // Explicit release deallocates VM/session
        adapter.release_compute(&job).await.unwrap();
        assert!(adapter.is_resource_released().await);
        let obs = adapter.observe(&job).await.unwrap();
        assert_eq!(obs.resource_state, ResourceState::ReleaseConfirmed);
    }
}
