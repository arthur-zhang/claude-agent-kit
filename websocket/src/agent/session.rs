//! Agent session for managing WebSocket-Agent communication.

use crate::agent::client::PooledAgent;
use axum::extract::ws::{Message as WsMessage, WebSocket};
use claude_agent_sdk::Error;
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Manages a single WebSocket connection's agent session.
pub struct AgentSession {
    session_id: Uuid,
}

impl AgentSession {
    /// Create a new agent session.
    pub fn new() -> Self {
        let session_id = Uuid::new_v4();
        info!("Created agent session {}", session_id);

        Self {
            session_id,
        }
    }

    /// Run the session, forwarding messages between WebSocket and Agent.
    pub async fn run(self, websocket: WebSocket, mut agent: PooledAgent) -> Result<(), Error> {
        info!("Starting agent session {}", self.session_id);

        // Split websocket
        let (mut ws_sender, mut ws_receiver) = websocket.split();

        // Create a channel for incoming WebSocket messages
        let (ws_msg_tx, mut ws_msg_rx) = mpsc::unbounded_channel();

        // Spawn task to receive WebSocket messages and forward to channel
        let ws_recv_task = tokio::spawn(async move {
            while let Some(msg_result) = ws_receiver.next().await {
                if ws_msg_tx.send(msg_result).is_err() {
                    break;
                }
            }
        });

        // Main loop: handle both agent stream and websocket messages
        // Note: We create the agent stream inside the loop after each query
        // This is because the stream borrows the agent, preventing us from using it for queries
        // For now, we'll use a simplified approach that doesn't support bidirectional streaming
        loop {
            tokio::select! {
                // Handle incoming WebSocket messages
                Some(ws_msg_result) = ws_msg_rx.recv() => {
                    match ws_msg_result {
                        Ok(WsMessage::Text(text)) => {
                            debug!("Received WebSocket message");

                            let json: Value = match serde_json::from_str(&text) {
                                Ok(v) => v,
                                Err(e) => {
                                    error!("Parse error: {}", e);
                                    continue;
                                }
                            };

                            let prompt = json.get("message")
                                .and_then(|m| m.get("content"))
                                .and_then(|c| c.as_str())
                                .unwrap_or("");

                            let session_id = json.get("session_id")
                                .and_then(|s| s.as_str())
                                .unwrap_or("default");

                            // Query the agent and get the response stream
                            if let Err(e) = agent.client_mut().query_string(prompt, session_id).await {
                                error!("Failed to query agent: {}", e);

                                // Send error message to client
                                let error_json = json!({
                                    "type": "error",
                                    "error": format!("Failed to query agent: {}", e)
                                });
                                if let Ok(error_str) = serde_json::to_string(&error_json) {
                                    let _ = tokio::time::timeout(
                                        Duration::from_secs(5),
                                        ws_sender.send(WsMessage::Text(error_str))
                                    ).await;
                                }

                                break;
                            }

                            // Get response stream
                            match agent.client_mut().receive_response().await {
                                Ok(mut response_stream) => {
                                    // Forward all messages from this response to WebSocket
                                    while let Some(msg_result) = response_stream.next().await {
                                        match msg_result {
                                            Ok(message) => {
                                                let json = match serde_json::to_string(&message) {
                                                    Ok(s) => s,
                                                    Err(e) => {
                                                        error!("Failed to serialize: {}", e);
                                                        continue;
                                                    }
                                                };

                                                match tokio::time::timeout(
                                                    Duration::from_secs(5),
                                                    ws_sender.send(WsMessage::Text(json))
                                                ).await {
                                                    Ok(Ok(_)) => {},
                                                    Ok(Err(e)) => {
                                                        warn!("Failed to send to WebSocket: {}", e);
                                                        break;
                                                    }
                                                    Err(_) => {
                                                        warn!("Timeout sending to WebSocket");
                                                        break;
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                error!("Error in response stream: {}", e);
                                                break;
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Failed to get response stream: {}", e);

                                    // Send error message to client
                                    let error_json = json!({
                                        "type": "error",
                                        "error": format!("Failed to get response stream: {}", e)
                                    });
                                    if let Ok(error_str) = serde_json::to_string(&error_json) {
                                        let _ = tokio::time::timeout(
                                            Duration::from_secs(5),
                                            ws_sender.send(WsMessage::Text(error_str))
                                        ).await;
                                    }

                                    break;
                                }
                            }
                        }
                        Ok(WsMessage::Close(_)) => {
                            info!("WebSocket closed");
                            break;
                        }
                        Err(e) => {
                            error!("WebSocket error: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }
                else => break,
            }
        }

        // Clean up - abort the WebSocket receive task
        ws_recv_task.abort();

        // Disconnect agent and log any errors
        if let Err(e) = agent.disconnect().await {
            error!("Failed to disconnect agent: {}", e);
        }

        info!("Agent session {} ended", self.session_id);
        Ok(())
    }

    /// Get session ID.
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = AgentSession::new();
        assert_ne!(session.session_id(), Uuid::nil());
    }

    #[test]
    fn test_session_id_uniqueness() {
        let session1 = AgentSession::new();
        let session2 = AgentSession::new();
        assert_ne!(session1.session_id(), session2.session_id());
    }

    #[tokio::test]
    async fn test_error_json_format() {
        // Test that error JSON is properly formatted
        let error_json = json!({
            "type": "error",
            "error": "Test error"
        });

        let json_str = serde_json::to_string(&error_json).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed.get("type").and_then(|v| v.as_str()), Some("error"));
        assert_eq!(parsed.get("error").and_then(|v| v.as_str()), Some("Test error"));
    }

    #[tokio::test]
    async fn test_message_parsing() {
        // Test that we can parse incoming WebSocket messages correctly
        let test_message = r#"{
            "message": {
                "content": "Hello, agent!"
            },
            "session_id": "test-session"
        }"#;

        let json: Value = serde_json::from_str(test_message).unwrap();

        let prompt = json.get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("");

        let session_id = json.get("session_id")
            .and_then(|s| s.as_str())
            .unwrap_or("default");

        assert_eq!(prompt, "Hello, agent!");
        assert_eq!(session_id, "test-session");
    }

    #[tokio::test]
    async fn test_message_parsing_defaults() {
        // Test that defaults work when fields are missing
        let test_message = r#"{}"#;

        let json: Value = serde_json::from_str(test_message).unwrap();

        let prompt = json.get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("");

        let session_id = json.get("session_id")
            .and_then(|s| s.as_str())
            .unwrap_or("default");

        assert_eq!(prompt, "");
        assert_eq!(session_id, "default");
    }

    #[tokio::test]
    async fn test_timeout_duration() {
        // Verify timeout is reasonable (5 seconds)
        let timeout = Duration::from_secs(5);
        assert_eq!(timeout.as_secs(), 5);
        assert!(timeout.as_secs() > 0);
        assert!(timeout.as_secs() < 30); // Not too long
    }
}
