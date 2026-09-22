pub mod banner;
pub mod diff;
pub mod prompt;
pub mod spinner;

use console::style;
use custos_core_domain::TaskStatus;

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
