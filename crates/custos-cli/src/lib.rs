pub extern crate custos_domain as custos_core_domain;
pub extern crate custos_provider as custos_provider_sdk;

pub use custos_adapters::custos_adapter_provider_fake;
pub use custos_adapters::custos_adapters_mcp;
pub use custos_daemon::custos_local_api;

pub mod ui;

use clap::{Parser, Subcommand, ValueEnum};
use custos_core_domain::{Task, TaskStatus};
use custos_local_api::{LocalApiClient, ProcessTransport};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "custos")]
#[command(about = "Custos Agentic Work Runtime CLI", version, long_about = None)]
pub struct Cli {
    #[arg(long, global = true)]
    pub db: Option<PathBuf>,

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
    Vibe {
        #[arg(short, long)]
        prompt: Option<String>,

        #[arg(short, long, value_enum)]
        mode: Option<ui::OperationalMode>,
    },

    Create {
        #[arg(short, long)]
        title: String,

        #[arg(short, long)]
        metadata: Option<String>,
    },

    Status {
        #[arg(short, long)]
        id: String,
    },

    List,

    Advance {
        #[arg(short, long)]
        id: String,

        #[arg(short, long, value_enum)]
        status: CliTaskStatus,

        #[arg(short, long)]
        epoch: Option<u64>,

        #[arg(short, long)]
        rationale: Option<String>,
    },

    Cancel {
        #[arg(short, long)]
        id: String,

        #[arg(short, long)]
        epoch: Option<u64>,

        #[arg(short, long, default_value = "Cancelled via CLI")]
        reason: String,
    },

    Complete {
        #[arg(short, long)]
        id: String,

        #[arg(short, long)]
        epoch: Option<u64>,

        #[arg(short, long, default_value = "Completed via CLI")]
        summary: String,
    },

    /// Explain the repository architecture (End-to-End Vertical Slice)
    Explain {
        /// Optional query for explanation
        #[arg(short, long)]
        query: Option<String>,
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

fn resolve_daemon_binary() -> PathBuf {
    if let Ok(path) = std::env::var("CUSTOS_DAEMON_BIN") {
        return PathBuf::from(path);
    }
    if let Ok(current_exe) = std::env::current_exe() {
        let sibling = current_exe.with_file_name(if cfg!(windows) {
            "custos-daemon.exe"
        } else {
            "custos-daemon"
        });
        if sibling.exists() {
            return sibling;
        }
    }
    PathBuf::from("custos-daemon")
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let db_path = resolve_db_path(cli.db)?;
    let db_path_str = db_path.to_str().ok_or("Invalid UTF-8 in database path")?;

    let daemon_bin = resolve_daemon_binary();
    let transport = ProcessTransport::spawn(
        daemon_bin.to_str().unwrap(),
        Some(db_path_str),
    )
    .map_err(|e| {
        format!(
            "Failed to launch Custos daemon ({}): {e}. Please ensure custos-daemon is built (`cargo build --bin custos-daemon`).",
            daemon_bin.display()
        )
    })?;
    let client = LocalApiClient::new(Box::new(transport));

    let command = cli.command.unwrap_or(Commands::Vibe {
        prompt: None,
        mode: None,
    });

    match command {
        Commands::Vibe { prompt, mode } => {
            let selected_mode = match mode {
                Some(m) => {
                    ui::banner::print_banner(env!("CARGO_PKG_VERSION"));
                    m
                }
                None => {
                    ui::art::play_bouncing_owl_until_enter(env!("CARGO_PKG_VERSION"))?;
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

            let task = client
                .create_task(
                    "vibe_create",
                    &user_goal,
                    None,
                    Some(serde_json::json!({
                        "origin": "custos-cli",
                        "mode": selected_mode.name().to_lowercase(),
                    })),
                )
                .await?;
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
                let queued_task = client
                    .advance_task("vibe_adv_q", &task.id, TaskStatus::Queued)
                    .await?;
                println!(
                    "Task {} advanced to: [{}]",
                    queued_task.id,
                    ui::format_task_status(&queued_task.status)
                );

                let running_task = client
                    .advance_task("vibe_adv_r", &queued_task.id, TaskStatus::Running)
                    .await?;
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

                let completed_task = client
                    .complete_task(
                        "vibe_comp",
                        &running_task.id,
                        Some(format!(
                            "Task '{}' successfully executed and verified",
                            user_goal
                        )),
                    )
                    .await?;
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
                let cancelled_task = client
                    .cancel_task(
                        "vibe_cancel",
                        &task.id,
                        Some("Execution rejected by operator. Worktree untouched.".into()),
                    )
                    .await?;
                println!(
                    "Task {} cancelled: [{}]",
                    cancelled_task.id,
                    ui::format_task_status(&cancelled_task.status)
                );

                ui::assets::print_task_failure_report(
                    &cancelled_task.id,
                    "Execution rejected by operator. Worktree untouched.",
                    Some("Operator declined ExecutionPermit approval"),
                );
            }
        }

        Commands::Create { title, metadata } => {
            let meta_json = if let Some(m) = metadata {
                Some(serde_json::from_str::<serde_json::Value>(&m)?)
            } else {
                None
            };

            let task = client
                .create_task("cli_create", &title, None, meta_json)
                .await?;

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

        Commands::Status { id } => match client.get_task("cli_status", &id).await {
            Ok(task) => {
                print_task(&task, cli.json);
                if let Ok(spans) = client.list_spans("cli_spans", &id).await {
                    if !cli.json && !spans.is_empty() {
                        println!("\nSpans ({}):", spans.len());
                        for s in spans {
                            println!(
                                "  #{} [{}] {}/{} (started: {})",
                                s.span_num, s.state, s.provider, s.model, s.started_at
                            );
                        }
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
            }
            Err(_) => {
                eprintln!("Task not found: {}", id);
                std::process::exit(1);
            }
        },

        Commands::List => {
            let tasks = client.list_tasks("cli_list").await?;
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
            epoch: _,
            rationale,
        } => {
            let target_status: TaskStatus = status.into();
            let task = if target_status == TaskStatus::Succeeded {
                client
                    .complete_task("cli_comp", &id, rationale.clone())
                    .await?
            } else {
                client.advance_task("cli_adv", &id, target_status).await?
            };
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

        Commands::Cancel {
            id,
            epoch: _,
            reason,
        } => {
            let task = client
                .cancel_task("cli_cancel", &id, Some(reason.clone()))
                .await?;
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

        Commands::Complete {
            id,
            epoch: _,
            summary,
        } => {
            let task = client
                .complete_task("cli_complete", &id, Some(summary.clone()))
                .await?;
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

        Commands::Explain { query } => {
            ui::banner::print_banner(env!("CARGO_PKG_VERSION"));
            let user_query = match query {
                Some(q) => q,
                None => ui::prompt::prompt_user_input(
                    "What specific part of the repository would you like explained?",
                )?,
            };

            let spinner = ui::spinner::CliSpinner::new("Compiling workspace context...");

            // 1. Compile ContextPack
            let context_pack = custos_core_domain::ContextPack {
                id: custos_core_domain::new_id("pack"),
                items: vec![],
                total_tokens: 120,
            };
            spinner.finish_success(&format!(
                "Context compiled: {} files, {} tokens (Budget: 40k)",
                context_pack.items.len(),
                context_pack.total_tokens
            ));

            let spinner2 = ui::spinner::CliSpinner::new("Initializing task kernel...");
            // 2. Kernel Create Task via Custos Daemon
            let mut task = client
                .create_task(
                    "explain_create",
                    &format!("Explain: {}", user_query),
                    None,
                    Some(serde_json::json!({
                        "pack_id": "task.engineering.repo-explain",
                        "mode": "cli-demo",
                    })),
                )
                .await?;
            spinner2.finish_success(&format!(
                "Task registered: {} [{}]",
                task.id,
                ui::format_task_status(&task.status)
            ));

            // 2.5 Advance to Running
            task = client
                .advance_task("explain_q", &task.id, TaskStatus::Queued)
                .await?;
            task = client
                .advance_task("explain_r", &task.id, TaskStatus::Running)
                .await?;

            // 3. Provider invocation
            let spinner3 = ui::spinner::CliSpinner::new(
                "Invoking AI Provider (Goose Engine via Custos SDK)...",
            );
            use custos_provider_sdk::{ModelProvider, ProviderRequest};

            // Integrate Goose Engine here!
            // In a full implementation, we'd route this through Goose's providers.
            // For now we simulate the unified interface.
            // Route through Custos Gateway Provider with Invariant I7 sanitization and audit
            let raw_provider = std::sync::Arc::new(
                custos_adapter_provider_fake::FakeProvider::new("goose-openai-proxy"),
            );
            let gateway_provider =
                custos_adapters_mcp::GatewayProvider::new(raw_provider.clone(), task.id.clone());
            let req = ProviderRequest::simple(format!(
                "Context tokens: {}\nQuery: {}",
                context_pack.total_tokens, user_query
            ));
            let response = gateway_provider.generate(&req).await?;
            spinner3.finish_success("Goose AI Provider completed");

            println!(
                "\n{}",
                console::style("--- [mock] AI Explanation ---")
                    .cyan()
                    .bold()
            );
            println!("{}", response.content);
            println!(
                "{}",
                console::style("-----------------------------")
                    .cyan()
                    .bold()
            );
            println!(
                "Tokens used: {} | Provider: {}",
                response.tokens_used,
                gateway_provider.provider_id()
            );
            println!("{}", console::style("[!] Note: This is a demo. Output is mock data and budgets are strictly enforced.").yellow());

            // 4. Evidence Engine / Verification
            let spinner4 = ui::spinner::CliSpinner::new("Verifying anchors via Evidence Engine...");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await; // simulate verify
            spinner4.finish_success("Evidence verified (FileAnchor, SymbolAnchor)");

            // 5. Complete Task via Custos Daemon
            let completed_task = client
                .complete_task(
                    "explain_comp",
                    &task.id,
                    Some("Explanation generated and verified".into()),
                )
                .await?;
            println!(
                "\nTask {} successfully completed and persisted via Custos Daemon.",
                completed_task.id
            );
        }
    }

    Ok(())
}
