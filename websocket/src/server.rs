use crate::connection::ConnectionManager;
use crate::protocol::types::{PermissionMode, SessionConfig};
use crate::session::handler::{HandlerConfig, handle_session_with_agent};
use axum::{
    Router,
    extract::{Query, State, WebSocketUpgrade, ws::WebSocket},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use claude_agent_sdk::{ClaudeAgentOptions, ClaudeClient};
use rust_embed::RustEmbed;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

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

    // Create client
    let options = ClaudeAgentOptions::new();
    let mut client = ClaudeClient::new(options);

    // Connect to Claude
    if let Err(e) = client.connect(None).await {
        error!("Failed to connect client for session {}: {}", session_id, e);
        return;
    }

    let client = Arc::new(Mutex::new(client));

    // Create session config
    let config = SessionConfig {
        permission_mode: PermissionMode::Manual,
        max_turns: None,
        metadata: Default::default(),
    };

    // Use new handler
    let handler_config = HandlerConfig::default();
    if let Err(e) =
        handle_session_with_agent(socket, session_id.clone(), config, handler_config, client).await
    {
        error!("Session handler error for {}: {}", session_id, e);
    }

    info!("WebSocket connection closed for session {}", session_id);
}
