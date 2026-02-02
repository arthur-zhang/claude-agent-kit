//! Event conversion utilities for WebSocket protocol events.
//!
//! This module provides helper functions to create and manipulate protocol events.

use crate::protocol::events::{
    AgentEvent, FileOperation, PermissionContext, SessionInitData, TokenUsage, UserQuestion, SessionStatus,
};

/// Helper to create a SessionInit event from system init data
pub fn create_session_init(session_id: &str, data: &serde_json::Value) -> AgentEvent {
    // Deserialize the data into SessionInitData
    let init_data: SessionInitData = serde_json::from_value(data.clone())
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to deserialize session init data: {}", e);
            SessionInitData::default()
        });

    AgentEvent::SessionInit {
        success: true,
        session_id: session_id.to_string(),
        error: None,
        data: init_data,
    }
}

/// Helper to create a TurnStarted event
pub fn create_turn_started(session_id: &str) -> AgentEvent {
    AgentEvent::TurnStarted {
        session_id: session_id.to_string(),
    }
}

/// Helper to create a TurnCompleted event
pub fn create_turn_completed(
    session_id: &str,
    usage: TokenUsage,
    duration_ms: Option<u64>,
    duration_api_ms: Option<u64>,
    num_turns: Option<u32>,
    total_cost_usd: Option<f64>,
) -> AgentEvent {
    AgentEvent::TurnCompleted {
        session_id: session_id.to_string(),
        usage,
        duration_ms,
        duration_api_ms,
        num_turns,
        total_cost_usd,
    }
}

/// Helper to create a TurnFailed event
pub fn create_turn_failed(session_id: &str, error: String) -> AgentEvent {
    AgentEvent::TurnFailed {
        session_id: session_id.to_string(),
        error,
    }
}

/// Helper to create an AssistantMessage event
pub fn create_assistant_message(session_id: &str, text: String, is_final: bool) -> AgentEvent {
    AgentEvent::AssistantMessage {
        session_id: session_id.to_string(),
        text,
        is_final,
    }
}

/// Helper to create an AssistantReasoning event
pub fn create_assistant_reasoning(session_id: &str, text: String) -> AgentEvent {
    AgentEvent::AssistantReasoning {
        session_id: session_id.to_string(),
        text,
    }
}

/// Helper to create a ToolStarted event
pub fn create_tool_started(
    session_id: &str,
    tool_name: String,
    tool_id: String,
    arguments: serde_json::Value,
) -> AgentEvent {
    create_tool_started_with_parent(session_id, tool_name, tool_id, arguments, None)
}

/// Helper to create a ToolStarted event with optional parent_tool_use_id
pub fn create_tool_started_with_parent(
    session_id: &str,
    tool_name: String,
    tool_id: String,
    arguments: serde_json::Value,
    parent_tool_use_id: Option<String>,
) -> AgentEvent {
    AgentEvent::ToolStarted {
        session_id: session_id.to_string(),
        tool_name,
        tool_id,
        arguments,
        parent_tool_use_id,
    }
}

/// Helper to create a ToolCompleted event
pub fn create_tool_completed(
    session_id: &str,
    tool_id: String,
    success: bool,
    result: Option<String>,
    error: Option<String>,
) -> AgentEvent {
    create_tool_completed_with_parent(session_id, tool_id, success, result, error, None)
}

/// Helper to create a ToolCompleted event with optional parent_tool_use_id
pub fn create_tool_completed_with_parent(
    session_id: &str,
    tool_id: String,
    success: bool,
    result: Option<String>,
    error: Option<String>,
    parent_tool_use_id: Option<String>,
) -> AgentEvent {
    AgentEvent::ToolCompleted {
        session_id: session_id.to_string(),
        tool_id,
        success,
        result,
        error,
        parent_tool_use_id,
    }
}

/// Helper to create a ControlRequest event
pub fn create_control_request(
    session_id: &str,
    request_id: String,
    tool_name: String,
    tool_use_id: Option<String>,
    input: serde_json::Value,
    context: PermissionContext,
) -> AgentEvent {
    AgentEvent::ControlRequest {
        session_id: session_id.to_string(),
        request_id,
        tool_name,
        tool_use_id,
        input,
        context,
    }
}

/// Helper to create a FileChanged event
pub fn create_file_changed(session_id: &str, path: String, operation: FileOperation) -> AgentEvent {
    AgentEvent::FileChanged {
        session_id: session_id.to_string(),
        path,
        operation,
    }
}

/// Helper to create a CommandOutput event
pub fn create_command_output(
    session_id: &str,
    command: String,
    output: String,
    exit_code: Option<i32>,
    is_streaming: bool,
) -> AgentEvent {
    AgentEvent::CommandOutput {
        session_id: session_id.to_string(),
        command,
        output,
        exit_code,
        is_streaming,
    }
}

/// Helper to create a TokenUsage event
pub fn create_token_usage(
    session_id: &str,
    usage: TokenUsage,
    context_window: Option<i64>,
    usage_percent: Option<f32>,
) -> AgentEvent {
    AgentEvent::TokenUsage {
        session_id: session_id.to_string(),
        usage,
        context_window,
        usage_percent,
    }
}

/// Helper to create a ContextCompaction event
pub fn create_context_compaction(
    session_id: &str,
    reason: String,
    tokens_before: i64,
    tokens_after: i64,
) -> AgentEvent {
    AgentEvent::ContextCompaction {
        session_id: session_id.to_string(),
        reason,
        tokens_before,
        tokens_after,
    }
}

/// Helper to create an Error event
pub fn create_error(session_id: &str, message: String, is_fatal: bool) -> AgentEvent {
    AgentEvent::Error {
        session_id: session_id.to_string(),
        message,
        is_fatal,
    }
}

/// Helper to create an AskUserQuestion event
pub fn create_ask_user_question(
    session_id: &str,
    request_id: String,
    questions: Vec<UserQuestion>,
) -> AgentEvent {
    AgentEvent::AskUserQuestion {
        session_id: session_id.to_string(),
        request_id,
        questions,
    }
}

/// Helper to create an ExitPlanMode event
pub fn create_exit_plan_mode(
    session_id: &str,
    request_id: String,
    plan_file_path: Option<String>,
) -> AgentEvent {
    AgentEvent::ExitPlanMode {
        session_id: session_id.to_string(),
        request_id,
        plan_file_path,
    }
}

/// Helper to create a SessionInfo event
pub fn create_session_info(session_id: &str, status: SessionStatus) -> AgentEvent {
    AgentEvent::SessionInfo {
        session_id: session_id.to_string(),
        status,
    }
}

/// Helper to create a Heartbeat event
pub fn create_heartbeat(session_id: &str) -> AgentEvent {
    AgentEvent::Heartbeat {
        session_id: session_id.to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_session_init() {
        let data = serde_json::json!({
            "model": "claude-sonnet-4",
            "cwd": "/test/path",
            "tools": ["Task", "Bash"],
            "agents": ["general-purpose"]
        });
        let event = create_session_init("test-session", &data);
        match event {
            AgentEvent::SessionInit { session_id, data, .. } => {
                assert_eq!(session_id, "test-session");
                assert_eq!(data.model, Some("claude-sonnet-4".to_string()));
                assert_eq!(data.cwd, Some("/test/path".to_string()));
                assert_eq!(data.tools.len(), 2);
            }
            _ => panic!("Expected SessionInit event"),
        }
    }

    #[test]
    fn test_create_turn_started() {
        let event = create_turn_started("test-session");
        assert!(matches!(event, AgentEvent::TurnStarted { .. }));
    }

    #[test]
    fn test_create_token_usage() {
        let usage = TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            cached_tokens: 20,
            total_tokens: 150,
        };
        let event = create_token_usage("test-session", usage, Some(200000), Some(0.075));

        match event {
            AgentEvent::TokenUsage { usage, context_window, usage_percent, .. } => {
                assert_eq!(usage.input_tokens, 100);
                assert_eq!(usage.output_tokens, 50);
                assert_eq!(context_window, Some(200000));
                assert_eq!(usage_percent, Some(0.075));
            }
            _ => panic!("Expected TokenUsage event"),
        }
    }

    #[test]
    fn test_create_error() {
        let event = create_error("test-session", "Test error".to_string(), false);
        match event {
            AgentEvent::Error { message, is_fatal, .. } => {
                assert_eq!(message, "Test error");
                assert!(!is_fatal);
            }
            _ => panic!("Expected Error event"),
        }
    }

    #[test]
    fn test_create_heartbeat() {
        let event = create_heartbeat("test-session");
        match event {
            AgentEvent::Heartbeat { timestamp, .. } => {
                assert!(timestamp > 0);
            }
            _ => panic!("Expected Heartbeat event"),
        }
    }
}
