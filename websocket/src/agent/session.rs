//! Agent session for managing WebSocket-Agent communication.

use crate::agent::client::PooledAgent;
use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures::{SinkExt, StreamExt};
use serde_json::Value;
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
    pub async fn run(self, websocket: WebSocket, mut agent: PooledAgent) -> Result<(), String> {
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

                                                if ws_sender.send(WsMessage::Text(json)).await.is_err() {
                                                    warn!("Failed to send to WebSocket");
                                                    break;
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

        // Clean up
        ws_recv_task.abort();
        let _ = agent.disconnect().await;

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
}
