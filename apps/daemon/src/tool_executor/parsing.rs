use custos_contracts::DomainError;

pub(crate) fn is_known_workbench_tool(name: &str) -> bool {
    matches!(
        name,
        "run_backtest"
            | "quant_bars_read"
            | "quant_indicators_compute"
            | "quant_lookahead_audit"
            | "quant_dsr_calculate"
            | "quant_pbo_evaluate"
            | "quant_evidence_assess"
            | "quant_factors_discover"
            | "quant_positions_read"
            | "quant_review_analyze"
            | "quant_order_propose"
            | "quant_emergency_kill_switch"
            | "search_literature"
            | "schedule_task"
    )
}

pub(crate) fn parse_tool_invocation(
    name: &str,
    params: &serde_json::Value,
) -> Result<(String, custos_contracts::RiskLevel, String), DomainError> {
    parse_tool_with_registry(name, params, None)
}

pub(crate) fn parse_tool_with_registry(
    name: &str,
    params: &serde_json::Value,
    registry: Option<&custos_engine::DomainPackRegistry>,
) -> Result<(String, custos_contracts::RiskLevel, String), DomainError> {
    // 1. Parameter safety check: directory traversal
    for key in &["path", "file", "target_file", "target", "dir"] {
        if let Some(val) = params.get(key).and_then(|v| v.as_str()) {
            if val.contains("..") {
                return Err(DomainError::Validation(format!(
                    "Path traversal is forbidden in parameter '{key}': '{val}'"
                )));
            }
        }
    }

    // 2. Resolve action name, risk level, and target
    if name.contains("__") {
        let risk = if custos_integrations::mcp::security::is_dangerous_tool_pattern(name) {
            custos_contracts::RiskLevel::High
        } else {
            custos_contracts::RiskLevel::Medium
        };
        let tgt = params
            .get("target")
            .or_else(|| params.get("path"))
            .or_else(|| params.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or(name);
        Ok((name.to_string(), risk, tgt.to_string()))
    } else {
        match name {
            "read_file" | "read" => {
                let tgt = params
                    .get("path")
                    .or_else(|| params.get("file"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("workspace");
                Ok((
                    "read_file".to_string(),
                    custos_contracts::RiskLevel::Low,
                    tgt.to_string(),
                ))
            }
            "write_file" | "write" => {
                let tgt = params
                    .get("path")
                    .or_else(|| params.get("file"))
                    .or_else(|| params.get("target_file"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("workspace");
                Ok((
                    "write_file".to_string(),
                    custos_contracts::RiskLevel::Medium,
                    tgt.to_string(),
                ))
            }
            "list_dir" | "list" | "list_files" => {
                let tgt = params
                    .get("path")
                    .or_else(|| params.get("dir"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(".");
                Ok((
                    "list_dir".to_string(),
                    custos_contracts::RiskLevel::Low,
                    tgt.to_string(),
                ))
            }
            "bash" | "shell" | "exec" => {
                let cmd = params
                    .get("command")
                    .or_else(|| params.get("cmd"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let risk = if cmd.contains("mkfs")
                    || cmd.contains("dd if=")
                    || cmd.contains("format_disk")
                {
                    custos_contracts::RiskLevel::Critical
                } else {
                    custos_contracts::RiskLevel::High
                };
                Ok(("bash".to_string(), risk, cmd.to_string()))
            }
            "patch_preview" => {
                let tgt = params
                    .get("path")
                    .or_else(|| params.get("file"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("workspace");
                Ok((
                    "patch_preview".to_string(),
                    custos_contracts::RiskLevel::Low,
                    tgt.to_string(),
                ))
            }
            domain_tool => {
                // Check if domain_tool is declared by an enabled pack in registry
                if let Some(reg) = registry {
                    if reg.is_tool_registered(domain_tool) {
                        let risk = if matches!(
                            domain_tool,
                            "quant_order_propose" | "quant_emergency_kill_switch"
                        ) {
                            custos_contracts::RiskLevel::High
                        } else if domain_tool.starts_with("run_")
                            || domain_tool.starts_with("edit_")
                            || domain_tool.starts_with("schedule_")
                            || domain_tool == "quant_factors_discover"
                        {
                            custos_contracts::RiskLevel::Medium
                        } else {
                            custos_contracts::RiskLevel::Low
                        };
                        let tgt = params
                            .get("strategy_id")
                            .or_else(|| params.get("query"))
                            .or_else(|| params.get("title"))
                            .or_else(|| params.get("path"))
                            .or_else(|| params.get("target"))
                            .and_then(|v| v.as_str())
                            .unwrap_or(domain_tool);
                        return Ok((domain_tool.to_string(), risk, tgt.to_string()));
                    }
                }

                // Check if domain_tool belongs to known workbenches that are uninstalled or disabled
                if is_known_workbench_tool(domain_tool) {
                    return Err(DomainError::Unauthorized(format!(
                        "Tool '{domain_tool}' is unavailable because its workbench pack is not installed or enabled"
                    )));
                }

                Err(DomainError::Validation(format!(
                    "Unknown or unmapped tool: '{domain_tool}'"
                )))
            }
        }
    }
}
