use crate::connection::ConnectionManager;
use crate::protocol::common::{Decision, PermissionContext, RiskLevel};
use crate::protocol::events::{
    AgentEvent, ClientMessage, ControlRequestMessage, ControlSubtype, SessionInitData,
    SlashCommandInfo, WorkspaceInitResponse,
};
use crate::protocol::types::{PermissionMode, SessionConfig};
use crate::session::query::{
    PermissionHandler, PermissionRequest, PermissionResponse, QueryError, QueryOptions, Session,
};
use axum::extract::ws::Message as WsMessage;
use axum::{
    extract::{Query, State, WebSocketUpgrade, ws::WebSocket},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use claude_agent_sdk::types::ProtocolMessage;
use futures::{pin_mut, SinkExt, StreamExt, stream::SplitStream};
use rust_embed::RustEmbed;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender;
use tokio::sync::{oneshot, Mutex};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

// Embed static files at compile time
#[derive(RustEmbed)]
#[folder = "static/"]
struct StaticAssets;

#[derive(Clone)]
pub struct AppState {
    pub connection_manager: ConnectionManager,
    // pub session_manager: SessionManager,
}

#[derive(Deserialize)]
pub struct WsQuery {
    session_id: Option<String>,
}

pub async fn create_router() -> Result<Router, Box<dyn std::error::Error>> {
    let state = AppState {
        connection_manager: ConnectionManager::new(),
        // session_manager: SessionManager::new(),
    };

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/", get(serve_index))
        .route("/*path", get(serve_static))
        .with_state(state);

    Ok(app)
}

// Serve index.html for root path
async fn serve_index() -> impl IntoResponse {
    serve_static_file("index.html".to_string())
}

// Serve embedded static files
async fn serve_static(axum::extract::Path(path): axum::extract::Path<String>) -> impl IntoResponse {
    serve_static_file(path)
}

fn serve_static_file(path: String) -> impl IntoResponse {
    match StaticAssets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(&path).first_or_octet_stream();
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime.as_ref())],
                content.data,
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(state): State<AppState>,
) -> Response {
    let session_id = query
        .session_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    ws.on_upgrade(move |socket| handle_socket(socket, state, session_id))
}

async fn handle_socket(socket: WebSocket, _state: AppState, session_id: String) {
    info!("New WebSocket connection for session {}", session_id);

    // Split socket
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Create channel for outgoing messages
    let (tx, mut rx) = tokio::sync::mpsc::channel::<WsMessage>(100);

    // Spawn writer task
    let session_id_clone = session_id.clone();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = ws_sender.send(msg).await {
                error!("Failed to send WebSocket message for session {}: {}", session_id_clone, e);
                break;
            }
        }
        // Ensure we close the WebSocket when the channel is dropped
        let _ = ws_sender.close().await;
    });

    let ws_sender = tx;

    // 权限请求处理 - 使用简单的 Option 因为一次只有一个待处理的权限请求
    let (perm_tx, mut perm_rx) = tokio::sync::mpsc::channel::<(PermissionRequest, oneshot::Sender<PermissionResponse>)>(10);
    let pending_permission: Arc<Mutex<Option<oneshot::Sender<PermissionResponse>>>> = Arc::new(Mutex::new(None));

    // Phase 1: Wait for UserSessionInit message
    let init_result = wait_for_init_message(&mut ws_receiver, &session_id).await;

    let init_data = match init_result {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to receive init message for session {}: {}", session_id, e);
            send_error_and_close(ws_sender.clone(), &session_id, e).await;
            return;
        }
    };

    // Save workspace_init info for response format
    let is_workspace_init = init_data.is_workspace_init;
    let _workspace_init_request_id = init_data.request_id.clone();

    let is_resume = init_data.resume.is_some();
    let resume_session_id = init_data.resume.clone();
    info!("Session {} initialized with cwd: {}, resume: {:?}", session_id, init_data.cwd, resume_session_id);

    // Create session config from init data
    let config = SessionConfig {
        permission_mode: convert_permission_mode(init_data.permission_mode.as_ref()),
        max_turns: init_data.max_turns,
        max_thinking_tokens: init_data.max_thinking_tokens,
        dangerously_skip_permissions: init_data.dangerously_skip_permissions,
        metadata: Default::default(),
    };

    // Save disallowed_tools for use in subsequent queries
    let disallowed_tools = init_data.disallowed_tools.clone();

    // Create permission handler
    let perm_tx_clone = perm_tx.clone();
    let permission_handler: PermissionHandler = Arc::new(move |req: PermissionRequest| {
        let tx = perm_tx_clone.clone();
        Box::pin(async move {
            let (resp_tx, resp_rx) = oneshot::channel();

            // 发送权限请求到主循环
            if tx.send((req, resp_tx)).await.is_err() {
                return PermissionResponse::Deny;
            }

            // 等待响应，5分钟超时
            match tokio::time::timeout(std::time::Duration::from_secs(300), resp_rx).await {
                Ok(Ok(response)) => response,
                Ok(Err(_)) => PermissionResponse::Deny, // channel closed
                Err(_) => PermissionResponse::Deny, // timeout
            }
        })
    });

    // Build QueryOptions from init data
    let query_options = QueryOptions {
        permission_mode: config.permission_mode.clone(),
        permission_handler: Some(permission_handler.clone()),
        max_turns: config.max_turns,
        env: None,
        disallowed_tools: disallowed_tools.clone(),
    };

    // Phase 2: Create Session using Session::new() or Session::resume()
    let cwd = PathBuf::from(&init_data.cwd);
    let mut session = if let Some(ref resume_id) = resume_session_id {
        match Session::resume(
            session_id.clone(),
            resume_id.clone(),
            cwd,
            config.clone(),
            &query_options,
        ).await {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to resume session {}: {}", session_id, e);
                send_init_error(&ws_sender, is_workspace_init, &session_id, &e.to_string()).await;
                return;
            }
        }
    } else {
        match Session::new(
            session_id.clone(),
            cwd,
            config.clone(),
            &query_options,
        ).await {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to create session {}: {}", session_id, e);
                send_init_error(&ws_sender, is_workspace_init, &session_id, &e.to_string()).await;
                return;
            }
        }
    };

    // Get process handle for cleanup
    let mut process_handle = {
        let mut client_guard = session.client().lock().await;
        client_guard.process_handle()
    };

    // For resume sessions, we don't wait for System(init) - just use the resume_id directly
    // For new sessions, wait for System(init) message to get session init data
    let (actual_session_id, session_init_data) = if is_resume {
        // Resume case: use the resume_id as the session_id
        let resume_id = resume_session_id.clone().unwrap_or_else(|| session_id.clone());
        info!("Resume session: using resume_id {} as session_id", resume_id);

        // Update session with the resume_id
        session.set_session_id(resume_id.clone());

        // Return minimal init data for resume - the session already exists
        (resume_id, SessionInitData::default())
    } else {
        // New session case: wait for System(init) message
        match wait_for_session_init(&session).await {
            Ok((actual_session_id, session_init_data)) => {
                // Update session with actual session_id from SDK
                if actual_session_id != session.session_id() {
                    session.set_session_id(actual_session_id.clone());
                }
                (actual_session_id, session_init_data)
            }
            Err(e) => {
                error!("Session init failed for {}: {}", session_id, e);
                send_init_error(&ws_sender, is_workspace_init, &session_id, &e).await;
                if let Some(ref mut handle) = process_handle {
                    let _ = handle.kill().await;
                }
                return;
            }
        }
    };

    // Send response in appropriate format based on request type
    if is_workspace_init {
        // Send workspace_init_output format
        let response = WorkspaceInitResponse {
            id: actual_session_id.clone(),
            msg_type: "workspace_init_output".to_string(),
            agent_type: "claude".to_string(),
            slash_commands: if session_init_data.slash_commands.is_empty() {
                None
            } else {
                Some(session_init_data.slash_commands.iter().map(|s| SlashCommandInfo {
                    name: s.clone(),
                    description: None,
                }).collect())
            },
            mcp_servers: if session_init_data.mcp_servers.is_empty() {
                None
            } else {
                Some(session_init_data.mcp_servers.clone())
            },
            tools: if session_init_data.tools.is_empty() {
                None
            } else {
                Some(session_init_data.tools.clone())
            },
            agents: if session_init_data.agents.is_empty() {
                None
            } else {
                Some(session_init_data.agents.clone())
            },
            skills: if session_init_data.skills.is_empty() {
                None
            } else {
                Some(session_init_data.skills.clone())
            },
            plugins: if session_init_data.plugins.is_empty() {
                None
            } else {
                Some(session_init_data.plugins.iter().map(|p| p.name.clone()).collect())
            },
            model: session_init_data.model.clone(),
            cwd: session_init_data.cwd.clone(),
            claude_code_version: session_init_data.claude_code_version.clone(),
            error: None,
        };
        if let Ok(json) = serde_json::to_string(&response) {
            if let Err(e) = ws_sender.send(WsMessage::Text(json)).await {
                error!("Failed to send workspace_init_output for session {}: {}", session_id, e);
                return;
            }
        }
    } else {
        // Send session_init format (legacy)
        let init_response = AgentEvent::SessionInit {
            success: true,
            session_id: actual_session_id,
            error: None,
            data: session_init_data,
        };
        if let Ok(json) = serde_json::to_string(&init_response) {
            if let Err(e) = ws_sender.send(WsMessage::Text(json)).await {
                error!("Failed to send init response for session {}: {}", session_id, e);
                return;
            }
        }
    }

    // Phase 3: Message loop - process UserMessage with session.query()
    let mut current_cancel: Option<CancellationToken> = None;

    loop {
        tokio::select! {
            // 处理 WebSocket 消息
            msg = ws_receiver.next() => {
                match msg {
                    Some(Ok(WsMessage::Text(text))) => {
                        if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                            match client_msg {
                                ClientMessage::UserMessage { content, .. } => {
                                    let result = process_query_stream(
                                        &session,
                                        content,
                                        &config,
                                        permission_handler.clone(),
                                        &disallowed_tools,
                                        &ws_sender,
                                        &mut ws_receiver,
                                        &mut perm_rx,
                                        &pending_permission,
                                        &mut current_cancel,
                                    ).await;

                                    if matches!(result, QueryStreamResult::WebSocketClosed) {
                                        break;
                                    }
                                }
                                ClientMessage::Query { prompt, .. } => {
                                    let result = process_query_stream(
                                        &session,
                                        prompt,
                                        &config,
                                        permission_handler.clone(),
                                        &disallowed_tools,
                                        &ws_sender,
                                        &mut ws_receiver,
                                        &mut perm_rx,
                                        &pending_permission,
                                        &mut current_cancel,
                                    ).await;

                                    if matches!(result, QueryStreamResult::WebSocketClosed) {
                                        break;
                                    }
                                }
                                ClientMessage::PermissionResponse { id: _, decision, .. } => {
                                    // 处理权限响应
                                    let mut pending = pending_permission.lock().await;
                                    if let Some(sender) = pending.take() {
                                        let response = match decision {
                                            Decision::Allow => PermissionResponse::Allow,
                                            Decision::Deny => PermissionResponse::Deny,
                                            Decision::AllowAlways => PermissionResponse::AllowAlways,
                                        };
                                        let _ = sender.send(response);
                                    }
                                }
                                ClientMessage::ControlRequest { subtype: ControlSubtype::Interrupt, .. } => {
                                    if let Some(token) = current_cancel.take() {
                                        token.cancel();
                                    }
                                }
                                ClientMessage::Cancel { .. } => {
                                    if let Some(token) = current_cancel.take() {
                                        token.cancel();
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) | None => break,
                    _ => {}
                }
            }

            // 处理权限请求 (在没有活跃 query 时)
            Some((req, resp_tx)) = perm_rx.recv() => {
                // 存储响应 channel
                {
                    let mut pending = pending_permission.lock().await;
                    *pending = Some(resp_tx);
                }

                // 发送权限请求事件到前端
                let msg = ControlRequestMessage {
                    msg_type: "permission_request".to_string(),
                    id: session.session_id().to_string(),
                    agent_type: "claude".to_string(),
                    tool_name: req.tool_name,
                    tool_use_id: req.tool_use_id,
                    input: req.input,
                    context: PermissionContext {
                        description: "Tool permission request".to_string(),
                        risk_level: RiskLevel::Medium,
                    },
                };
                send_control_request(&ws_sender, msg).await;
            }
        }
    }

    // Cleanup: terminate Claude Code process when WebSocket disconnects
    if let Some(ref mut handle) = process_handle {
        info!("Terminating Claude Code process for session {}", session_id);
        if let Err(e) = handle.kill().await {
            warn!("Failed to kill Claude Code process for session {}: {}", session_id, e);
        } else {
            info!("Claude Code process terminated for session {}", session_id);
        }
    }

    info!("WebSocket connection closed for session {}", session_id);
}

/// User session initialization data
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct UserSessionInitData {
    /// Request ID (for workspace_init format)
    request_id: Option<String>,
    /// Whether this is a workspace_init request (vs user_session_init)
    is_workspace_init: bool,
    cwd: String,
    model: Option<String>,
    permission_mode: Option<crate::protocol::events::PermissionMode>,
    max_turns: Option<i32>,
    max_budget_usd: Option<f64>,
    user: Option<String>,
    disallowed_tools: Option<Vec<String>>,
    max_thinking_tokens: Option<i32>,
    /// Resume a previous session by its session ID
    resume: Option<String>,
    /// Allow bypassing permission checks (required for bypassPermissions mode)
    dangerously_skip_permissions: Option<bool>,
}

/// Session initialization errors
#[derive(Debug)]
#[allow(dead_code)]
enum SessionError {
    InitTimeout,
    UnexpectedMessage(String),
    ConnectionClosed,
    WebSocketError(String),
    ClientInitFailed(String),
    ParseError(String),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionError::InitTimeout => write!(f, "Timeout waiting for UserSessionInit message"),
            SessionError::UnexpectedMessage(msg) => write!(f, "Expected UserSessionInit, got: {}", msg),
            SessionError::ConnectionClosed => write!(f, "Connection closed before initialization"),
            SessionError::WebSocketError(e) => write!(f, "WebSocket error: {}", e),
            SessionError::ClientInitFailed(e) => write!(f, "Failed to initialize Claude client: {}", e),
            SessionError::ParseError(e) => write!(f, "Failed to parse message: {}", e),
        }
    }
}

impl std::error::Error for SessionError {}

/// Wait for UserSessionInit or WorkspaceInit message from client
async fn wait_for_init_message(
    ws_receiver: &mut futures::stream::SplitStream<WebSocket>,
    _session_id: &str,
) -> Result<UserSessionInitData, SessionError> {
    let timeout_duration = Duration::from_secs(30);
    let deadline = Instant::now() + timeout_duration;

    loop {
        let now = Instant::now();
        if now >= deadline {
            return Err(SessionError::InitTimeout);
        }

        let remaining = deadline.saturating_duration_since(now);

        match tokio::time::timeout(remaining, ws_receiver.next()).await {
            Ok(Some(Ok(msg))) => {
                // Parse WebSocket message
                let text = match msg {
                    WsMessage::Text(t) => t,
                    WsMessage::Binary(b) => String::from_utf8(b.to_vec())
                        .map_err(|e| SessionError::ParseError(e.to_string()))?,
                    WsMessage::Close(_) => return Err(SessionError::ConnectionClosed),
                    // Ignore ping/pong/other non-text frames while waiting for init
                    _ => continue,
                };

                // Parse JSON to ClientMessage
                let client_msg: ClientMessage = serde_json::from_str(&text)
                    .map_err(|e| SessionError::ParseError(e.to_string()))?;

                // Check if it's UserSessionInit or WorkspaceInit
                match client_msg {
                    ClientMessage::UserSessionInit {
                        cwd,
                        model,
                        permission_mode,
                        max_turns,
                        max_budget_usd,
                        user,
                        disallowed_tools,
                        max_thinking_tokens,
                        resume,
                        dangerously_skip_permissions,
                    } => {
                        // Validate cwd
                        let cwd_path = PathBuf::from(&cwd);
                        if !cwd_path.exists() {
                            warn!("Working directory does not exist: {}", cwd);
                        }

                        return Ok(UserSessionInitData {
                            request_id: None,
                            is_workspace_init: false,
                            cwd,
                            model,
                            permission_mode,
                            max_turns,
                            max_budget_usd,
                            user,
                            disallowed_tools,
                            max_thinking_tokens,
                            resume,
                            dangerously_skip_permissions,
                        });
                    }
                    ClientMessage::WorkspaceInit { id: _, agent_type: _, options } => {
                        // Validate cwd
                        let cwd_path = PathBuf::from(&options.cwd);
                        if !cwd_path.exists() {
                            warn!("Working directory does not exist: {}", options.cwd);
                        }

                        return Ok(UserSessionInitData {
                            request_id: None,
                            is_workspace_init: true,
                            cwd: options.cwd,
                            model: options.model,
                            permission_mode: options.permission_mode,
                            max_turns: None,
                            max_budget_usd: None,
                            user: None,
                            disallowed_tools: options.disallowed_tools,
                            max_thinking_tokens: options.max_thinking_tokens,
                            resume: options.resume,
                            dangerously_skip_permissions: options.dangerously_skip_permissions,
                        });
                    }
                    other => {
                        let msg_type = format!("{:?}", other)
                            .split('{')
                            .next()
                            .unwrap_or("unknown")
                            .to_string();
                        return Err(SessionError::UnexpectedMessage(msg_type));
                    }
                }
            }
            Ok(Some(Err(e))) => return Err(SessionError::WebSocketError(e.to_string())),
            Ok(None) => return Err(SessionError::ConnectionClosed),
            Err(_) => return Err(SessionError::InitTimeout),
        }
    }
}

/// Convert protocol PermissionMode to types PermissionMode
/// Since both events.rs and types.rs use the same PermissionMode from common.rs,
/// this is now just a pass-through with a default value.
fn convert_permission_mode(mode: Option<&crate::protocol::events::PermissionMode>) -> PermissionMode {
    mode.cloned().unwrap_or(PermissionMode::Default)
}

/// Send initialization error in appropriate format
async fn send_init_error(
    ws_sender: &Sender<WsMessage>,
    is_workspace_init: bool,
    session_id: &str,
    error: &str,
) {
    if is_workspace_init {
        let response = WorkspaceInitResponse {
            id: session_id.to_string(),
            msg_type: "workspace_init_output".to_string(),
            agent_type: "claude".to_string(),
            slash_commands: None,
            mcp_servers: None,
            tools: None,
            agents: None,
            skills: None,
            plugins: None,
            model: None,
            cwd: None,
            claude_code_version: None,
            error: Some(error.to_string()),
        };
        if let Ok(json) = serde_json::to_string(&response) {
            let _ = ws_sender.send(WsMessage::Text(json)).await;
        }
    } else {
        let init_response = AgentEvent::SessionInit {
            success: false,
            session_id: String::new(),
            error: Some(error.to_string()),
            data: SessionInitData::default(),
        };
        if let Ok(json) = serde_json::to_string(&init_response) {
            let _ = ws_sender.send(WsMessage::Text(json)).await;
        }
    }
}

/// Send error event and close connection
async fn send_error_and_close(
    ws_sender: tokio::sync::mpsc::Sender<WsMessage>,
    session_id: &str,
    error: SessionError,
) {
    let error_event = AgentEvent::Error {
        session_id: session_id.to_string(),
        message: error.to_string(),
        is_fatal: true,
    };

    // Send error message
    if let Ok(json) = serde_json::to_string(&error_event) {
        let _ = ws_sender.send(WsMessage::Text(json)).await;
        // The channel drop will eventually close the connection, but we can't explicitly close here easily
        // relying on the receiver loop to close when sender is dropped or error occurs
    }
}

/// 发送 ControlRequestMessage 到 WebSocket
async fn send_control_request(ws_sender: &Sender<WsMessage>, msg: ControlRequestMessage) {
    if let Ok(json) = serde_json::to_string(&msg) {
        if let Err(e) = ws_sender.send(WsMessage::Text(json)).await {
            error!("Failed to send control request: {}", e);
        }
    }
}

/// 发送 SidecarMessage 到 WebSocket
async fn send_message(ws_sender: &Sender<WsMessage>, session_id: &str, msg: &ProtocolMessage) {
    use crate::protocol::events::SidecarMessage;

    let sidecar_msg = SidecarMessage {
        id: session_id.to_string(),
        msg_type: "message".to_string(),
        agent_type: "claude".to_string(),
        data: serde_json::to_value(msg).unwrap_or(serde_json::Value::Null),
    };
    if let Ok(json) = serde_json::to_string(&sidecar_msg) {
        if let Err(e) = ws_sender.send(WsMessage::Text(json)).await {
            error!("Failed to send message: {}", e);
        }
    }
}

/// 发送 SidecarError 到 WebSocket
async fn send_error(ws_sender: &Sender<WsMessage>, session_id: &str, error: &str) {
    use crate::protocol::events::SidecarError;

    let sidecar_error = SidecarError {
        id: session_id.to_string(),
        msg_type: "error".to_string(),
        agent_type: "claude".to_string(),
        error: error.to_string(),
        data: None,
    };
    if let Ok(json) = serde_json::to_string(&sidecar_error) {
        let _ = ws_sender.send(WsMessage::Text(json)).await;
    }
}

/// Query stream processing result
enum QueryStreamResult {
    /// Query completed normally
    Completed,
    /// WebSocket was closed during query
    WebSocketClosed,
}

/// Process a query stream, handling messages, permissions, and cancellation.
/// This is the unified handler for both UserMessage and Query message types.
async fn process_query_stream(
    session: &Session,
    message: String,
    config: &SessionConfig,
    permission_handler: PermissionHandler,
    disallowed_tools: &Option<Vec<String>>,
    ws_sender: &Sender<WsMessage>,
    ws_receiver: &mut SplitStream<WebSocket>,
    perm_rx: &mut tokio::sync::mpsc::Receiver<(PermissionRequest, oneshot::Sender<PermissionResponse>)>,
    pending_permission: &Arc<Mutex<Option<oneshot::Sender<PermissionResponse>>>>,
    current_cancel: &mut Option<CancellationToken>,
) -> QueryStreamResult {
    // Create cancel token
    let cancel_token = CancellationToken::new();
    *current_cancel = Some(cancel_token.clone());

    // Execute query
    let options = QueryOptions {
        permission_mode: config.permission_mode.clone(),
        permission_handler: Some(permission_handler),
        max_turns: config.max_turns,
        env: None,
        disallowed_tools: disallowed_tools.clone(),
    };
    let stream = session.query(message, options, cancel_token);
    pin_mut!(stream);

    // Inner loop: process query stream while also handling permission requests and WebSocket messages
    loop {
        tokio::select! {
            result = stream.next() => {
                match result {
                    Some(Ok(msg)) => {
                        // Send raw ProtocolMessage
                        send_message(ws_sender, session.session_id(), &msg).await;

                        // Check if this is a Result message (indicates turn end)
                        if matches!(msg, ProtocolMessage::Result(_)) {
                            *current_cancel = None;
                            return QueryStreamResult::Completed;
                        }
                    }
                    Some(Err(QueryError::Interrupted)) => {
                        send_error(ws_sender, session.session_id(), "Interrupted by user").await;
                        *current_cancel = None;
                        return QueryStreamResult::Completed;
                    }
                    Some(Err(e)) => {
                        send_error(ws_sender, session.session_id(), &e.to_string()).await;
                        *current_cancel = None;
                        return QueryStreamResult::Completed;
                    }
                    None => {
                        // Stream ended
                        *current_cancel = None;
                        return QueryStreamResult::Completed;
                    }
                }
            }

            // Handle permission requests
            Some((req, resp_tx)) = perm_rx.recv() => {
                // Store response channel
                {
                    let mut pending = pending_permission.lock().await;
                    *pending = Some(resp_tx);
                }

                // Send permission request event to frontend
                let msg = ControlRequestMessage {
                    msg_type: "permission_request".to_string(),
                    id: session.session_id().to_string(),
                    agent_type: "claude".to_string(),
                    tool_name: req.tool_name,
                    tool_use_id: req.tool_use_id,
                    input: req.input,
                    context: PermissionContext {
                        description: "Tool permission request".to_string(),
                        risk_level: RiskLevel::Medium,
                    },
                };
                send_control_request(ws_sender, msg).await;
            }

            // Handle WebSocket messages (permission responses and interrupts)
            ws_msg = ws_receiver.next() => {
                match ws_msg {
                    Some(Ok(WsMessage::Text(text))) => {
                        if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                            match client_msg {
                                ClientMessage::PermissionResponse { id: _, decision, .. } => {
                                    let mut pending = pending_permission.lock().await;
                                    if let Some(sender) = pending.take() {
                                        let response = match decision {
                                            Decision::Allow => PermissionResponse::Allow,
                                            Decision::Deny => PermissionResponse::Deny,
                                            Decision::AllowAlways => PermissionResponse::AllowAlways,
                                        };
                                        let _ = sender.send(response);
                                    }
                                }
                                ClientMessage::ControlRequest { subtype: ControlSubtype::Interrupt, .. } => {
                                    if let Some(token) = current_cancel.take() {
                                        token.cancel();
                                    }
                                }
                                ClientMessage::Cancel { .. } => {
                                    if let Some(token) = current_cancel.take() {
                                        token.cancel();
                                    }
                                }
                                _ => {} // Ignore other messages during query
                            }
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) | None => {
                        // WebSocket closed, exit inner loop and mark
                        *current_cancel = None;
                        return QueryStreamResult::WebSocketClosed;
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Wait for session initialization (System init message) from the Session
/// This is only called for new sessions, not for resume sessions.
async fn wait_for_session_init(
    session: &Session,
) -> Result<(String, SessionInitData), String> {
    use crate::protocol::events::PluginInfo;

    let client_guard = session.client().lock().await;

    // Send empty query and interrupt to get session init data
    info!("Sending empty query to get session init data for session {}", session.session_id());

    // Send empty query to trigger session initialization
    if let Err(e) = client_guard.send_input_message(
        claude_agent_sdk::types::InputMessage::user("".to_string(), session.session_id().to_string())
    ).await {
        return Err(format!("Failed to send empty query: {}", e));
    }

    // Immediately interrupt to stop processing
    let _ = client_guard.interrupt().await;
    info!("Sent interrupt after empty query for session {}", session.session_id());

    // Subscribe to protocol messages
    let mut stream = match client_guard.receive_protocol_messages().await {
        Ok(s) => s,
        Err(e) => return Err(format!("Failed to subscribe to protocol messages: {}", e)),
    };

    // Drop the lock before waiting for messages
    drop(client_guard);

    // Wait for System(init) message with timeout
    let timeout = Duration::from_secs(30);
    let deadline = Instant::now() + timeout;

    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("Timeout waiting for session initialization".to_string());
        }

        let msg_result = tokio::time::timeout(remaining, stream.next()).await;

        match msg_result {
            Ok(Some(Ok(msg))) => {
                info!("Received message from stdout: {:#?}", msg);
                match &msg {
                    ProtocolMessage::System(system) if system.subtype == "init" => {
                        let extra = &system.extra;

                        let actual_session_id = extra
                            .get("session_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or(session.session_id())
                            .to_string();

                        info!("Session initialized, actual session_id: {}", actual_session_id);

                        // Extract session init data
                        let get_string_array = |key: &str| -> Vec<String> {
                            extra.get(key)
                                .and_then(|v| v.as_array())
                                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                                .unwrap_or_default()
                        };

                        let get_string = |key: &str| -> Option<String> {
                            extra.get(key).and_then(|v| v.as_str()).map(String::from)
                        };

                        let plugins: Vec<PluginInfo> = extra.get("plugins")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter().filter_map(|p| {
                                    let name = p.get("name")?.as_str()?.to_string();
                                    let path = p.get("path")?.as_str()?.to_string();
                                    Some(PluginInfo { name, path })
                                }).collect()
                            })
                            .unwrap_or_default();

                        let init_data = SessionInitData {
                            cwd: get_string("cwd"),
                            model: get_string("model"),
                            tools: get_string_array("tools"),
                            mcp_servers: get_string_array("mcp_servers"),
                            permission_mode: get_string("permissionMode"),
                            slash_commands: get_string_array("slash_commands"),
                            api_key_source: get_string("apiKeySource"),
                            claude_code_version: get_string("claude_code_version"),
                            output_style: get_string("output_style"),
                            agents: get_string_array("agents"),
                            skills: get_string_array("skills"),
                            plugins,
                            uuid: get_string("uuid"),
                        };

                        return Ok((actual_session_id, init_data));
                    }
                    ProtocolMessage::Result(result) if result.is_error => {
                        let error_msg = if !result.errors.is_empty() {
                            result.errors.join("; ")
                        } else {
                            result.result.clone().unwrap_or_else(|| "Unknown error".to_string())
                        };
                        return Err(error_msg);
                    }
                    _ => {
                        debug!("Waiting for System(init), got: {:?}", msg);
                        continue;
                    }
                }
            }
            Ok(Some(Err(e))) => return Err(format!("Stream error: {}", e)),
            Ok(None) => return Err("Stream ended unexpectedly".to_string()),
            Err(_) => return Err("Timeout waiting for session initialization".to_string()),
        }
    }
}
