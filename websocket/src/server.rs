use crate::agent::{AgentSession, SessionManager};
use crate::connection::ConnectionManager;
use axum::{
    extract::{
        ws::WebSocket,
        Query, State, WebSocketUpgrade,
    },
    response::Response,
    routing::get,
    Router,
};
use claude_agent_sdk::{ClaudeAgentOptions, ClaudeClient};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;
use tracing::{error, info};

#[derive(Clone)]
pub struct AppState {
    pub connection_manager: ConnectionManager,
    pub session_manager: SessionManager,
}

#[derive(Deserialize)]
pub struct WsQuery {
    session_id: Option<String>,
}

pub async fn create_router() -> Result<Router, Box<dyn std::error::Error>> {
    let state = AppState {
        connection_manager: ConnectionManager::new(),
        session_manager: SessionManager::new(),
    };

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .nest_service("/", ServeDir::new("static"))
        .with_state(state);

    Ok(app)
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(state): State<AppState>,
) -> Response {
    let session_id = query.session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    ws.on_upgrade(move |socket| handle_socket(socket, state, session_id))
}

async fn handle_socket(socket: WebSocket, state: AppState, session_id: String) {
    info!("New WebSocket connection for session {}", session_id);

    // Check if client already exists for this session_id
    let client = if let Some(existing_client) = state.session_manager.get(&session_id) {
        info!("Reusing existing client for session {}", session_id);
        existing_client
    } else {
        info!("Creating new client for session {}", session_id);

        // Create new client
        let options = ClaudeAgentOptions::new();
        let mut new_client = ClaudeClient::new(options, None);

        // Connect to CLI process
        if let Err(e) = new_client.connect(None).await {
            error!("Failed to connect client: {}", e);
            return;
        }

        info!("Created and connected client");

        // Wrap client in Arc<Mutex>
        let client = Arc::new(Mutex::new(new_client));

        // Register to SessionManager
        state.session_manager.register(session_id.clone(), Arc::clone(&client));

        client
    };

    // Create and run session
    let session = AgentSession::new();

    if let Err(e) = session.run(socket, client.clone(), session_id.clone()).await {
        error!("Session {} error: {}", session_id, e);
        // On error, remove from SessionManager and disconnect
        if let Err(e) = state.session_manager.remove(&session_id).await {
            error!("Failed to remove session: {}", e);
        }
    } else {
        info!("Session {} completed successfully", session_id);
    }

    info!("WebSocket connection closed for session {}", session_id);
}
