pub mod art;
pub mod assets;
pub mod banner;
pub mod diff;
pub mod prompt;
pub mod spinner;

use console::style;
use custos_core_domain::TaskStatus;

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationalMode {
    Code,
    Research,
    Assitant,
}

impl OperationalMode {
    pub fn name(&self) -> &'static str {
        match self {
            OperationalMode::Code => "Code",
            OperationalMode::Research => "Research",
            OperationalMode::Assitant => "Assitant",
        }
    }

    #[allow(dead_code)]
    pub fn description(&self) -> &'static str {
        match self {
            OperationalMode::Code => "Software engineering, refactoring, and code analysis",
            OperationalMode::Research => "Deep investigation, architecture deliberation, and synthesis",
            OperationalMode::Assitant => "Autonomous workflow orchestration and task execution",
        }
    }
}

pub fn format_task_status(status: &TaskStatus) -> String {
    match status {
        TaskStatus::Draft => format!("{}", style(status).dim()),
        TaskStatus::Queued => format!("{}", style(status).yellow()),
        TaskStatus::Running => format!("{}", style(status).cyan().bold()),
        TaskStatus::Blocked => format!("{}", style(status).magenta().bold()),
        TaskStatus::Succeeded => format!("{}", style(status).green().bold()),
        TaskStatus::Failed => format!("{}", style(status).red().bold()),
        TaskStatus::Cancelled => format!("{}", style(status).dim().red()),
    }
}
