use console::style;
use dialoguer::{Confirm, Input, Select};
use std::error::Error;
use super::OperationalMode;

#[allow(dead_code)]
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
            RiskLevel::Low => format!("{}", style("[LOW RISK]").cyan()),
            RiskLevel::Medium => format!("{}", style("[MEDIUM RISK]").yellow().bold()),
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

    match Confirm::new()
        .with_prompt("Approve this execution permit?")
        .default(risk == RiskLevel::Low)
        .interact()
    {
        Ok(c) => Ok(c),
        Err(_) => {
            use std::io::{self, BufRead};
            let stdin = io::stdin();
            let mut line = String::new();
            if stdin.lock().read_line(&mut line).is_ok() {
                let trimmed = line.trim().to_lowercase();
                if trimmed == "y" || trimmed == "yes" {
                    return Ok(true);
                } else if trimmed == "n" || trimmed == "no" {
                    return Ok(false);
                }
            }
            Ok(risk == RiskLevel::Low)
        }
    }
}

pub fn wait_for_mode_prompt() -> Result<(), Box<dyn Error>> {
    use std::io::{self, BufRead, Write};
    print!(
        "  {} {}",
        style("❄ Mode:").bold().cyan(),
        style("Nhấn [Enter] để vào chọn chế độ hoạt động (Operational Mode)...").bold().white()
    );
    let _ = io::stdout().flush();
    let stdin = io::stdin();
    let mut line = String::new();
    let _ = stdin.lock().read_line(&mut line);
    println!();
    Ok(())
}

pub fn prompt_mode_selection() -> Result<OperationalMode, Box<dyn Error>> {
    let options = vec![
        "Code       • Lập trình, sửa lỗi & tái cấu trúc mã nguồn",
        "Research   • Điều tra, kiến trúc & suy luận chuyên sâu",
        "Assitant   • Trợ lý điều phối quy trình tự động",
    ];

    match Select::new()
        .with_prompt(format!("{}", style("❄ Chọn chế độ Custos (Operational Mode)").bold().cyan()))
        .default(0)
        .items(&options)
        .interact()
    {
        Ok(0) => Ok(OperationalMode::Code),
        Ok(1) => Ok(OperationalMode::Research),
        Ok(_) => Ok(OperationalMode::Assitant),
        Err(_) => Ok(OperationalMode::Code),
    }
}

pub fn prompt_user_input(prompt_text: &str) -> Result<String, Box<dyn Error>> {
    match Input::new()
        .with_prompt(format!("{}", style(prompt_text).bold().cyan()))
        .interact_text()
    {
        Ok(s) => Ok(s),
        Err(_) => {
            use std::io::{self, BufRead};
            let stdin = io::stdin();
            let mut line = String::new();
            stdin.lock().read_line(&mut line)?;
            let trimmed = line.trim().to_string();
            if trimmed.is_empty() {
                Ok("Default agentic goal".into())
            } else {
                Ok(trimmed)
            }
        }
    }
}
