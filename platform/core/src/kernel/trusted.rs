use std::sync::Arc;

use async_trait::async_trait;
use custos_domain::{
    ActionIntent, DomainError, Permit, Task, TaskContract, TaskStatus, VerificationClaim,
};

use crate::authority::AuthorityEngine;
use crate::contracts::kernel::KernelPort;
use crate::evidence::{EvidenceBundle, EvidencePipeline};
use crate::kernel::{CompleteTask, CreateTask, TaskService, TaskStore};

/// Reference implementation composing the three trusted engines already in this crate.
pub struct TrustedKernel {
    tasks: TaskService,
    authority: AuthorityEngine,
    evidence: EvidencePipeline,
}

impl TrustedKernel {
    pub fn new(store: Arc<dyn TaskStore>) -> Self {
        Self::with_parts(
            TaskService::new(store),
            AuthorityEngine::default(),
            EvidencePipeline::with_standard_verifiers(),
        )
    }

    pub fn with_source_acquisition(
        store: Arc<dyn TaskStore>,
        source_acquisition: Arc<dyn crate::contracts::source_acquisition::SourceAcquisitionPort>,
    ) -> Self {
        Self::with_parts(
            TaskService::new(store),
            AuthorityEngine::default(),
            EvidencePipeline::with_source_acquisition(source_acquisition),
        )
    }

    pub fn with_parts(
        tasks: TaskService,
        authority: AuthorityEngine,
        evidence: EvidencePipeline,
    ) -> Self {
        Self {
            tasks,
            authority,
            evidence,
        }
    }

    pub fn authority(&self) -> &AuthorityEngine {
        &self.authority
    }

    pub fn evidence_mut(&mut self) -> &mut EvidencePipeline {
        &mut self.evidence
    }
}

#[async_trait]
impl KernelPort for TrustedKernel {
    async fn create_task(&self, title: String) -> Result<Task, DomainError> {
        self.tasks.create_task(title).await
    }

    async fn create_task_with_contract(
        &self,
        title: String,
        contract: TaskContract,
    ) -> Result<Task, DomainError> {
        let (task, _) = self
            .tasks
            .execute_create(CreateTask {
                title,
                metadata: None,
                contract: Some(contract),
            })
            .await?;
        Ok(task)
    }

    async fn get_task(&self, task_id: &str) -> Result<Option<Task>, DomainError> {
        self.tasks.get_task(task_id).await
    }

    async fn transition_task(&self, task_id: &str, next: TaskStatus) -> Result<Task, DomainError> {
        self.tasks.transition_task(task_id, next).await
    }

    async fn request_permit(
        &self,
        task_id: &str,
        intent: &ActionIntent,
        actor: &str,
    ) -> Result<Permit, DomainError> {
        self.authority
            .authorize_action(task_id, intent, actor)
            .await
    }

    async fn consume_permit(
        &self,
        permit_id: &str,
        intent: &ActionIntent,
    ) -> Result<Permit, DomainError> {
        self.authority
            .permits
            .consume_permit(permit_id, &intent.argument_digest())
    }

    async fn verify_evidence(
        &self,
        bundle: &EvidenceBundle,
    ) -> Result<VerificationClaim, DomainError> {
        self.evidence.verify_bundle(bundle).await
    }

    async fn grant_capability(
        &self,
        task_id: &str,
        grantee: &str,
        capability: &str,
    ) -> Result<custos_domain::Grant, DomainError> {
        let grant = custos_domain::Grant::new(
            task_id.to_string(),
            grantee.to_string(),
            capability.to_string(),
            serde_json::json!({}),
            None,
        );
        self.authority.grants.insert(grant.clone());
        Ok(grant)
    }

    async fn find_active_grant(
        &self,
        task_id: &str,
        capability: &str,
    ) -> Result<Option<custos_domain::Grant>, DomainError> {
        Ok(self.authority.grants.find_active_grant(task_id, capability))
    }

    async fn complete_task(
        &self,
        task_id: &str,
        summary: String,
        claims: Vec<VerificationClaim>,
    ) -> Result<Task, DomainError> {
        let task = self
            .tasks
            .get_task(task_id)
            .await?
            .ok_or_else(|| DomainError::NotFound {
                kind: "Task".into(),
                id: task_id.to_string(),
            })?;
        let (updated, _) = self
            .tasks
            .execute_complete(CompleteTask {
                task_id: task.id,
                expected_epoch: task.epoch,
                summary,
                evidence_claims: claims,
            })
            .await?;
        Ok(updated)
    }
}

