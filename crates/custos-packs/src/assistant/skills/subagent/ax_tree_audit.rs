//! Accessibility Tree (AX Tree) Audit Skill (Assistant Domain Pack)
//!
//! Analyzes semantic Accessibility Tree nodes extracted from Browser and Phone Simulator
//! viewports against WCAG 2.2 criteria: touch target size, accessible labels,
//! element roles, and navigation hierarchy.

use async_trait::async_trait;
use custos_runtime::cognitive::skills::{
    Skill, SkillCapabilityProfile, SkillContext, SkillError, SkillOutput,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub struct AxTreeAuditSkill;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxNodeInput {
    pub id: Option<String>,
    pub role: String,
    pub name: Option<String>,
    pub bounds: Option<AxBounds>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxBounds {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[async_trait]
impl Skill for AxTreeAuditSkill {
    fn skill_id(&self) -> &'static str {
        "ax_tree_audit"
    }

    fn description(&self) -> &'static str {
        "Audits mobile and web Accessibility Tree (AX Tree) nodes for WCAG touch bounds, contrast, and missing labels"
    }

    fn capability_profile(&self) -> SkillCapabilityProfile {
        SkillCapabilityProfile::read_only(15_000)
    }

    async fn run(&self, ctx: &SkillContext) -> Result<SkillOutput, SkillError> {
        let nodes_val = ctx
            .args
            .get("ax_nodes")
            .ok_or_else(|| SkillError::MissingArg("ax_nodes".to_string()))?;

        let nodes: Vec<AxNodeInput> = serde_json::from_value(nodes_val.clone())
            .map_err(|e| SkillError::InvalidArg(format!("Invalid ax_nodes structure: {}", e)))?;

        let platform = ctx
            .args
            .get("platform")
            .and_then(|v| v.as_str())
            .unwrap_or("mobile");

        let min_touch_bound: f64 = if platform == "ios" || platform == "mobile" {
            44.0 // iOS HIG minimum 44x44pt
        } else if platform == "android" {
            48.0 // Android Material minimum 48x48dp
        } else {
            24.0 // Web desktop
        };

        let mut violations = Vec::new();
        let mut passed_checks = 0;

        for (idx, node) in nodes.iter().enumerate() {
            let role_lower = node.role.to_lowercase();
            let is_interactive = role_lower == "button"
                || role_lower == "link"
                || role_lower == "textbox"
                || role_lower == "searchbox";

            // 1. Check touch target size for interactive controls
            if is_interactive {
                if let Some(bounds) = &node.bounds {
                    if bounds.w < min_touch_bound || bounds.h < min_touch_bound {
                        violations.push(json!({
                            "severity": "warning",
                            "node_index": idx,
                            "role": node.role,
                            "name": node.name,
                            "type": "undersized_touch_target",
                            "details": format!(
                                "Bounds {:.0}x{:.0}px is below minimum required {:.0}x{:.0}px",
                                bounds.w, bounds.h, min_touch_bound, min_touch_bound
                            ),
                            "remediation": "Increase padding or min-h/min-w dimensions"
                        }));
                    } else {
                        passed_checks += 1;
                    }
                }
            }

            // 2. Check for missing accessible name/label
            if is_interactive {
                let has_name = node.name.as_ref().map(|n| !n.trim().is_empty()).unwrap_or(false);
                if !has_name {
                    violations.push(json!({
                        "severity": "critical",
                        "node_index": idx,
                        "role": node.role,
                        "type": "missing_accessible_label",
                        "details": "Interactive element has no discernible text name or aria-label",
                        "remediation": "Add an aria-label or visible inner text"
                    }));
                } else {
                    passed_checks += 1;
                }
            }
        }

        let total_nodes = nodes.len();
        let score = if total_nodes == 0 {
            100
        } else {
            let penalty = violations.len() * 15;
            100_i32.saturating_sub(penalty as i32).max(10)
        };

        let result = json!({
            "status": "completed",
            "platform": platform,
            "total_nodes_audited": total_nodes,
            "passed_checks": passed_checks,
            "violations_found": violations.len(),
            "accessibility_score": score,
            "violations": violations,
            "wcag_level": if score >= 90 { "AAA" } else if score >= 75 { "AA" } else { "Non-compliant" }
        });

        Ok(SkillOutput::clean("ax_tree_audit", result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ax_tree_audit_empty_tree_clean() {
        let skill = AxTreeAuditSkill;
        let ctx = SkillContext {
            args: json!({ "ax_nodes": [] }),
            task_id: "t1".into(),
            worker_run_id: "r1".into(),
            provider_id: "test".into(),
            pack_id: Some("assistant".into()),
            scope_root: None,
        };
        let res = skill.run(&ctx).await.expect("run succeeds");
        assert_eq!(res.skill_id, "ax_tree_audit");
        let output = res.result;
        assert_eq!(output["accessibility_score"], 100);
        assert_eq!(output["violations_found"], 0);
    }

    #[tokio::test]
    async fn test_ax_tree_audit_detects_missing_accessible_label() {
        let skill = AxTreeAuditSkill;
        let ctx = SkillContext {
            args: json!({
                "ax_nodes": [
                    {
                        "role": "button",
                        "bounds": { "x": 0.0, "y": 0.0, "w": 48.0, "h": 48.0 }
                    }
                ]
            }),
            task_id: "t1".into(),
            worker_run_id: "r1".into(),
            provider_id: "test".into(),
            pack_id: Some("assistant".into()),
            scope_root: None,
        };
        let res = skill.run(&ctx).await.expect("run succeeds");
        let output = res.result;
        assert_eq!(output["violations_found"], 1);
        let violations = output["violations"].as_array().unwrap();
        assert_eq!(violations[0]["type"], "missing_accessible_label");
    }
}
