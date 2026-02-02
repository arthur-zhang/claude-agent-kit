# WebSocket Protocol Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 实现一个 WebSocket 代理服务器，将前端协议消息转换为 Claude Agent SDK 格式，并实现完整的消息转换、权限处理和流式消息支持。

**Architecture:**
- `protocol/types.rs` - 定义协议文档中的 26 种消息类型
- `protocol/converter.rs` - 实现 SDK 消息与协议消息的双向转换
- `session/state.rs` - 会话状态管理，包括等待权限响应的状态
- `session/handler.rs` - 消息路由和处理逻辑

**Tech Stack:** Rust, Axum WebSocket, tokio, serde, agent-sdk

---

## Task 1: Create protocol module structure

**Files:**
- Create: `websocket/src/protocol/mod.rs`
- Create: `websocket/src/protocol/types.rs`

**Step 1: Write the module structure**

Create `websocket/src/protocol/mod.rs`:

```rust
//! WebSocket protocol message types and conversion.

pub mod types;
pub mod converter;

pub use types::{ClientMessage, ServerMessage};
```

**Step 2: Add protocol module to websocket/src/lib.rs**

Add to `websocket/src/lib.rs`:

```rust
pub mod protocol;
```

**Step 3: Run cargo check to verify module structure**

Run: `cargo check -p websocket`
Expected: OK (no errors yet, just empty modules)

**Step 4: Commit**

```bash
git add websocket/src/protocol/mod.rs websocket/src/lib.rs
git commit -m "feat: add protocol module structure"
```

---

## Task 2: Define protocol message types - Part 1 (ClientMessage)

**Files:**
- Create: `websocket/src/protocol/types.rs`

**Step 1: Define shared types**

First, define the shared types used by both client and server messages. Create `websocket/src/protocol/types.rs`:

```rust
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
```

**Step 2: Define ClientMessage enum**

Add to `websocket/src/protocol/types.rs`:

```rust
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
```

**Step 3: Run cargo check**

Run: `cargo check -p websocket`
Expected: OK

**Step 4: Write basic tests for ClientMessage**

Add to `websocket/src/protocol/types.rs`:

```rust
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
```

**Step 5: Run tests**

Run: `cargo test -p websocket protocol::types::client_message_tests`
Expected: PASS

**Step 6: Commit**

```bash
git add websocket/src/protocol/types.rs
git commit -m "feat: define ClientMessage types with tests"
```

---

## Task 3: Define protocol message types - Part 2 (ServerMessage)

**Files:**
- Modify: `websocket/src/protocol/types.rs`

**Step 1: Define delta types for assistant messages**

Add to `websocket/src/protocol/types.rs`:

```rust
/// Delta content for assistant_message_delta messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
```

**Step 2: Define ServerMessage enum**

Add to `websocket/src/protocol/types.rs`:

```rust
/// Messages sent from server to client.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        /// Result subtype: "success", "error", "interrupted"
        subtype: String,
        /// Duration in milliseconds
        duration_ms: i64,
        /// API duration in milliseconds
        duration_api_ms: i64,
        /// Number of turns
        num_turns: i32,
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
        status: String,
    },

    /// Heartbeat - keep-alive message
    Heartbeat {
        /// Unique message ID
        id: String,
        /// Session ID
        session_id: String,
        /// Timestamp
        timestamp: i64,
    },
}
```

**Step 3: Run cargo check**

Run: `cargo check -p websocket`
Expected: OK

**Step 4: Write tests for ServerMessage**

Add to `websocket/src/protocol/types.rs`:

```rust
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
}
```

**Step 5: Run tests**

Run: `cargo test -p websocket protocol::types::server_message_tests`
Expected: PASS

**Step 6: Commit**

```bash
git add websocket/src/protocol/types.rs
git commit -m "feat: define ServerMessage types with tests"
```

---

## Task 4: Implement protocol/converter.rs - SDK to Protocol conversion

**Files:**
- Create: `websocket/src/protocol/converter.rs`

**Step 1: Write the failing test**

First, create a test to verify conversion from SDK messages to protocol messages:

```rust
//! Protocol message conversion.
//!
//! Converts between agent-sdk message types and WebSocket protocol message types.

use crate::protocol::types::*;
use claude_agent_sdk::{Message, ContentBlock, MessageContent};
use serde_json::json;

#[cfg(test)]
mod converter_tests {
    use super::*;

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
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p websocket protocol::converter::converter_tests`
Expected: FAIL with "module not found" or function not defined

**Step 3: Implement minimal converter**

Create `websocket/src/protocol/converter.rs`:

```rust
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
                        // Send as both delta and separate tool_use
                        result.push(ServerMessage::AssistantMessageDelta {
                            id: msg_id.clone(),
                            session_id: session_id.to_string(),
                            delta: Delta::ToolUse {
                                tool_use_id: id.clone(),
                                tool_name: name.clone(),
                                tool_input: input.clone(),
                            },
                        });
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
            vec![ServerMessage::Result {
                id: Uuid::new_v4().to_string(),
                session_id: session_id.to_string(),
                subtype: result.subtype.clone(),
                duration_ms: result.duration_ms,
                duration_api_ms: result.duration_api_ms,
                num_turns: result.num_turns,
                is_error: result.is_error,
                error: result.result.clone(),
                total_cost_usd: result.total_cost_usd,
            }]
        }
        Message::System(system) => {
            // System messages like init are handled internally
            // tracing::debug!("System message: subtype={}, data={:?}", system.subtype, system.data);
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
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p websocket protocol::converter::converter_tests`
Expected: PASS

**Step 5: Commit**

```bash
git add websocket/src/protocol/converter.rs
git commit -m "feat: implement SDK to protocol message converter"
```

---

## Task 5: Implement session/state.rs

**Files:**
- Create: `websocket/src/session/mod.rs`
- Create: `websocket/src/session/state.rs`

**Step 1: Create session module**

Create `websocket/src/session/mod.rs`:

```rust
//! Session management.

pub mod state;
pub mod handler;

pub use state::{SessionState, SessionStatus, PendingPermission};
```

**Step 2: Add session module to lib.rs**

Add to `websocket/src/lib.rs`:

```rust
pub mod session;
```

**Step 3: Write the failing test**

Create `websocket/src/session/state.rs` with tests first:

```rust
//! Session state management.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

/// Session state.
pub struct SessionState {
    pub session_id: String,
    pub config: SessionConfig,
    pub status: Arc<Mutex<SessionStatus>>,
    pub pending_permission: Arc<Mutex<Option<PendingPermission>>>,
    message_id_counter: AtomicU64,
}

impl SessionState {
    /// Create a new session state.
    pub fn new(session_id: String, config: SessionConfig) -> Self {
        Self {
            session_id,
            config,
            status: Arc::new(Mutex::new(SessionStatus::Idle)),
            pending_permission: Arc::new(Mutex::new(None)),
            message_id_counter: AtomicU64::new(0),
        }
    }

    /// Generate next message ID.
    pub fn next_message_id(&self) -> String {
        let id = self.message_id_counter.fetch_add(1, Ordering::SeqCst);
        format!("msg-{}", id)
    }

    /// Set session status.
    pub async fn set_status(&self, status: SessionStatus) {
        *self.status.lock().await = status;
    }

    /// Get current session status.
    pub async fn status(&self) -> SessionStatus {
        self.status.lock().await.clone()
    }
}

/// Session status.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionStatus {
    Idle,
    Thinking,
    ExecutingTool,
    WaitingPermission,
}

/// Pending permission request.
pub struct PendingPermission {
    pub request_id: String,
    pub tool_name: String,
    pub tool_input: serde_json::Value,
    pub response_tx: oneshot::Sender<PermissionDecision>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::types::SessionConfig;

    #[test]
    fn test_session_state_creation() {
        let config = SessionConfig {
            session_id: "test-session".to_string(),
            permission_mode: "manual".to_string(),
            max_turns: None,
            metadata: Default::default(),
        };
        let state = SessionState::new("test-session".to_string(), config);
        assert_eq!(state.session_id, "test-session");
    }

    #[test]
    fn test_message_id_generation() {
        let config = SessionConfig {
            session_id: "test-session".to_string(),
            permission_mode: "manual".to_string(),
            max_turns: None,
            metadata: Default::default(),
        };
        let state = SessionState::new("test-session".to_string(), config);
        assert_eq!(state.next_message_id(), "msg-0");
        assert_eq!(state.next_message_id(), "msg-1");
        assert_eq!(state.next_message_id(), "msg-2");
    }
}
```

**Step 4: Run cargo check**

Run: `cargo check -p websocket`
Expected: OK

**Step 5: Run tests**

Run: `cargo test -p websocket session::state::tests`
Expected: PASS

**Step 6: Commit**

```bash
git add websocket/src/session/
git commit -m "feat: implement session state management"
```

---

## Task 6: Implement session/handler.rs - Message routing

**Files:**
- Create: `websocket/src/session/handler.rs`
- Modify: `websocket/src/agent/session.rs` (refactor to use new handler)

**Step 1: Write handler tests**

Create `websocket/src/session/handler.rs`:

```rust
//! Session message handler.
//!
//! Routes messages between WebSocket and agent SDK.

use crate::protocol::types::*;
use crate::protocol::converter;
use crate::session::state::{SessionState, SessionStatus};
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
            send_session_info(ws_sender, &session_id, "active", config).await?;
        }
        ClientMessage::UserMessage { .. } => {
            // TODO: Forward to agent SDK
            state.set_status(SessionStatus::Thinking).await;
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
    status: &str,
    config: &HandlerConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let info_msg = ServerMessage::SessionInfo {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        status: status.to_string(),
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
```

**Step 2: Run cargo check**

Run: `cargo check -p websocket`
Expected: OK (some warnings about unused code is fine)

**Step 3: Run tests**

Run: `cargo test -p websocket session::handler::tests`
Expected: PASS

**Step 4: Commit**

```bash
git add websocket/src/session/handler.rs
git commit -m "feat: implement session message handler"
```

---

## Task 7: Integrate agent-sdk with handler

**Files:**
- Modify: `websocket/src/session/handler.rs`
- Modify: `websocket/src/agent/session.rs` (refactor)

**Step 1: Update handler to use agent SDK**

Modify `websocket/src/session/handler.rs` to integrate with the agent SDK. This is a significant change, so we'll do it in parts.

First, add the agent client field to the handler:

```rust
use claude_agent_sdk::ClaudeClient;

/// Handle a WebSocket session with agent SDK integration.
pub async fn handle_session_with_agent(
    websocket: WebSocket,
    session_id: String,
    config: SessionConfig,
    handler_config: HandlerConfig,
    client: Arc<Mutex<ClaudeClient>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(SessionState::new(session_id.clone(), config));
    let (mut ws_sender, mut ws_receiver) = websocket.split();

    // Get agent message stream
    let client_clone = Arc::clone(&client);
    let mut agent_guard = client_clone.lock().await;
    let agent_stream = agent_guard.receive_messages().await?;
    drop(agent_guard); // Release lock

    info!("Starting session handler for session {}", session_id);

    loop {
        tokio::select! {
            // Handle agent messages
            Some(msg_result) = agent_stream.next() => {
                match msg_result {
                    Ok(agent_msg) => {
                        let protocol_msgs = converter::sdk_to_protocol(&agent_msg, &session_id);
                        for proto_msg in protocol_msgs {
                            if let Ok(json) = serde_json::to_string(&proto_msg) {
                                if tokio::time::timeout(
                                    std::time::Duration::from_secs(handler_config.send_timeout_secs),
                                    ws_sender.send(WsMessage::Text(json)),
                                ).await.is_err() {
                                    warn!("Timeout sending to WebSocket");
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Agent stream error: {}", e);
                        send_error(&mut ws_sender, &session_id, None, &format!("Agent error: {}", e), &handler_config).await;
                        break;
                    }
                }
            }

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

                        if let Err(e) = handle_client_message_with_agent(client_msg, &state, &client, &mut ws_sender, &handler_config).await {
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

/// Handle a client message with agent SDK.
async fn handle_client_message_with_agent(
    msg: ClientMessage,
    state: &Arc<SessionState>,
    client: &Arc<Mutex<ClaudeClient>>,
    ws_sender: &mut futures::stream::SplitSink<WebSocket, WsMessage>,
    config: &HandlerConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    match msg {
        ClientMessage::UserMessage { content, parent_tool_use_id, session_id, .. } => {
            state.set_status(SessionStatus::Thinking).await;
            let mut client_guard = client.lock().await;
            // TODO: Implement query_string
        }
        ClientMessage::PermissionResponse { .. } => {
            // TODO: Handle via pending_permission
        }
        ClientMessage::SessionStart { session_id, .. } => {
            send_session_info(ws_sender, &session_id, "active", config).await?;
        }
        _ => {}
    }
    Ok(())
}
```

**Step 2: Run cargo check**

Run: `cargo check -p websocket`
Expected: May have errors due to incomplete ClaudeClient API

**Step 3: Fix compilation errors based on actual ClaudeClient API**

Check the actual ClaudeClient API and adjust accordingly.

**Step 4: Commit**

```bash
git add websocket/src/session/handler.rs
git commit -m "feat: integrate agent SDK with session handler"
```

---

## Task 8: Update WebSocket route to use new handler

**Files:**
- Modify: `websocket/src/agent/session.rs` or `websocket/src/main.rs`

**Step 1: Update the WebSocket upgrade handler**

Locate the WebSocket upgrade handler in your codebase and update it to use the new `handle_session_with_agent` function.

**Step 2: Test the integration**

Run: `cargo run`
Expected: Server starts without errors

**Step 3: Commit**

```bash
git add websocket/src/
git commit -m "feat: update WebSocket route to use new handler"
```

---

## Task 9: Add integration tests

**Files:**
- Create: `websocket/tests/integration_test.rs`

**Step 1: Write basic integration test**

Create integration tests for the WebSocket protocol:

```rust
//! WebSocket protocol integration tests.

use axum::extract::ws::{WebSocket, Message};
use futures::{SinkExt, StreamExt};

#[tokio::test]
async fn test_session_start_flow() {
    // Test: session_start → session_info
}

#[tokio::test]
async fn test_user_message_flow() {
    // Test: user_message → assistant_message_start → deltas → assistant_message_complete
}

#[tokio::test]
async fn test_permission_request_flow() {
    // Test: tool_use → permission_request → permission_response → tool_result
}
```

**Step 2: Run integration tests**

Run: `cargo test -p websocket --test integration_test`
Expected: Tests pass (some may be TODO)

**Step 3: Commit**

```bash
git add websocket/tests/
git commit -m "test: add integration tests"
```

---

## Task 10: Final cleanup and documentation

**Files:**
- Modify: `websocket/README.md` or create documentation

**Step 1: Update documentation**

Document the protocol usage and examples.

**Step 2: Run all tests**

Run: `cargo test -p websocket`
Expected: All tests pass

**Step 3: Run clippy**

Run: `cargo clippy -p websocket`
Expected: No warnings

**Step 4: Format code**

Run: `cargo fmt -p websocket`

**Step 5: Final commit**

```bash
git add .
git commit -m "docs: update protocol documentation"
```

---

## Summary

This plan implements the WebSocket protocol in 10 tasks:

1. **Module structure** - Create protocol module
2. **ClientMessage types** - Define client-to-server messages
3. **ServerMessage types** - Define server-to-client messages
4. **Converter** - SDK ↔ Protocol message conversion
5. **Session state** - State management
6. **Session handler** - Message routing
7. **Agent integration** - Connect to Claude Agent SDK
8. **Route update** - Use new handler in WebSocket route
9. **Integration tests** - End-to-end tests
10. **Cleanup** - Documentation and finalization
