//! WebSocket protocol message types.
//!
//! This module defines all message types according to the protocol specification.
//! Messages use a flat JSON format with a "type" discriminator.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Decision type for permission responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Allow,
    Deny,
    AllowAlways,
}

/// Session configuration passed during session_start.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Session ID
    pub session_id: String,
    /// Permission mode: "auto", "manual", "bypass"
    #[serde(default = "default_permission_mode")]
    pub permission_mode: String,
    /// Maximum turns (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<i32>,
    /// Additional metadata
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

fn default_permission_mode() -> String {
    "manual".to_string()
}

/// Permission context for permission_request messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionContext {
    /// Human-readable description
    pub description: String,
    /// Risk level: "low", "medium", "high"
    #[serde(default = "default_risk_level")]
    pub risk_level: String,
}

fn default_risk_level() -> String {
    "medium".to_string()
}

/// Messages sent from client to server.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
