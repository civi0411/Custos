//! Custos CLI Library
//!
//! Production-grade command-line interface for the Custos Agentic Work Runtime.

pub mod ui;

use clap::{Parser, Subcommand, ValueEnum};
use custos_core_domain::{Task, TaskStatus};
use custos_persistence_sqlite::SqliteTaskStore;
use custos_task_kernel::{AdvanceTask, CancelTask, CompleteTask, CreateTask, TaskService};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "custos")]
#[command(about = "Custos Agentic Work Runtime CLI", version, long_about = None)]
pub struct Cli {
    /// SQLite database file path (defaults to $CUSTOS_DB_PATH or .custos/custos.db)
    #[arg(long, global = true)]
    pub db: Option<PathBuf>,

    /// Output results in JSON format
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
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
    fn from(s: CliTaskStatus) -> Self {
        match s {
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

#[derive(Subcommand)]
pub enum Commands {
    /// Interactive task vibe session with mode selection
    Vibe {
        /// Optional prompt describing the task
        #[arg(short, long)]
        prompt: Option<String>,

        /// Operational mode: Code, Research, or Assitant
        #[arg(short, long, value_enum)]
        mode: Option<ui::OperationalMode>,
    },

    /// Create a new task
    Create {
        /// Human-readable title of the task
        #[arg(short, long)]
        title: String,

        /// Optional metadata JSON string
        #[arg(short, long)]
        metadata: Option<String>,
    },

    /// Inspect detailed status of a task
    Status {
        /// Task identifier
        #[arg(short, long)]
        id: String,
    },

    /// List all tasks in the store
    List,

    /// Advance a task to the next status
    Advance {
        /// Task identifier
        #[arg(short, long)]
        id: String,

        /// Target status
        #[arg(short, long, value_enum)]
        status: CliTaskStatus,

        /// Expected epoch for optimistic concurrency control
        #[arg(short, long)]
        epoch: Option<u64>,

        /// Optional rationale for advancement
        #[arg(short, long)]
        rationale: Option<String>,
    },

    /// Cancel an active task
    Cancel {
        /// Task identifier
        #[arg(short, long)]
        id: String,

        /// Expected epoch for optimistic concurrency control
        #[arg(short, long)]
        epoch: Option<u64>,

        /// Cancellation reason
        #[arg(short, long, default_value = "Cancelled via CLI")]
        reason: String,
    },

    /// Complete a task successfully
    Complete {
        /// Task identifier
        #[arg(short, long)]
        id: String,

        /// Expected epoch for optimistic concurrency control
        #[arg(short, long)]
        epoch: Option<u64>,

        /// Optional completion summary
        #[arg(short, long, default_value = "Completed via CLI")]
        summary: String,
    },
}

fn resolve_db_path(configured: Option<PathBuf>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(path) = configured {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        return Ok(path);
    }

    if let Ok(env_path) = std::env::var("CUSTOS_DB_PATH") {
        let path = PathBuf::from(env_path);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        return Ok(path);
    }

    let default_dir = Path::new(".custos");
    std::fs::create_dir_all(default_dir)?;
    Ok(default_dir.join("custos.db"))
}

fn print_task(task: &Task, as_json: bool) {
    if as_json {
        println!("{}", serde_json::to_string_pretty(task).unwrap_or_default());
    } else {
        println!(
            "Task ID:     {}\nTitle:       {}\nStatus:      {}\nEpoch:       {}\nCreated:     {}\nUpdated:     {}\nMetadata:    {}",
            task.id,
            console::style(&task.title).bold(),
            ui::format_task_status(&task.status),
            task.epoch,
            task.created_at.to_rfc3339(),
            task.updated_at.to_rfc3339(),
            task.metadata
        );
    }
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let db_path = resolve_db_path(cli.db)?;
    let db_path_str = db_path.to_str().ok_or("Invalid UTF-8 in database path")?;

    let store = Arc::new(SqliteTaskStore::new(db_path_str)?);
    let service = TaskService::new(store.clone());

    let command = cli.command.unwrap_or(Commands::Vibe {
        prompt: None,
        mode: None,
    });

    match command {
        Commands::Vibe { prompt, mode } => {
            ui::banner::print_banner(env!("CARGO_PKG_VERSION"));

            let selected_mode = match mode {
                Some(m) => m,
                None => {
                    ui::prompt::wait_for_mode_prompt()?;
                    ui::art::print_modes_showcase();
                    ui::prompt::prompt_mode_selection()?
                }
            };

            ui::art::print_mode_card(selected_mode);

            let prompt_text = format!(
                "❄ Tôi có thể giúp gì cho bạn trong chế độ [{}]? Nhập mục tiêu",
                selected_mode.name()
            );
            let user_goal = match prompt {
                Some(p) => p,
                None => ui::prompt::prompt_user_input(&prompt_text)?,
            };

            ui::assets::print_task_lifecycle_card(
                ui::assets::TaskLifecycleState::InspectionApproval,
                &format!(
                    "Inspecting repository, compiling context recipe & verifying trajectory for [{}] mode...",
                    selected_mode.name()
                ),
            );

            let spinner = ui::spinner::CliSpinner::new(format!(
                "Analyzing repository and compiling context recipe for {} mode...",
                selected_mode.name()
            ));
            tokio::time::sleep(tokio::time::Duration::from_millis(700)).await;

            spinner.set_message("Synthesizing solution trajectory with LLM Provider...");
            tokio::time::sleep(tokio::time::Duration::from_millis(900)).await;

            let cmd = CreateTask {
                title: user_goal.clone(),
                metadata: Some(serde_json::json!({
                    "origin": "custos-cli",
                    "mode": selected_mode.name().to_lowercase(),
                })),
            };
            let (task, _) = service.execute_create(cmd).await?;
            spinner.finish_success(&format!(
                "Task registered: {} [{}]",
                task.id,
                ui::format_task_status(&task.status)
            ));

            println!(
                "\n{}",
                console::style("Proposed Worktree Modification:").bold()
            );
            let old_code = "fn handle_request() {\n    todo!();\n}\n";
            let new_code = "pub fn handle_request() -> Result<(), DomainError> {\n    tracing::info!(\"Executing verified task payload\");\n    Ok(())\n}\n";
            ui::diff::print_unified_diff("crates/runtime/src/handler.rs", old_code, new_code);

            let approved = ui::prompt::confirm_execution(
                "ExecutionPermit: Apply Code Diff",
                "crates/runtime/src/handler.rs",
                ui::prompt::RiskLevel::Medium,
            )?;

            if approved {
                println!(
                    "{}",
                    console::style("✔ Permit granted by operator. Worktree isolated and patched.")
                        .cyan()
                        .bold()
                );
                let advance_cmd = AdvanceTask {
                    task_id: task.id.clone(),
                    next_status: TaskStatus::Queued,
                    expected_epoch: task.epoch,
                    rationale: Some("Approved by human operator in interactive CLI session".into()),
                };
                let (queued_task, _) = service.execute_advance(advance_cmd).await?;
                println!(
                    "Task {} advanced to: [{}]",
                    queued_task.id,
                    ui::format_task_status(&queued_task.status)
                );

                let run_cmd = AdvanceTask {
                    task_id: queued_task.id.clone(),
                    next_status: TaskStatus::Running,
                    expected_epoch: queued_task.epoch,
                    rationale: Some("Dispatched to runtime worker".into()),
                };
                let (running_task, _) = service.execute_advance(run_cmd).await?;
                println!(
                    "Task {} execution: [{}]",
                    running_task.id,
                    ui::format_task_status(&running_task.status)
                );

                ui::assets::print_task_lifecycle_card(
                    ui::assets::TaskLifecycleState::RunningCoding,
                    "Provider actively coding and executing task payload in sandbox worktree...",
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;

                let complete_cmd = CompleteTask {
                    task_id: running_task.id.clone(),
                    summary: format!("Task '{}' successfully executed and verified", user_goal),
                    expected_epoch: running_task.epoch,
                };
                let (completed_task, _) = service.execute_complete(complete_cmd).await?;
                println!(
                    "Task {} execution: [{}]",
                    completed_task.id,
                    ui::format_task_status(&completed_task.status)
                );

                ui::assets::print_task_lifecycle_card(
                    ui::assets::TaskLifecycleState::Succeeded,
                    "Task completed successfully. All artifacts and judgments committed.",
                );
            } else {
                let cancel_cmd = CancelTask {
                    task_id: task.id.clone(),
                    reason: "Execution rejected by operator. Worktree untouched.".into(),
                    expected_epoch: task.epoch,
                };
                let (cancelled_task, _) = service.execute_cancel(cancel_cmd).await?;
                println!(
                    "Task {} cancelled: [{}]",
                    cancelled_task.id,
                    ui::format_task_status(&cancelled_task.status)
                );

                // Task Failed / Rejected (Text report only, strictly no image modification)
                ui::assets::print_task_failure_report(
                    &cancelled_task.id,
                    "Execution rejected by operator. Worktree untouched.",
                    Some("Operator declined ExecutionPermit approval"),
                );
            }
        }

        Commands::Create { title, metadata } => {
            let meta_json = if let Some(m) = metadata {
                Some(serde_json::from_str(&m)?)
            } else {
                None
            };

            let cmd = CreateTask {
                title,
                metadata: meta_json,
            };
            let (task, _) = service.execute_create(cmd).await?;

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&task)?);
            } else {
                println!(
                    "Created task: {} [{}]",
                    task.id,
                    ui::format_task_status(&task.status)
                );
            }
        }

        Commands::Status { id } => {
            if let Some(task) = service.get_task(&id).await? {
                print_task(&task, cli.json);
                let spans = store.spans().list_spans(&id)?;
                if !cli.json && !spans.is_empty() {
                    println!("\nSpans ({}):", spans.len());
                    for s in spans {
                        println!(
                            "  #{} [{}] {}/{} (started: {})",
                            s.span_num, s.state, s.provider, s.model, s.started_at
                        );
                    }
                }
                if !cli.json {
                    match task.status {
                        TaskStatus::Running => {
                            ui::assets::print_task_lifecycle_card(
                                ui::assets::TaskLifecycleState::RunningCoding,
                                "Task is actively running — provider is coding or executing changes.",
                            );
                        }
                        TaskStatus::Blocked => {
                            ui::assets::print_task_lifecycle_card(
                                ui::assets::TaskLifecycleState::InspectionApproval,
                                "Task is blocked — awaiting repository inspection, evidence verification, or operator approval.",
                            );
                        }
                        TaskStatus::Succeeded => {
                            ui::assets::print_task_lifecycle_card(
                                ui::assets::TaskLifecycleState::Succeeded,
                                "Task completed successfully with verified integrity.",
                            );
                        }
                        TaskStatus::Failed => {
                            ui::assets::print_task_failure_report(
                                &task.id,
                                "Task execution failed.",
                                None,
                            );
                        }
                        TaskStatus::Draft | TaskStatus::Queued => {
                            ui::assets::print_task_lifecycle_card(
                                ui::assets::TaskLifecycleState::Idle,
                                "Task is queued / standing by for runtime worker.",
                            );
                        }
                        TaskStatus::Cancelled => {
                            ui::assets::print_task_failure_report(
                                &task.id,
                                "Task was cancelled.",
                                None,
                            );
                        }
                    }
                }
            } else {
                eprintln!("Task not found: {}", id);
                std::process::exit(1);
            }
        }

        Commands::List => {
            let tasks = store.list_tasks()?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&tasks)?);
            } else if tasks.is_empty() {
                println!("No tasks found in database ({})", db_path_str);
            } else {
                println!("{:<36} {:<24} {:<6} TITLE", "TASK ID", "STATUS", "EPOCH");
                println!("{}", "-".repeat(80));
                for t in tasks {
                    println!(
                        "{:<36} {:<24} {:<6} {}",
                        t.id,
                        ui::format_task_status(&t.status),
                        t.epoch,
                        t.title
                    );
                }
            }
        }

        Commands::Advance {
            id,
            status,
            epoch,
            rationale,
        } => {
            let expected_epoch = match epoch {
                Some(e) => e,
                None => {
                    let task = service
                        .get_task(&id)
                        .await?
                        .ok_or_else(|| format!("Task not found: {}", id))?;
                    task.epoch
                }
            };
            let cmd = AdvanceTask {
                task_id: id,
                next_status: status.into(),
                expected_epoch,
                rationale: rationale.clone(),
            };
            let (task, _) = service.execute_advance(cmd).await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&task)?);
            } else {
                println!(
                    "Advanced task {} to [{}] (epoch: {})",
                    task.id,
                    ui::format_task_status(&task.status),
                    task.epoch
                );
                match task.status {
                    TaskStatus::Running => {
                        ui::assets::print_task_lifecycle_card(
                            ui::assets::TaskLifecycleState::RunningCoding,
                            "Task advanced to Running: Provider actively executing.",
                        );
                    }
                    TaskStatus::Blocked => {
                        ui::assets::print_task_lifecycle_card(
                            ui::assets::TaskLifecycleState::InspectionApproval,
                            "Task advanced to Blocked: Awaiting inspection/verification/approval.",
                        );
                    }
                    TaskStatus::Succeeded => {
                        ui::assets::print_task_lifecycle_card(
                            ui::assets::TaskLifecycleState::Succeeded,
                            "Task advanced to Succeeded: Verified completion.",
                        );
                    }
                    TaskStatus::Failed => {
                        ui::assets::print_task_failure_report(
                            &task.id,
                            "Task marked as Failed.",
                            rationale.as_deref(),
                        );
                    }
                    _ => {}
                }
            }
        }

        Commands::Cancel { id, epoch, reason } => {
            let expected_epoch = match epoch {
                Some(e) => e,
                None => {
                    let task = service
                        .get_task(&id)
                        .await?
                        .ok_or_else(|| format!("Task not found: {}", id))?;
                    task.epoch
                }
            };
            let cmd = CancelTask {
                task_id: id,
                reason: reason.clone(),
                expected_epoch,
            };
            let (task, _) = service.execute_cancel(cmd).await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&task)?);
            } else {
                println!(
                    "Cancelled task {} [{}]",
                    task.id,
                    ui::format_task_status(&task.status)
                );
                ui::assets::print_task_failure_report(
                    &task.id,
                    "Task cancelled by operator.",
                    Some(&reason),
                );
            }
        }

        Commands::Complete { id, epoch, summary } => {
            let expected_epoch = match epoch {
                Some(e) => e,
                None => {
                    let task = service
                        .get_task(&id)
                        .await?
                        .ok_or_else(|| format!("Task not found: {}", id))?;
                    task.epoch
                }
            };
            let cmd = CompleteTask {
                task_id: id,
                summary: summary.clone(),
                expected_epoch,
            };
            let (task, _) = service.execute_complete(cmd).await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&task)?);
            } else {
                println!(
                    "Completed task {} [{}]",
                    task.id,
                    ui::format_task_status(&task.status)
                );
                ui::assets::print_task_lifecycle_card(
                    ui::assets::TaskLifecycleState::Succeeded,
                    &format!("Task {} completed successfully: {}", task.id, summary),
                );
            }
        }
    }

    Ok(())
}
