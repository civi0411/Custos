pub mod art;
pub mod assets;
pub mod banner;
pub mod diff;
pub mod prompt;
pub mod spinner;

use console::style;
use custos_core_domain::TaskStatus;

/// Reliably query the current terminal width in character columns.
/// Evaluates live console screen buffer dimensions on each call,
/// supporting live window resizing, with fallback to COLUMNS environment variable.
pub fn get_terminal_width() -> usize {
    if let Ok(c_str) = std::env::var("COLUMNS") {
        if let Ok(c) = c_str.trim().parse::<usize>() {
            if c >= 20 {
                return c;
            }
        }
    }
    let (_, term_cols) = console::Term::stdout().size();
    if term_cols >= 20 {
        term_cols as usize
    } else {
        100
    }
}

/// Responsive layout tier based on active terminal width
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponsiveTier {
    /// < 70 cols: Split-screen / Narrow
    Compact,
    /// 70..104 cols: Standard console window
    Standard,
    /// 105..159 cols: Wide desktop terminal
    Wide,
    /// >= 160 cols: Ultra-wide / Maximized
    UltraWide,
}

impl ResponsiveTier {
    pub fn current() -> Self {
        Self::from_width(get_terminal_width())
    }

    pub fn from_width(width: usize) -> Self {
        if width < 70 {
            ResponsiveTier::Compact
        } else if width < 105 {
            ResponsiveTier::Standard
        } else if width < 160 {
            ResponsiveTier::Wide
        } else {
            ResponsiveTier::UltraWide
        }
    }
}

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
