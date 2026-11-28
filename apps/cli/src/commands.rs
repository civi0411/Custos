//! Agent-drivable domain subcommands for the Custos CLI.
//!
//! Typed wrappers around `LocalApiClient` JSON-RPC endpoints so external CLI
//! agents can drive worktrees, browser, memory, MCP, outbox, and automation.

use clap::Subcommand;
use custos_sdk::local_api::{
    ApiRequest, LocalApiClient, METHOD_ASSISTANT_OUTBOX_APPROVE, METHOD_ASSISTANT_OUTBOX_DISPATCH,
    METHOD_ASSISTANT_OUTBOX_LIST, METHOD_AUTOMATION_JOBS_CREATE, METHOD_AUTOMATION_JOBS_LIST,
    METHOD_AUTOMATION_JOBS_RUN, METHOD_BROWSER_SESSIONS_LIST, METHOD_BROWSER_TABS_CLICK,
    METHOD_BROWSER_TABS_FILL, METHOD_BROWSER_TABS_LIST, METHOD_BROWSER_TABS_NAVIGATE,
    METHOD_BROWSER_TABS_SCREENSHOT, METHOD_BROWSER_TABS_SNAPSHOT, METHOD_MCP_SERVERS_LIST,
    METHOD_MCP_SERVERS_TOGGLE, METHOD_MCP_TOOLS_LIST, METHOD_MEMORY_PROPOSALS_LIST,
    METHOD_MEMORY_PROPOSALS_RESOLVE, METHOD_WORKSPACES_AGENT_STEER,
    METHOD_WORKSPACES_CANDIDATES_COMPARE, METHOD_WORKSPACES_CANDIDATES_SPAWN,
    METHOD_WORKSPACES_INTEGRATE,
};
use serde_json::{json, Value};

#[derive(Debug, Subcommand)]
pub enum WorktreeCommand {
    /// Spawn candidate worktrees for parallel agent execution
    Spawn {
        workspace_id: String,
        #[arg(long, default_value_t = 2)]
        count: usize,
        #[arg(long)]
        task: Option<String>,
    },
    /// Compare candidate worktrees in a workspace
    Compare { workspace_id: String },
    /// Integrate a winning candidate worktree
    Integrate {
        workspace_id: String,
        candidate_id: String,
    },
    /// Steer the workspace agent with inline guidance
    Steer {
        workspace_id: String,
        instruction: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum BrowserCommand {
    /// List active browser sessions
    Sessions,
    /// List browser tabs
    Tabs {
        #[arg(long)]
        session_id: Option<String>,
    },
    /// Navigate a browser tab to a URL
    Navigate { tab_id: String, url: String },
    /// Capture DOM/accessibility snapshot for a browser tab
    Snapshot { tab_id: String },
    /// Capture screenshot for a browser tab
    Screenshot { tab_id: String },
    /// Click an element selector in a browser tab
    Click { tab_id: String, selector: String },
    /// Fill an input selector in a browser tab
    Fill {
        tab_id: String,
        selector: String,
        value: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum MemoryCommand {
    /// List pending or resolved memory proposals
    Proposals {
        #[arg(long)]
        status: Option<String>,
    },
    /// Resolve a memory proposal (approve | reject)
    Resolve {
        proposal_id: String,
        decision: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum McpCommand {
    /// List configured MCP servers
    Servers,
    /// List discovered MCP tools
    Tools {
        #[arg(long)]
        server_id: Option<String>,
    },
    /// Enable or disable an MCP server
    Toggle {
        server_id: String,
        #[arg(long, default_value_t = true)]
        enabled: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum OutboxCommand {
    /// List assistant outbox actions
    List,
    /// Approve a pending outbox action
    Approve { id: String },
    /// Dispatch an approved outbox action
    Dispatch { id: String },
}

#[derive(Debug, Subcommand)]
pub enum AutomationCommand {
    /// List registered headless automation jobs
    List,
    /// Create a headless automation job
    Create {
        name: String,
        command: String,
        #[arg(long)]
        cron: Option<String>,
    },
    /// Trigger an automation job through the task kernel
    Run { job_id: String },
}

pub(crate) async fn send_rpc(
    client: &LocalApiClient,
    request_id: &str,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    let resp = client
        .send_request(ApiRequest::new(request_id, method, params))
        .await?;
    if let Some(err) = resp.error {
        return Err(err);
    }
    Ok(resp.result.unwrap_or(Value::Null))
}

pub async fn execute_worktree_command(
    client: &LocalApiClient,
    action: WorktreeCommand,
) -> Result<Value, String> {
    let (method, params) = match action {
        WorktreeCommand::Spawn {
            workspace_id,
            count,
            task,
        } => (
            METHOD_WORKSPACES_CANDIDATES_SPAWN,
            json!({ "workspace_id": workspace_id, "count": count, "task": task }),
        ),
        WorktreeCommand::Compare { workspace_id } => (
            METHOD_WORKSPACES_CANDIDATES_COMPARE,
            json!({ "workspace_id": workspace_id }),
        ),
        WorktreeCommand::Integrate {
            workspace_id,
            candidate_id,
        } => (
            METHOD_WORKSPACES_INTEGRATE,
            json!({ "workspace_id": workspace_id, "candidate_id": candidate_id }),
        ),
        WorktreeCommand::Steer {
            workspace_id,
            instruction,
        } => (
            METHOD_WORKSPACES_AGENT_STEER,
            json!({ "workspace_id": workspace_id, "instruction": instruction }),
        ),
    };
    send_rpc(client, "cli-worktrees", method, params).await
}

pub async fn execute_browser_command(
    client: &LocalApiClient,
    action: BrowserCommand,
) -> Result<Value, String> {
    let (method, params) = match action {
        BrowserCommand::Sessions => (METHOD_BROWSER_SESSIONS_LIST, json!({})),
        BrowserCommand::Tabs { session_id } => (
            METHOD_BROWSER_TABS_LIST,
            json!({ "session_id": session_id }),
        ),
        BrowserCommand::Navigate { tab_id, url } => (
            METHOD_BROWSER_TABS_NAVIGATE,
            json!({ "tab_id": tab_id, "url": url }),
        ),
        BrowserCommand::Snapshot { tab_id } => {
            (METHOD_BROWSER_TABS_SNAPSHOT, json!({ "tab_id": tab_id }))
        }
        BrowserCommand::Screenshot { tab_id } => {
            (METHOD_BROWSER_TABS_SCREENSHOT, json!({ "tab_id": tab_id }))
        }
        BrowserCommand::Click { tab_id, selector } => (
            METHOD_BROWSER_TABS_CLICK,
            json!({ "tab_id": tab_id, "selector": selector }),
        ),
        BrowserCommand::Fill {
            tab_id,
            selector,
            value,
        } => (
            METHOD_BROWSER_TABS_FILL,
            json!({ "tab_id": tab_id, "selector": selector, "value": value }),
        ),
    };
    send_rpc(client, "cli-browser", method, params).await
}

pub async fn execute_memory_command(
    client: &LocalApiClient,
    action: MemoryCommand,
) -> Result<Value, String> {
    let (method, params) = match action {
        MemoryCommand::Proposals { status } => {
            (METHOD_MEMORY_PROPOSALS_LIST, json!({ "status": status }))
        }
        MemoryCommand::Resolve {
            proposal_id,
            decision,
        } => (
            METHOD_MEMORY_PROPOSALS_RESOLVE,
            json!({ "proposal_id": proposal_id, "decision": decision }),
        ),
    };
    send_rpc(client, "cli-memory", method, params).await
}

pub async fn execute_mcp_command(
    client: &LocalApiClient,
    action: McpCommand,
) -> Result<Value, String> {
    let (method, params) = match action {
        McpCommand::Servers => (METHOD_MCP_SERVERS_LIST, json!({})),
        McpCommand::Tools { server_id } => {
            (METHOD_MCP_TOOLS_LIST, json!({ "server_id": server_id }))
        }
        McpCommand::Toggle { server_id, enabled } => (
            METHOD_MCP_SERVERS_TOGGLE,
            json!({ "server_id": server_id, "enabled": enabled }),
        ),
    };
    send_rpc(client, "cli-mcp", method, params).await
}

pub async fn execute_outbox_command(
    client: &LocalApiClient,
    action: OutboxCommand,
) -> Result<Value, String> {
    let (method, params) = match action {
        OutboxCommand::List => (METHOD_ASSISTANT_OUTBOX_LIST, json!({})),
        OutboxCommand::Approve { id } => (METHOD_ASSISTANT_OUTBOX_APPROVE, json!({ "id": id })),
        OutboxCommand::Dispatch { id } => (METHOD_ASSISTANT_OUTBOX_DISPATCH, json!({ "id": id })),
    };
    send_rpc(client, "cli-outbox", method, params).await
}

pub async fn execute_automation_command(
    client: &LocalApiClient,
    action: AutomationCommand,
) -> Result<Value, String> {
    let (method, params) = match action {
        AutomationCommand::List => (METHOD_AUTOMATION_JOBS_LIST, json!({})),
        AutomationCommand::Create {
            name,
            command,
            cron,
        } => (
            METHOD_AUTOMATION_JOBS_CREATE,
            json!({ "name": name, "command": command, "cron": cron }),
        ),
        AutomationCommand::Run { job_id } => {
            (METHOD_AUTOMATION_JOBS_RUN, json!({ "job_id": job_id }))
        }
    };
    send_rpc(client, "cli-automation", method, params).await
}

