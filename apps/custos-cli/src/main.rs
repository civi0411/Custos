//! Custos CLI
//!
//! Production-grade command-line interface for the Custos Agentic Work Runtime.

use clap::{Parser, Subcommand, ValueEnum};
use custos_core_domain::{Task, TaskStatus};
use custos_persistence_sqlite::SqliteTaskStore;
use custos_task_kernel::{AdvanceTask, CancelTask, CompleteTask, CreateTask, TaskService};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "custos")]
#[command(about = "Custos Agentic Work Runtime CLI", version, long_about = None)]
struct Cli {
    /// SQLite database file path (defaults to $CUSTOS_DB_PATH or .custos/custos.db)
    #[arg(long, global = true)]
    db: Option<PathBuf>,

    /// Output results in JSON format
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum CliTaskStatus {
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
enum Commands {
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
            task.title,
            task.status,
            task.epoch,
            task.created_at.to_rfc3339(),
            task.updated_at.to_rfc3339(),
            task.metadata
        );
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let db_path = resolve_db_path(cli.db)?;
    let db_path_str = db_path.to_str().ok_or("Invalid UTF-8 in database path")?;

    let store = Arc::new(SqliteTaskStore::new(db_path_str)?);
    let service = TaskService::new(store.clone());

    match cli.command {
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
                println!("Created task: {} [{}]", task.id, task.status);
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
                println!("{:<36} {:<12} {:<6} TITLE", "TASK ID", "STATUS", "EPOCH");
                println!("{}", "-".repeat(80));
                for t in tasks {
                    println!(
                        "{:<36} {:<12} {:<6} {}",
                        t.id,
                        t.status.to_string(),
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
                rationale,
            };
            let (task, _) = service.execute_advance(cmd).await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&task)?);
            } else {
                println!(
                    "Advanced task {} to [{}] (epoch: {})",
                    task.id, task.status, task.epoch
                );
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
                reason,
                expected_epoch,
            };
            let (task, _) = service.execute_cancel(cmd).await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&task)?);
            } else {
                println!("Cancelled task {} [{}]", task.id, task.status);
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
                summary,
                expected_epoch,
            };
            let (task, _) = service.execute_complete(cmd).await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&task)?);
            } else {
                println!("Completed task {} [{}]", task.id, task.status);
            }
        }
    }

    Ok(())
}
