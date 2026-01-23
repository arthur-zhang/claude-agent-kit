//! WebSocket protocol integration tests.

use websocket::protocol::types::*;
// use websocket::session::handler::{handle_session, HandlerConfig};

#[tokio::test]
async fn test_session_start_flow() {
    // Test: session_start → session_info
    // TODO: Implement test
}

#[tokio::test]
async fn test_user_message_flow() {
    // Test: user_message → assistant_message_start → deltas → assistant_message_complete
    // TODO: Implement test
}

#[tokio::test]
async fn test_permission_request_flow() {
    // Test: tool_use → permission_request → permission_response → tool_result
    // TODO: Implement test
}

#[tokio::test]
async fn test_message_serialization() {
    // Test: Verify protocol messages serialize/deserialize correctly
    let msg = ClientMessage::UserMessage {
        id: "test-1".to_string(),
        session_id: "session-123".to_string(),
        content: "Hello".to_string(),
        parent_tool_use_id: None,
    };

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"type\":\"user_message\""));

    let parsed: ClientMessage = serde_json::from_str(&json).unwrap();
    match parsed {
        ClientMessage::UserMessage { content, .. } => {
            assert_eq!(content, "Hello");
        }
        _ => panic!("Wrong message type"),
    }
}

#[tokio::test]
async fn test_server_message_serialization() {
    // Test: Verify server messages serialize correctly
    let msg = ServerMessage::AssistantMessageStart {
        id: "msg-1".to_string(),
        session_id: "session-123".to_string(),
        model: "claude-sonnet-4".to_string(),
    };

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"type\":\"assistant_message_start\""));
    assert!(json.contains("\"model\":\"claude-sonnet-4\""));

    let parsed: ServerMessage = serde_json::from_str(&json).unwrap();
    match parsed {
        ServerMessage::AssistantMessageStart { model, .. } => {
            assert_eq!(model, "claude-sonnet-4");
        }
        _ => panic!("Wrong message type"),
    }
}

#[tokio::test]
async fn test_delta_serialization() {
    // Test: Verify delta types serialize correctly
    let delta = Delta::Text {
        text: "Hello world".to_string(),
    };

    let json = serde_json::to_string(&delta).unwrap();
    assert!(json.contains("\"delta_type\":\"text\""));
    assert!(json.contains("\"text\":\"Hello world\""));

    let parsed: Delta = serde_json::from_str(&json).unwrap();
    match parsed {
        Delta::Text { text } => {
            assert_eq!(text, "Hello world");
        }
        _ => panic!("Wrong delta type"),
    }
}

#[tokio::test]
async fn test_permission_response_serialization() {
    // Test: Verify permission response serializes correctly
    let msg = ClientMessage::PermissionResponse {
        id: "resp-1".to_string(),
        session_id: "session-123".to_string(),
        request_id: "req-1".to_string(),
        decision: Decision::Allow,
        explanation: Some("User approved".to_string()),
    };

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"type\":\"permission_response\""));
    assert!(json.contains("\"decision\":\"allow\""));

    let parsed: ClientMessage = serde_json::from_str(&json).unwrap();
    match parsed {
        ClientMessage::PermissionResponse { decision, .. } => {
            assert_eq!(decision, Decision::Allow);
        }
        _ => panic!("Wrong message type"),
    }
}

#[tokio::test]
async fn test_session_config_serialization() {
    // Test: Verify session config serializes correctly
    let config = SessionConfig {
        permission_mode: PermissionMode::Manual,
        max_turns: Some(10),
        metadata: std::collections::HashMap::new(),
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("\"permission_mode\":\"manual\""));
    assert!(json.contains("\"max_turns\":10"));

    let parsed: SessionConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.permission_mode, PermissionMode::Manual);
    assert_eq!(parsed.max_turns, Some(10));
}

#[tokio::test]
async fn test_result_message_serialization() {
    // Test: Verify result message serializes correctly
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

    let parsed: ServerMessage = serde_json::from_str(&json).unwrap();
    match parsed {
        ServerMessage::Result { subtype, num_turns, .. } => {
            assert_eq!(subtype, ResultSubtype::Success);
            assert_eq!(num_turns, 3);
        }
        _ => panic!("Wrong message type"),
    }
}
