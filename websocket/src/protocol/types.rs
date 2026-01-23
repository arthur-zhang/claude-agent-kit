//! WebSocket protocol message types.
//!
//! This module defines all message types according to the protocol specification.
//! Messages use a flat JSON format with a "type" discriminator.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Decision type for permission responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Allow,
    Deny,
    AllowAlways,
}

/// Permission mode for session configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PermissionMode {
    Auto,
    #[default]
    Manual,
    Bypass,
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

/// Result subtype for Result messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultSubtype {
    Success,
    Error,
    Interrupted,
}

/// Session status for SessionInfo messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Paused,
    Completed,
    Error,
}

/// Session configuration passed during session_start.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Permission mode: auto, manual, or bypass
    #[serde(default)]
    pub permission_mode: PermissionMode,
    /// Maximum turns (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<i32>,
    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

/// Permission context for permission_request messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionContext {
    /// Human-readable description
    pub description: String,
    /// Risk level: low, medium, or high
    #[serde(default)]
    pub risk_level: RiskLevel,
}

/// Messages sent from client to server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// User message - sends a text message to the agent
    UserMessage {
        /// Unique message ID
        id: String,
        /// Session ID
        session_id: String,
        /// Message content (text or JSON string)
        content: String,
        /// Parent tool use ID (for tool result messages)
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_tool_use_id: Option<String>,
    },

    /// Permission response - responds to a permission request
    PermissionResponse {
        /// Unique message ID
        id: String,
        /// Session ID
        session_id: String,
        /// ID of the permission request being responded to
        request_id: String,
        /// Allow/deny decision
        decision: Decision,
        /// Optional explanation
        #[serde(skip_serializing_if = "Option::is_none")]
        explanation: Option<String>,
    },

    /// Session start - initialize a new session
    SessionStart {
        /// Unique message ID
        id: String,
        /// Session ID
        session_id: String,
        /// Session configuration
        #[serde(flatten)]
        config: SessionConfig,
    },

    /// Session end - terminate the session
    SessionEnd {
        /// Unique message ID
        id: String,
        /// Session ID
        session_id: String,
    },

    /// Interrupt - interrupt current execution
    Interrupt {
        /// Unique message ID
        id: String,
        /// Session ID
        session_id: String,
        /// Optional reason for interruption
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },

    /// Resume - resume after interrupt
    Resume {
        /// Unique message ID
        id: String,
        /// Session ID
        session_id: String,
    },

    /// Cancel - cancel a specific request
    Cancel {
        /// Unique message ID
        id: String,
        /// Session ID
        session_id: String,
        /// ID of the request to cancel
        target_id: String,
    },

    /// Tool result - explicit tool result message (alternative to parent_tool_use_id in user_message)
    ToolResult {
        /// Unique message ID
        id: String,
        /// Session ID
        session_id: String,
        /// Tool use ID this result is for
        tool_use_id: String,
        /// Result content
        content: String,
        /// Whether this result indicates an error
        #[serde(default)]
        is_error: bool,
    },
}

/// Delta content for assistant_message_delta messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "delta_type", rename_all = "snake_case")]
pub enum Delta {
    Text {
        text: String,
    },
    Thinking {
        thinking: String,
    },
    ToolUse {
        tool_use_id: String,
        tool_name: String,
        tool_input: serde_json::Value,
    },
}

/// Messages sent from server to client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Assistant message start - marks the beginning of an assistant response
    AssistantMessageStart {
        /// Unique message ID
        id: String,
        /// Session ID
        session_id: String,
        /// Model being used
        model: String,
    },

    /// Assistant message delta - streaming content update
    AssistantMessageDelta {
        /// Unique message ID
        id: String,
        /// Session ID
        session_id: String,
        /// Delta content
        delta: Delta,
    },

    /// Assistant message complete - marks the end of an assistant response
    AssistantMessageComplete {
        /// Unique message ID
        id: String,
        /// Session ID
        session_id: String,
    },

    /// Tool use - explicit tool use notification
    ToolUse {
        /// Unique message ID
        id: String,
        /// Session ID
        session_id: String,
        /// Tool use ID
        tool_use_id: String,
        /// Tool name
        tool_name: String,
        /// Tool input parameters
        tool_input: serde_json::Value,
    },

    /// Tool result - result from tool execution
    ToolResult {
        /// Unique message ID
        id: String,
        /// Session ID
        session_id: String,
        /// Associated request ID
        request_id: String,
        /// Tool use ID this result is for
        tool_use_id: String,
        /// Result content
        content: String,
        /// Whether this result indicates an error
        is_error: bool,
    },

    /// Permission request - request permission to execute a tool
    PermissionRequest {
        /// Unique message ID
        id: String,
        /// Session ID
        session_id: String,
        /// Tool name requiring permission
        tool_name: String,
        /// Tool input parameters
        tool_input: serde_json::Value,
        /// Permission context
        context: PermissionContext,
    },

    /// Result - final result message
    Result {
        /// Unique message ID
        id: String,
        /// Session ID
        session_id: String,
        /// Result subtype: success, error, or interrupted
        subtype: ResultSubtype,
        /// Duration in milliseconds
        duration_ms: u64,
        /// API duration in milliseconds
        duration_api_ms: u64,
        /// Number of turns
        num_turns: u32,
        /// Whether the result indicates an error
        is_error: bool,
        /// Optional error message
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
        /// Optional total cost in USD
        #[serde(skip_serializing_if = "Option::is_none")]
        total_cost_usd: Option<f64>,
    },

    /// Error - error message
    Error {
        /// Unique message ID
        id: String,
        /// Session ID
        session_id: String,
        /// Associated request ID (if applicable)
        #[serde(skip_serializing_if = "Option::is_none")]
        request_id: Option<String>,
        /// Error message
        message: String,
    },

    /// Warning - non-fatal warning
    Warning {
        /// Unique message ID
        id: String,
        /// Session ID
        session_id: String,
        /// Warning message
        message: String,
    },

    /// Session info - information about the current session
    SessionInfo {
        /// Unique message ID
        id: String,
        /// Session ID
        session_id: String,
        /// Session status
        status: SessionStatus,
    },

    /// Heartbeat - keep-alive message
    Heartbeat {
        /// Unique message ID
        id: String,
        /// Session ID
        session_id: String,
        /// Timestamp
        timestamp: u64,
    },
}

#[cfg(test)]
mod client_message_tests {
    use super::*;

    #[test]
    fn test_user_message_serialization() {
        let msg = ClientMessage::UserMessage {
            id: "msg-1".to_string(),
            session_id: "session-123".to_string(),
            content: "Hello, agent!".to_string(),
            parent_tool_use_id: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"user_message\""));
        assert!(json.contains("\"id\":\"msg-1\""));
    }

    #[test]
    fn test_permission_response_deserialization() {
        let json = r#"{
            "type": "permission_response",
            "id": "resp-1",
            "session_id": "session-123",
            "request_id": "req-1",
            "decision": "allow"
        }"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::PermissionResponse { decision, .. } => {
                assert!(matches!(decision, Decision::Allow));
            }
            _ => panic!("Expected PermissionResponse"),
        }
    }
}

#[cfg(test)]
mod server_message_tests {
    use super::*;

    #[test]
    fn test_server_message_serialization() {
        let msg = ServerMessage::AssistantMessageStart {
            id: "msg-1".to_string(),
            session_id: "session-123".to_string(),
            model: "claude-sonnet-4".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"assistant_message_start\""));
    }

    #[test]
    fn test_delta_text_serialization() {
        let delta = Delta::Text {
            text: "Hello".to_string(),
        };
        let json = serde_json::to_string(&delta).unwrap();
        assert!(json.contains("\"delta_type\":\"text\""));
        assert!(json.contains("\"text\":\"Hello\""));
    }

    #[test]
    fn test_result_subtype_serialization() {
        let subtype = ResultSubtype::Success;
        let json = serde_json::to_string(&subtype).unwrap();
        assert_eq!(json, "\"success\"");

        let subtype = ResultSubtype::Error;
        let json = serde_json::to_string(&subtype).unwrap();
        assert_eq!(json, "\"error\"");

        let subtype = ResultSubtype::Interrupted;
        let json = serde_json::to_string(&subtype).unwrap();
        assert_eq!(json, "\"interrupted\"");
    }

    #[test]
    fn test_session_status_serialization() {
        let status = SessionStatus::Active;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"active\"");

        let status = SessionStatus::Completed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"completed\"");
    }

    #[test]
    fn test_result_message_serialization() {
        let msg = ServerMessage::Result {
            id: "result-1".to_string(),
            session_id: "session-123".to_string(),
            subtype: ResultSubtype::Success,
            duration_ms: 1500,
            duration_api_ms: 1200,
            num_turns: 3,
            is_error: false,
            error: None,
            total_cost_usd: Some(0.05),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"result\""));
        assert!(json.contains("\"subtype\":\"success\""));
        assert!(json.contains("\"duration_ms\":1500"));
        assert!(json.contains("\"num_turns\":3"));
    }

    #[test]
    fn test_session_info_serialization() {
        let msg = ServerMessage::SessionInfo {
            id: "info-1".to_string(),
            session_id: "session-123".to_string(),
            status: SessionStatus::Active,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"session_info\""));
        assert!(json.contains("\"status\":\"active\""));
    }
}
