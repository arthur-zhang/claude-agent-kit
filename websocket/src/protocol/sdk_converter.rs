//! SDK to unified event conversion.
//!
//! Converts agent-sdk message types to the unified AgentEvent system.

use crate::protocol::event_converter::*;
use crate::protocol::events::{AgentEvent, SdkUsage, TokenUsage};
use claude_agent_sdk::{ContentBlock, ContentBlockContent, Message, MessageContent};

/// Convert SDK message to unified AgentEvent(s) with parent tool use context.
///
/// A single SDK message may map to multiple events.
/// The parent_tool_use_id indicates if this message is from a SubAgent.
pub fn sdk_to_events_with_parent(
    sdk_msg: &Message,
    session_id: &str,
    parent_tool_use_id: Option<&str>,
) -> Vec<AgentEvent> {
    match sdk_msg {
        Message::Assistant(assistant) => {
            // Use the message's own parent_tool_use_id if available, otherwise use the passed one
            let effective_parent = assistant
                .parent_tool_use_id
                .as_deref()
                .or(parent_tool_use_id);

            let mut events = vec![];

            // Process content blocks
            for block in &assistant.content {
                match block {
                    ContentBlock::Text { text } => {
                        events.push(create_assistant_message(session_id, text.clone(), false));
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        events.push(create_tool_started_with_parent(
                            session_id,
                            name.clone(),
                            id.clone(),
                            input.clone(),
                            effective_parent.map(|s| s.to_string()),
                        ));
                    }
                    ContentBlock::Thinking { thinking, .. } => {
                        events.push(create_assistant_reasoning(session_id, thinking.clone()));
                    }
                    other => {
                        tracing::warn!("Unhandled ContentBlock variant: {:?}", other);
                    }
                }
            }

            // Mark last text message as final if exists
            if let Some(last_event) = events.last_mut() {
                if let AgentEvent::AssistantMessage { is_final, .. } = last_event {
                    *is_final = true;
                }
            }

            events
        }
        // Delegate other message types to the original function
        _ => sdk_to_events(sdk_msg, session_id),
    }
}

/// Convert SDK message to unified AgentEvent(s).
///
/// A single SDK message may map to multiple events.
pub fn sdk_to_events(sdk_msg: &Message, session_id: &str) -> Vec<AgentEvent> {
    match sdk_msg {
        Message::Assistant(assistant) => {
            let mut events = vec![];

            // Get parent_tool_use_id from the assistant message (for SubAgent support)
            let parent_tool_use_id = assistant.parent_tool_use_id.clone();

            // Process content blocks
            for block in &assistant.content {
                match block {
                    ContentBlock::Text { text } => {
                        // Stream text as assistant message
                        events.push(create_assistant_message(
                            session_id,
                            text.clone(),
                            false, // Not final yet
                        ));
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        // Tool use started - pass parent_tool_use_id for SubAgent identification
                        events.push(create_tool_started_with_parent(
                            session_id,
                            name.clone(),
                            id.clone(),
                            input.clone(),
                            parent_tool_use_id.clone(),
                        ));
                    }
                    ContentBlock::Thinking { thinking, .. } => {
                        // Stream thinking as reasoning
                        events.push(create_assistant_reasoning(session_id, thinking.clone()));
                    }
                    other => {
                        tracing::warn!("Unhandled ContentBlock variant: {:?}", other);
                    }
                }
            }

            // Mark last text message as final if exists
            if let Some(last_event) = events.last_mut() {
                if let AgentEvent::AssistantMessage { is_final, .. } = last_event {
                    *is_final = true;
                }
            }

            events
        }

        Message::Result(result) => {
            // Extract token usage from result.usage field using serde
            let usage = if let Some(ref usage_value) = result.usage {
                // Deserialize SDK usage data
                match serde_json::from_value::<SdkUsage>(usage_value.clone()) {
                    Ok(sdk_usage) => TokenUsage {
                        input_tokens: sdk_usage.input_tokens,
                        output_tokens: sdk_usage.output_tokens,
                        cached_tokens: sdk_usage.cache_read_input_tokens,
                        total_tokens: sdk_usage.input_tokens + sdk_usage.output_tokens,
                    },
                    Err(e) => {
                        tracing::warn!("Failed to parse usage data: {}", e);
                        TokenUsage::default()
                    }
                }
            } else {
                TokenUsage::default()
            };

            if result.is_error {
                // Combine errors array and result field for error message
                let error_message = if !result.errors.is_empty() {
                    result.errors.join("; ")
                } else {
                    result.result.clone().unwrap_or_else(|| "Unknown error".to_string())
                };
                vec![create_turn_failed(session_id, error_message)]
            } else {
                vec![create_turn_completed(
                    session_id,
                    usage,
                    Some(result.duration_ms as u64),
                    Some(result.duration_api_ms as u64),
                    Some(result.num_turns as u32),
                    result.total_cost_usd,
                )]
            }
        }

        Message::System(system) => {
            tracing::debug!(
                "System message: subtype={}, extra={:?}",
                system.subtype,
                system.extra
            );

            // Handle init system message
            if system.subtype == "init" {
                tracing::info!("📋 Received init system message, converting to SessionInit");

                // Extract actual session_id from extra field (SDK's real session_id)
                let actual_session_id = system.extra
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or(session_id);

                tracing::info!("📋 Using session_id from init: {}", actual_session_id);
                tracing::debug!("📋 Init extra: {:?}", system.extra);

                // Pass the entire extra object as Value to create_session_init
                let extra_value = serde_json::Value::Object(system.extra.clone());
                return vec![create_session_init(actual_session_id, &extra_value)];
            }

            // Other system messages are not forwarded to client
            vec![]
        }

        Message::User(user) => {
            // Handle User messages that contain ToolResult blocks
            let mut events = vec![];

            // Get parent_tool_use_id from the user message (for SubAgent support)
            let parent_tool_use_id = user.parent_tool_use_id.clone();

            // Check if content is Blocks variant
            if let MessageContent::Blocks(blocks) = &user.content {
                for block in blocks {
                    match block {
                        ContentBlock::ToolResult {
                            tool_use_id,
                            content,
                            is_error,
                        } => {
                            let content_str = match content {
                                Some(ContentBlockContent::String(s)) => s.clone(),
                                Some(ContentBlockContent::Array(arr)) => {
                                    serde_json::to_string(arr).unwrap_or_default()
                                }
                                None => String::new(),
                            };

                            let success = !is_error.unwrap_or(false);
                            let error = if success {
                                None
                            } else {
                                Some(content_str.clone())
                            };
                            let result = if success { Some(content_str) } else { None };

                            events.push(create_tool_completed_with_parent(
                                session_id,
                                tool_use_id.clone(),
                                success,
                                result,
                                error,
                                parent_tool_use_id.clone(),
                            ));
                        }
                        ContentBlock::Text { text } => {
                            tracing::debug!("User message text: {}", text);
                        }
                        other => {
                            tracing::debug!("Unhandled User ContentBlock variant: {:?}", other);
                        }
                    }
                }
            } else if let MessageContent::String(text) = &user.content {
                tracing::debug!("User message string: {}", text);
            }

            events
        }

        Message::Stream(stream) => {
            // Handle stream events
            tracing::debug!("Stream event: {:?}", stream.event);

            // Check if this is a turn started event
            if let Some(event_type) = stream.event.get("type").and_then(|v| v.as_str()) {
                match event_type {
                    "turn_started" => {
                        vec![create_turn_started(session_id)]
                    }
                    _ => {
                        tracing::debug!("Unhandled stream event type: {}", event_type);
                        vec![]
                    }
                }
            } else {
                vec![]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_sdk_assistant_text_to_events() {
        let sdk_msg = Message::Assistant(claude_agent_sdk::AssistantMessage {
            content: vec![ContentBlock::Text {
                text: "Hello, world!".to_string(),
            }],
            model: "claude-sonnet-4".to_string(),
            parent_tool_use_id: None,
            error: None,
        });

        let events = sdk_to_events(&sdk_msg, "session-123");

        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::AssistantMessage { text, is_final, .. } => {
                assert_eq!(text, "Hello, world!");
                assert!(is_final);
            }
            _ => panic!("Expected AssistantMessage event"),
        }
    }

    #[test]
    fn test_sdk_assistant_thinking_to_events() {
        let sdk_msg = Message::Assistant(claude_agent_sdk::AssistantMessage {
            content: vec![ContentBlock::Thinking {
                thinking: "Let me think...".to_string(),
                signature: String::new(),
            }],
            model: "claude-sonnet-4".to_string(),
            parent_tool_use_id: None,
            error: None,
        });

        let events = sdk_to_events(&sdk_msg, "session-123");

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], AgentEvent::AssistantReasoning { .. }));
    }

    #[test]
    fn test_sdk_tool_use_to_events() {
        let sdk_msg = Message::Assistant(claude_agent_sdk::AssistantMessage {
            content: vec![ContentBlock::ToolUse {
                id: "tool-1".to_string(),
                name: "Bash".to_string(),
                input: json!({"command": "ls"}),
            }],
            model: "claude-sonnet-4".to_string(),
            parent_tool_use_id: None,
            error: None,
        });

        let events = sdk_to_events(&sdk_msg, "session-123");

        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::ToolStarted {
                tool_name, tool_id, ..
            } => {
                assert_eq!(tool_name, "Bash");
                assert_eq!(tool_id, "tool-1");
            }
            _ => panic!("Expected ToolStarted event"),
        }
    }

    #[test]
    fn test_sdk_result_success_to_events() {
        use serde_json::json;

        let sdk_msg = Message::Result(claude_agent_sdk::ResultMessage {
            subtype: "success".to_string(),
            duration_ms: 1500,
            duration_api_ms: 1200,
            num_turns: 3,
            is_error: false,
            session_id: "session-123".to_string(),
            result: None,
            total_cost_usd: Some(0.05),
            usage: Some(json!({
                "input_tokens": 1773,
                "output_tokens": 478,
                "cache_read_input_tokens": 32255
            })),
            structured_output: None,
            errors: vec![],
        });

        let events = sdk_to_events(&sdk_msg, "session-123");

        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::TurnCompleted { usage, .. } => {
                assert_eq!(usage.input_tokens, 1773);
                assert_eq!(usage.output_tokens, 478);
                assert_eq!(usage.cached_tokens, 32255);
                assert_eq!(usage.total_tokens, 1773 + 478);
            }
            _ => panic!("Expected TurnCompleted event"),
        }
    }

    #[test]
    fn test_sdk_result_error_to_events() {
        let sdk_msg = Message::Result(claude_agent_sdk::ResultMessage {
            subtype: "error".to_string(),
            duration_ms: 500,
            duration_api_ms: 400,
            num_turns: 1,
            is_error: true,
            session_id: "session-123".to_string(),
            result: Some("API error".to_string()),
            total_cost_usd: None,
            usage: None,
            structured_output: None,
            errors: vec![],
        });

        let events = sdk_to_events(&sdk_msg, "session-123");

        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::TurnFailed { error, .. } => {
                assert_eq!(error, "API error");
            }
            _ => panic!("Expected TurnFailed event"),
        }
    }

    #[test]
    fn test_sdk_result_error_with_errors_array() {
        let sdk_msg = Message::Result(claude_agent_sdk::ResultMessage {
            subtype: "error_during_execution".to_string(),
            duration_ms: 0,
            duration_api_ms: 0,
            num_turns: 0,
            is_error: true,
            session_id: "session-123".to_string(),
            result: None,
            total_cost_usd: Some(0.0),
            usage: None,
            structured_output: None,
            errors: vec!["No conversation found with session ID: abc123".to_string()],
        });

        let events = sdk_to_events(&sdk_msg, "session-123");

        assert_eq!(events.len(), 1);
        match &events[0] {
            AgentEvent::TurnFailed { error, .. } => {
                assert_eq!(error, "No conversation found with session ID: abc123");
            }
            _ => panic!("Expected TurnFailed event"),
        }
    }

    #[test]
    fn test_sdk_mixed_content_to_events() {
        let sdk_msg = Message::Assistant(claude_agent_sdk::AssistantMessage {
            content: vec![
                ContentBlock::Thinking {
                    thinking: "Let me check...".to_string(),
                    signature: String::new(),
                },
                ContentBlock::Text {
                    text: "I found the issue.".to_string(),
                },
                ContentBlock::ToolUse {
                    id: "tool-1".to_string(),
                    name: "Edit".to_string(),
                    input: json!({"file": "test.rs"}),
                },
            ],
            model: "claude-sonnet-4".to_string(),
            parent_tool_use_id: None,
            error: None,
        });

        let events = sdk_to_events(&sdk_msg, "session-123");

        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], AgentEvent::AssistantReasoning { .. }));
        assert!(matches!(events[1], AgentEvent::AssistantMessage { .. }));
        assert!(matches!(events[2], AgentEvent::ToolStarted { .. }));
    }
}
