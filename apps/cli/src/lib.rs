//! Custos Headless Terminal CLI Client (`custos`)
//!
//! Pure-Rust CLI entrypoint communicating with `custos-daemon` via `custos_sdk::LocalApiClient`.

use clap::{Parser, Subcommand, ValueEnum};
use custos_domain::TaskStatus;
use custos_sdk::local_api::{ApiRequest, LocalApiClient, METHOD_CAPABILITIES_LIST};
use custos_sdk::profile::{ensure_daemon_client_async, ProfileResolver};
use serde_json::{json, Value};

#[derive(Debug, Parser)]
#[command(name = "custos", about = "Custos headless CLI client")]
pub struct CliArgs {
    /// Target daemon profile ID (defaults to CUSTOS_PROFILE / 'default')
    #[arg(long, global = true)]
    pub profile: Option<String>,

    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(Debug, Subcommand)]
pub enum CliCommand {
    /// Check daemon health via v1.ping
    #[command(alias = "health")]
    Ping,
    /// List daemon capability descriptors
    Capabilities,
    /// Task management commands
    Tasks {
        #[command(subcommand)]
        action: TaskCommand,
    },
    /// Run execution supervision and diagnostics
    Supervision {
        #[command(subcommand)]
        action: SupervisionCommand,
    },
    /// Send a raw JSON-RPC request to the daemon
    Request {
        /// Method name (e.g. v1.tasks.list)
        method: String,
        /// JSON object string passed as request params
        #[arg(long, default_value = "{}")]
        params: String,
        /// Optional request ID
        #[arg(long, default_value = "cli-request")]
        id: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum TaskCommand {
    /// List all tasks
    List,
    /// Get a task by ID
    Get { task_id: String },
    /// Create a new task
    Create { title: String },
    /// Advance task status
    Advance {
        task_id: String,
        #[arg(value_enum)]
        status: CliTaskStatus,
    },
    /// Cancel a task
    Cancel {
        task_id: String,
        #[arg(long)]
        reason: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum SupervisionCommand {
    /// Summary counters and active profiles
    Overview,
    /// List supervised execution runs
    Runs {
        #[arg(long)]
        workbench: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long, default_value_t = 30)]
        limit: usize,
    },
    /// Inspect full step waterfall, logs, errors, and findings for one run
    Inspect { run_id: String },
    /// List deterministic anomaly findings
    Findings {
        #[arg(long)]
        run_id: Option<String>,
        #[arg(long)]
        workbench: Option<String>,
        #[arg(long)]
        severity: Option<String>,
    },
    /// List grouped error signatures sorted by frequency
    Errors {
        #[arg(long)]
        workbench: Option<String>,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// List registered workbench supervision profiles
    Profiles,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliTaskStatus {
    Draft,
    Queued,
    Running,
    Blocked,
    Succeeded,
    Failed,
    Cancelled,
}

impl From<CliTaskStatus> for TaskStatus {
    fn from(status: CliTaskStatus) -> Self {
        match status {
            CliTaskStatus::Draft => TaskStatus::Draft,
            CliTaskStatus::Queued => TaskStatus::Queued,
            CliTaskStatus::Running => TaskStatus::Running,
            CliTaskStatus::Blocked => TaskStatus::Blocked,
            CliTaskStatus::Succeeded => TaskStatus::Succeeded,
            CliTaskStatus::Failed => TaskStatus::Failed,
            CliTaskStatus::Cancelled => TaskStatus::Cancelled,
        }
    }
}

pub fn resolve_profile(profile: Option<&str>) -> ProfileResolver {
    match profile {
        Some(id) => ProfileResolver::for_profile(id),
        None => ProfileResolver::from_env(),
    }
}

pub async fn execute_with_client(
    client: &LocalApiClient,
    command: CliCommand,
) -> Result<Value, String> {
    match command {
        CliCommand::Ping => {
            client.ping().await?;
            Ok(json!({ "status": "ok" }))
        }
        CliCommand::Capabilities => {
            let req = ApiRequest::new("cli-capabilities", METHOD_CAPABILITIES_LIST, json!({}));
            let resp = client.send_request(req).await?;
            if let Some(err) = resp.error {
                return Err(err);
            }
            resp.result
                .ok_or_else(|| "Empty result from capabilities endpoint".to_string())
        }
        CliCommand::Tasks { action } => execute_task_command(client, action).await,
        CliCommand::Supervision { action } => execute_supervision_command(client, action).await,
        CliCommand::Request { method, params, id } => {
            let parsed_params: Value = serde_json::from_str(&params)
                .map_err(|e| format!("Invalid JSON for --params: {e}"))?;
            let resp = client
                .send_request(ApiRequest::new(id, method, parsed_params))
                .await?;
            if let Some(err) = resp.error {
                return Err(err);
            }
            Ok(resp.result.unwrap_or(Value::Null))
        }
    }
}

async fn execute_task_command(
    client: &LocalApiClient,
    action: TaskCommand,
) -> Result<Value, String> {
    let val = match action {
        TaskCommand::List => {
            let tasks = client.list_tasks("cli-tasks-list").await?;
            serde_json::to_value(tasks).map_err(|e| e.to_string())?
        }
        TaskCommand::Get { task_id } => {
            let task = client.get_task("cli-tasks-get", &task_id).await?;
            serde_json::to_value(task).map_err(|e| e.to_string())?
        }
        TaskCommand::Create { title } => {
            let task = client
                .create_task("cli-tasks-create", &title, None, None)
                .await?;
            serde_json::to_value(task).map_err(|e| e.to_string())?
        }
        TaskCommand::Advance { task_id, status } => {
            let task = client
                .advance_task("cli-tasks-advance", &task_id, status.into())
                .await?;
            serde_json::to_value(task).map_err(|e| e.to_string())?
        }
        TaskCommand::Cancel { task_id, reason } => {
            let task = client
                .cancel_task("cli-tasks-cancel", &task_id, reason)
                .await?;
            serde_json::to_value(task).map_err(|e| e.to_string())?
        }
    };
    Ok(val)
}

async fn execute_supervision_command(
    client: &LocalApiClient,
    action: SupervisionCommand,
) -> Result<Value, String> {
    let (method, params) = match action {
        SupervisionCommand::Overview => ("v1.workflow.overview", json!({})),
        SupervisionCommand::Runs {
            workbench,
            status,
            limit,
        } => (
            "v1.workflow.runs",
            json!({ "workbench_id": workbench, "status": status, "limit": limit }),
        ),
        SupervisionCommand::Inspect { run_id } => {
            ("v1.workflow.inspect", json!({ "run_id": run_id }))
        }
        SupervisionCommand::Findings {
            run_id,
            workbench,
            severity,
        } => (
            "v1.workflow.findings",
            json!({ "run_id": run_id, "workbench_id": workbench, "severity": severity }),
        ),
        SupervisionCommand::Errors { workbench, limit } => (
            "v1.workflow.errors",
            json!({ "workbench_id": workbench, "limit": limit }),
        ),
        SupervisionCommand::Profiles => ("v1.workflow.profiles", json!({})),
    };
    let resp = client
        .send_request(ApiRequest::new("cli-supervision", method, params))
        .await?;
    if let Some(err) = resp.error {
        return Err(err);
    }
    Ok(resp.result.unwrap_or(Value::Null))
}

pub async fn run_async(args: CliArgs) -> Result<Value, String> {
    let profile = resolve_profile(args.profile.as_deref());
    let client = ensure_daemon_client_async(&profile).await?;
    execute_with_client(&client, args.command).await
}

pub fn run() {
    let args = CliArgs::parse();
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("Failed to initialize CLI runtime: {e}");
            std::process::exit(1);
        }
    };

    match rt.block_on(run_async(args)) {
        Ok(value) => match serde_json::to_string_pretty(&value) {
            Ok(rendered) => println!("{rendered}"),
            Err(e) => {
                eprintln!("Failed to serialize response: {e}");
                std::process::exit(1);
            }
        },
        Err(err) => {
            eprintln!("Error: {err}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use custos_domain::Task;
    use custos_sdk::local_api::{
        ApiResponse, FnTransport, METHOD_TASKS_ADVANCE, METHOD_TASKS_CANCEL, METHOD_TASKS_CREATE,
        METHOD_TASKS_GET, METHOD_TASKS_LIST,
    };

    fn sample_task(status: TaskStatus) -> Task {
        let mut task = Task::new("task_1".into(), "Headless CLI Task".into());
        task.status = status;
        task
    }

    fn mock_client() -> LocalApiClient {
        LocalApiClient::new(Box::new(FnTransport::new(|req: ApiRequest| async move {
            match req.method.as_str() {
                "v1.ping" => ApiResponse::success(req.id, json!({ "pong": true })),
                METHOD_CAPABILITIES_LIST => {
                    ApiResponse::success(req.id, json!([{ "id": "workspace.git" }]))
                }
                METHOD_TASKS_LIST => {
                    ApiResponse::success(req.id, json!([sample_task(TaskStatus::Draft)]))
                }
                METHOD_TASKS_GET | METHOD_TASKS_CREATE => {
                    ApiResponse::success(req.id, json!(sample_task(TaskStatus::Draft)))
                }
                METHOD_TASKS_ADVANCE => {
                    ApiResponse::success(req.id, json!(sample_task(TaskStatus::Running)))
                }
                METHOD_TASKS_CANCEL => {
                    ApiResponse::success(req.id, json!(sample_task(TaskStatus::Cancelled)))
                }
                other => ApiResponse::error(req.id, format!("unexpected method: {other}")),
            }
        })))
    }

    #[test]
    fn cli_parses_health_and_task_subcommands() {
        let parsed = CliArgs::try_parse_from(["custos", "--profile", "dev", "health"]).unwrap();
        assert_eq!(parsed.profile.as_deref(), Some("dev"));
        assert!(matches!(parsed.command, CliCommand::Ping));

        let parsed = CliArgs::try_parse_from([
            "custos",
            "tasks",
            "advance",
            "task_1",
            "running",
        ])
        .unwrap();
        assert!(matches!(
            parsed.command,
            CliCommand::Tasks {
                action: TaskCommand::Advance { .. }
            }
        ));
    }

    #[tokio::test]
    async fn headless_cli_dispatches_all_commands_without_tauri() {
        let client = mock_client();

        let ping = execute_with_client(&client, CliCommand::Ping).await.unwrap();
        assert_eq!(ping["status"], "ok");

        let caps = execute_with_client(&client, CliCommand::Capabilities)
            .await
            .unwrap();
        assert_eq!(caps[0]["id"], "workspace.git");

        let listed = execute_with_client(
            &client,
            CliCommand::Tasks {
                action: TaskCommand::List,
            },
        )
        .await
        .unwrap();
        assert_eq!(listed.as_array().unwrap().len(), 1);

        let advanced = execute_with_client(
            &client,
            CliCommand::Tasks {
                action: TaskCommand::Advance {
                    task_id: "task_1".into(),
                    status: CliTaskStatus::Running,
                },
            },
        )
        .await
        .unwrap();
        assert_eq!(advanced["status"], "running");

        let raw = execute_with_client(
            &client,
            CliCommand::Request {
                method: "v1.ping".into(),
                params: "{}".into(),
                id: "req-1".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(raw["pong"], true);
    }
}
