//! Common types shared between protocol modules.
//!
//! This module contains types that are used by both `events.rs` and `types.rs`
//! to avoid duplication and ensure consistency.

use serde::{Deserialize, Serialize};

/// Decision type for permission responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Allow,
    Deny,
    AllowAlways,
}

/// Permission mode for session configuration.
/// Maps to Claude Code CLI's --permission-mode flag values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    /// Default permission mode - prompts for dangerous operations
    #[default]
    Default,
    /// Accept edits mode - auto-approve file edits, prompt for other dangerous ops
    AcceptEdits,
    /// Bypass permissions mode - auto-approve all operations (requires dangerously_skip_permissions)
    BypassPermissions,
    /// Plan mode - only allow planning, no execution
    Plan,
    /// Delegate mode - delegate permission decisions to parent process
    Delegate,
    /// Don't ask mode - deny all permission requests without prompting
    DontAsk,
}

/// Risk level for permission context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    #[default]
    Medium,
    High,
}

/// Permission context for permission requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionContext {
    /// Human-readable description
    pub description: String,
    /// Risk level: low, medium, or high
    #[serde(default)]
    pub risk_level: RiskLevel,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_serialization() {
        assert_eq!(serde_json::to_string(&Decision::Allow).unwrap(), "\"allow\"");
        assert_eq!(serde_json::to_string(&Decision::Deny).unwrap(), "\"deny\"");
        assert_eq!(serde_json::to_string(&Decision::AllowAlways).unwrap(), "\"allow_always\"");
    }

    #[test]
    fn test_permission_mode_serialization() {
        assert_eq!(serde_json::to_string(&PermissionMode::Default).unwrap(), "\"default\"");
        assert_eq!(serde_json::to_string(&PermissionMode::AcceptEdits).unwrap(), "\"acceptEdits\"");
        assert_eq!(serde_json::to_string(&PermissionMode::BypassPermissions).unwrap(), "\"bypassPermissions\"");
        assert_eq!(serde_json::to_string(&PermissionMode::Plan).unwrap(), "\"plan\"");
        assert_eq!(serde_json::to_string(&PermissionMode::Delegate).unwrap(), "\"delegate\"");
        assert_eq!(serde_json::to_string(&PermissionMode::DontAsk).unwrap(), "\"dontAsk\"");
    }

    #[test]
    fn test_permission_mode_deserialization() {
        assert_eq!(serde_json::from_str::<PermissionMode>("\"default\"").unwrap(), PermissionMode::Default);
        assert_eq!(serde_json::from_str::<PermissionMode>("\"acceptEdits\"").unwrap(), PermissionMode::AcceptEdits);
        assert_eq!(serde_json::from_str::<PermissionMode>("\"bypassPermissions\"").unwrap(), PermissionMode::BypassPermissions);
        assert_eq!(serde_json::from_str::<PermissionMode>("\"plan\"").unwrap(), PermissionMode::Plan);
        assert_eq!(serde_json::from_str::<PermissionMode>("\"delegate\"").unwrap(), PermissionMode::Delegate);
        assert_eq!(serde_json::from_str::<PermissionMode>("\"dontAsk\"").unwrap(), PermissionMode::DontAsk);
    }

    #[test]
    fn test_risk_level_serialization() {
        assert_eq!(serde_json::to_string(&RiskLevel::Low).unwrap(), "\"low\"");
        assert_eq!(serde_json::to_string(&RiskLevel::Medium).unwrap(), "\"medium\"");
        assert_eq!(serde_json::to_string(&RiskLevel::High).unwrap(), "\"high\"");
    }
}
