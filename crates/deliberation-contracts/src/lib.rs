#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum WorkerRole {
    Architect,
    Coder,
    Critic,
    Tester,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeliberationPlan {
    pub steps: Vec<String>,
    pub assigned_role: WorkerRole,
}
