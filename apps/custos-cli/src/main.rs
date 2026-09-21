//! Custos CLI

use clap::{Parser, Subcommand};
use custos_persistence_sqlite::SqliteTaskStore;
use custos_task_kernel::TaskService;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "custos")]
#[command(about = "Custos Agentic Work Runtime CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new task
    Create {
        #[arg(short, long)]
        title: String,
    },
    /// Inspect status of a task
    Status {
        #[arg(short, long)]
        id: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let store = Arc::new(SqliteTaskStore::new_in_memory()?);
    let service = TaskService::new(store);

    match cli.command {
        Commands::Create { title } => {
            let task = service.create_task(title).await?;
            println!("Created task: {} [{}]", task.id, task.status);
        }
        Commands::Status { id } => {
            if let Some(task) = service.get_task(&id).await? {
                println!(
                    "Task: {} - {} [{}] (epoch: {})",
                    task.id, task.title, task.status, task.epoch
                );
            } else {
                eprintln!("Task not found: {}", id);
            }
        }
    }

    Ok(())
}
