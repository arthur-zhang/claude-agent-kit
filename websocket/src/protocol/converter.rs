//! Protocol message conversion.
//!
//! Converts between agent-sdk message types and WebSocket protocol message types.

use crate::protocol::types::*;
use claude_agent_sdk::{Message, ContentBlock};
use uuid::Uuid;

/// Convert SDK message to protocol message(s).
///
/// A single SDK message may map to multiple protocol messages
/// (e.g., Assistant → start + deltas + complete).
pub fn sdk_to_protocol(sdk_msg: &Message, session_id: &str) -> Vec<ServerMessage> {
    match sdk_msg {
        Message::Assistant(assistant) => {
            let msg_id = Uuid::new_v4().to_string();
            let mut result = vec![];

            // Start message
            result.push(ServerMessage::AssistantMessageStart {
                id: msg_id.clone(),
                session_id: session_id.to_string(),
                model: assistant.model.clone(),
            });

            // Content blocks
            for block in &assistant.content {
                match block {
                    ContentBlock::Text { text } => {
                        result.push(ServerMessage::AssistantMessageDelta {
                            id: msg_id.clone(),
                            session_id: session_id.to_string(),
                            delta: Delta::Text {
                                text: text.clone(),
                            },
                        });
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        // Send as separate tool_use message only
                        result.push(ServerMessage::ToolUse {
                            id: Uuid::new_v4().to_string(),
                            session_id: session_id.to_string(),
                            tool_use_id: id.clone(),
                            tool_name: name.clone(),
                            tool_input: input.clone(),
                        });
                    }
                    ContentBlock::Thinking { thinking, .. } => {
                        result.push(ServerMessage::AssistantMessageDelta {
                            id: msg_id.clone(),
                            session_id: session_id.to_string(),
                            delta: Delta::Thinking {
                                thinking: thinking.clone(),
                            },
                        });
                    }
                    _ => {}
                }
            }

            // Complete message
            result.push(ServerMessage::AssistantMessageComplete {
                id: msg_id,
                session_id: session_id.to_string(),
            });

            result
        }
        Message::Result(result) => {
            // Convert string subtype to enum
            let subtype = match result.subtype.as_str() {
                "error" => ResultSubtype::Error,
                "interrupted" => ResultSubtype::Interrupted,
                _ => ResultSubtype::Success,
            };

            vec![ServerMessage::Result {
                id: Uuid::new_v4().to_string(),
                session_id: session_id.to_string(),
                subtype,
                duration_ms: result.duration_ms as u64,
                duration_api_ms: result.duration_api_ms as u64,
                num_turns: result.num_turns as u32,
                is_error: result.is_error,
                error: result.result.clone(),
                total_cost_usd: result.total_cost_usd,
            }]
        }
        Message::System(system) => {
            // System messages like init are handled internally
            tracing::debug!("System message: subtype={}, data={:?}", system.subtype, system.data);
            vec![]
        }
        _ => vec![],
    }
}

/// Convert protocol client message to SDK input.
pub fn protocol_to_sdk_input(client_msg: &ClientMessage) -> Option<SdkInput> {
    match client_msg {
        ClientMessage::UserMessage { content, parent_tool_use_id, .. } => {
            Some(SdkInput::Query {
                content: content.clone(),
                parent_tool_use_id: parent_tool_use_id.clone(),
            })
        }
        ClientMessage::PermissionResponse { decision, .. } => {
            Some(SdkInput::PermissionDecision {
                allow: matches!(decision, Decision::Allow | Decision::AllowAlways),
            })
        }
        _ => None,
    }
}

/// SDK input representation for conversion.
#[derive(Debug, Clone)]
pub enum SdkInput {
    Query {
        content: String,
        parent_tool_use_id: Option<String>,
    },
    PermissionDecision {
        allow: bool,
    },
}

#[cfg(test)]
mod converter_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_sdk_assistant_message_to_protocol() {
        // This will fail initially - we'll implement after
        let sdk_msg = Message::Assistant(claude_agent_sdk::AssistantMessage {
            content: vec![
                ContentBlock::Text {
                    text: "Hello, world!".to_string(),
                },
            ],
            model: "claude-sonnet-4".to_string(),
            parent_tool_use_id: None,
            error: None,
        });

        let protocol_msgs = crate::protocol::converter::sdk_to_protocol(&sdk_msg, "session-123");

        // Should produce: start, delta (text), complete
        assert_eq!(protocol_msgs.len(), 3);
        assert!(matches!(protocol_msgs[0], ServerMessage::AssistantMessageStart { .. }));
        assert!(matches!(protocol_msgs[2], ServerMessage::AssistantMessageComplete { .. }));
    }

    #[test]
    fn test_sdk_tool_use_to_protocol() {
        let sdk_msg = Message::Assistant(claude_agent_sdk::AssistantMessage {
            content: vec![
                ContentBlock::ToolUse {
                    id: "tool-1".to_string(),
                    name: "Bash".to_string(),
                    input: json!({"command": "ls"}),
                },
            ],
            model: "claude-sonnet-4".to_string(),
            parent_tool_use_id: None,
            error: None,
        });

        let protocol_msgs = crate::protocol::converter::sdk_to_protocol(&sdk_msg, "session-123");

        // Should produce: start, tool_use, complete
        assert_eq!(protocol_msgs.len(), 3);
    }
}
