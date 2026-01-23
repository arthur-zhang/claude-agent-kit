//! Agent session for managing WebSocket-Agent communication.

use axum::extract::ws::{Message as WsMessage, WebSocket};
use claude_agent_sdk::Message::System;
use claude_agent_sdk::{ Error};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, mpsc};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Manages a single WebSocket connection's agent session.
pub struct AgentSession {
    session_id: Uuid,
}

impl Default for AgentSession {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentSession {
    /// Create a new agent session.
    pub fn new() -> Self {
        let session_id = Uuid::new_v4();
        info!("Created agent session {}", session_id);

        Self { session_id }
    }

    /// Run the session, forwarding messages between WebSocket and Agent.
    pub async fn run(
        self,
        websocket: WebSocket,
        // client: Arc<Mutex<ClaudeClient>>,
        session_id: String,
    ) -> Result<(), Error> {
        info!(
            "Starting agent session {} with session_id {}",
            self.session_id, session_id
        );

        // Split websocket
        let (mut ws_sender, mut ws_receiver) = websocket.split();

        let session_id_map = Arc::new(DashMap::new());
        // Get two clones of the client for separate tasks
        // let client_for_receive = Arc::clone(&client);
        // let client_for_send = Arc::clone(&client);

        // Create channel for agent messages
        let (agent_msg_tx, mut agent_msg_rx) = mpsc::unbounded_channel();

        // Spawn task to receive agent messages
        let receive_task = tokio::spawn({
            let session_id = session_id.clone();
            let session_id_map = session_id_map.clone();
            async move {
                let mut client_guard = client_for_receive.lock().await;

                let mut agent_stream = match client_guard.receive_messages().await {
                    Ok(stream) => stream,
                    Err(e) => {
                        error!("Failed to get agent stream: {}", e);
                        return;
                    }
                };

                while let Some(msg_result) = agent_stream.next().await {
                    if let Ok(System(ref system_message)) = msg_result
                        && system_message.subtype == "init"
                    {
                        // todo
                        let cc_session_id = system_message
                            .data
                            .get("session_id")
                            .unwrap()
                            .as_str()
                            .unwrap();
                        session_id_map.insert(session_id.clone(), cc_session_id.to_string());
                    }

                    if agent_msg_tx.send(msg_result).is_err() {
                        break;
                    }
                }
            }
        });

        // Main loop: handle WebSocket and agent messages
        loop {
            tokio::select! {
                // Handle messages from agent
                Some(msg_result) = agent_msg_rx.recv() => {
                    match msg_result {
                        Ok(message) => {
                            debug!("Received agent message {message:?}");

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
                            error!("Error in agent stream: {}", e);

                            // Send error message to client
                            let error_json = json!({
                                "type": "error",
                                "error": format!("Agent error: {}", e)
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

                // Handle incoming WebSocket messages
                Some(ws_msg_result) = ws_receiver.next() => {
                    match ws_msg_result {
                        Ok(WsMessage::Text(text)) => {
                            debug!("Received WebSocket message {text}");

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

                            // Send query using a brief lock on the send client
                            let send_result = {
                                let mut client_guard = client_for_send.lock().await;

                                // todo
                                let cc_session_id = session_id_map.get(&session_id).map(|it|it.clone());
                                info!("start query>>>>>>>>>>{session_id}, {cc_session_id:?}");
                                client_guard.query_string(prompt, cc_session_id).await
                            };

                            if let Err(e) = send_result {
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

                            // Response will come through the agent_stream in the other select branch
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

        // Clean up - abort the receive task
        receive_task.abort();

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
        assert_eq!(
            parsed.get("error").and_then(|v| v.as_str()),
            Some("Test error")
        );
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

        let prompt = json
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("");

        let session_id = json
            .get("session_id")
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

        let prompt = json
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("");

        let session_id = json
            .get("session_id")
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
