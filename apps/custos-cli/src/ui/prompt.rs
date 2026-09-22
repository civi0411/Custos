use console::style;
use dialoguer::{Confirm, Input};
use std::error::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn badge(&self) -> String {
        match self {
            RiskLevel::Low => format!("{}", style("[LOW RISK]").green()),
            RiskLevel::Medium => format!("{}", style("[MEDIUM RISK]").yellow()),
            RiskLevel::High => format!("{}", style("[HIGH RISK]").red().bold()),
            RiskLevel::Critical => format!("{}", style("[CRITICAL RISK]").white().on_red().bold()),
        }
    }
}

pub fn confirm_execution(
    action_type: &str,
    target: &str,
    risk: RiskLevel,
) -> Result<bool, Box<dyn Error>> {
    println!(
        "\n{} {} -> {}",
        risk.badge(),
        style(action_type).bold().cyan(),
        style(target).white()
    );

    let confirmed = Confirm::new()
        .with_prompt("Approve this execution permit?")
        .default(risk == RiskLevel::Low)
        .interact()?;

    Ok(confirmed)
}

pub fn prompt_user_input(prompt_text: &str) -> Result<String, Box<dyn Error>> {
    let input: String = Input::new()
        .with_prompt(format!("{}", style(prompt_text).bold().green()))
        .interact_text()?;
    Ok(input)
}
