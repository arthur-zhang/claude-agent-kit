//! Session message handler.
//!
//! Routes messages between WebSocket and agent SDK.

use crate::protocol::types::*;
use crate::protocol::converter;
use crate::session::state::{SessionState, AgentState};
use axum::extract::ws::{Message as WsMessage, WebSocket};
use claude_agent_sdk::Message;
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// Session handler configuration.
pub struct HandlerConfig {
    /// Timeout for WebSocket sends (seconds)
    pub send_timeout_secs: u64,
}

impl Default for HandlerConfig {
    fn default() -> Self {
        Self {
            send_timeout_secs: 5,
        }
    }
}

/// Handle a WebSocket session.
pub async fn handle_session(
    websocket: WebSocket,
    session_id: String,
    config: SessionConfig,
    handler_config: HandlerConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(SessionState::new(session_id.clone(), config));
    let (mut ws_sender, mut ws_receiver) = websocket.split();

    // TODO: Connect to agent SDK
    // For now, handle basic messages

    info!("Starting session handler for session {}", session_id);

    loop {
        tokio::select! {
            // Handle WebSocket messages from client
            Some(ws_msg_result) = ws_receiver.next() => {
                match ws_msg_result {
                    Ok(WsMessage::Text(text)) => {
                        debug!("Received WebSocket message: {}", text);

                        let client_msg: ClientMessage = match serde_json::from_str(&text) {
                            Ok(msg) => msg,
                            Err(e) => {
                                error!("Failed to parse client message: {}", e);
                                send_error(&mut ws_sender, &session_id, None, &format!("Invalid message: {}", e), &handler_config).await;
                                continue;
                            }
                        };

                        if let Err(e) = handle_client_message(client_msg, &state, &mut ws_sender, &handler_config).await {
                            error!("Error handling client message: {}", e);
                            break;
                        }
                    }
                    Ok(WsMessage::Close(_)) => {
                        info!("WebSocket closed by client");
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

    info!("Session handler ending for session {}", session_id);
    Ok(())
}

/// Handle a client message.
async fn handle_client_message(
    msg: ClientMessage,
    state: &Arc<SessionState>,
    ws_sender: &mut futures::stream::SplitSink<WebSocket, WsMessage>,
    config: &HandlerConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    match msg {
        ClientMessage::SessionStart { session_id, config: session_config, .. } => {
            info!("Session start: {}", session_id);
            send_session_info(ws_sender, &session_id, SessionStatus::Active, config).await?;
        }
        ClientMessage::UserMessage { .. } => {
            // TODO: Forward to agent SDK
            state.set_status(AgentState::Thinking).await;
        }
        ClientMessage::PermissionResponse { request_id, decision, .. } => {
            debug!("Permission response for request {}: {:?}", request_id, decision);
            // TODO: Handle permission response via pending_permission
        }
        ClientMessage::SessionEnd { .. } => {
            info!("Session end requested");
        }
        _ => {
            debug!("Unhandled message type");
        }
    }
    Ok(())
}

/// Send an error message to the client.
async fn send_error(
    ws_sender: &mut futures::stream::SplitSink<WebSocket, WsMessage>,
    session_id: &str,
    request_id: Option<String>,
    message: &str,
    config: &HandlerConfig,
) {
    let error_msg = ServerMessage::Error {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        request_id,
        message: message.to_string(),
    };

    if let Ok(json) = serde_json::to_string(&error_msg) {
        let _ = tokio::time::timeout(
            std::time::Duration::from_secs(config.send_timeout_secs),
            ws_sender.send(WsMessage::Text(json)),
        ).await;
    }
}

/// Send session info to the client.
async fn send_session_info(
    ws_sender: &mut futures::stream::SplitSink<WebSocket, WsMessage>,
    session_id: &str,
    status: SessionStatus,
    config: &HandlerConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let info_msg = ServerMessage::SessionInfo {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        status,
    };

    let json = serde_json::to_string(&info_msg)?;
    tokio::time::timeout(
        std::time::Duration::from_secs(config.send_timeout_secs),
        ws_sender.send(WsMessage::Text(json)),
    ).await??;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_config_default() {
        let config = HandlerConfig::default();
        assert_eq!(config.send_timeout_secs, 5);
    }
}
