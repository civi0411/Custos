use serde::{Deserialize, Serialize};

/// Workbench-specific environment binding profile
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "lens", rename_all = "snake_case")]
pub enum EnvironmentProfile {
    /// Coding Workbench: requires dedicated Git worktree and terminal PTY
    Coding {
        repo_path: String,
        branch: String,
        worktree_path: String,
    },
    /// Assistant Workbench: logical project scope without filesystem mutation requirements
    Assistant {
        project_id: Option<String>,
    },
    /// Research Workbench: bound to document corpus and reproducible compute kernel
    Research {
        corpus_path: String,
        kernel_spec: Option<String>,
        dataset_hash: Option<String>,
    },
    /// Quant Workbench: bound to historical market data partition and backtest isolation
    Quant {
        data_partition: String,
        is_live_gateway: bool,
    },
}

impl EnvironmentProfile {
    /// Invariant check: returns whether this profile requires physical Git worktree isolation.
    pub fn requires_worktree(&self) -> bool {
        matches!(self, Self::Coding { .. })
    }

    /// Invariant check: returns whether live network sockets are permitted.
    pub fn allows_live_network(&self) -> bool {
        match self {
            Self::Quant { is_live_gateway, .. } => *is_live_gateway,
            _ => true,
        }
    }
}

