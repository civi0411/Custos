//! Browser Subagent & Accessibility Skills
//!
//! Provides skills for auditing accessibility trees and dispatching autonomous
//! browser subagent workflows within Custos OI boundaries.

pub mod ax_tree_audit;
pub mod browser_subagent_dispatch;

pub use ax_tree_audit::AxTreeAuditSkill;
pub use browser_subagent_dispatch::BrowserSubagentDispatchSkill;
