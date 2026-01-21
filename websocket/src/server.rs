use crate::agent::{AgentPool, AgentPoolConfig, AgentSession};
use crate::connection::ConnectionManager;
use axum::{
    extract::{
        ws::WebSocket,
        State, WebSocketUpgrade,
    },
    response::Response,
    routing::get,
    Router,
};
use std::sync::Arc;
use tower_http::services::ServeDir;
use tracing::{debug, info};

#[derive(Clone)]
pub struct AppState {
    pub connection_manager: ConnectionManager,
    pub agent_pool: Arc<AgentPool>,
}

pub async fn create_router(pool_config: AgentPoolConfig) -> Result<Router, Box<dyn std::error::Error>> {
    // Initialize agent pool
    let agent_pool = Arc::new(AgentPool::new(pool_config).await?);

    let state = AppState {
        connection_manager: ConnectionManager::new(),
        agent_pool,
    };

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .nest_service("/", ServeDir::new("static"))
        .with_state(state);

    Ok(app)
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    debug!("New WebSocket connection");

    // Acquire agent from pool
    let agent = match state.agent_pool.acquire().await {
        Ok(agent) => agent,
        Err(e) => {
            tracing::error!("Failed to acquire agent: {}", e);
            return;
        }
    };

    info!("Acquired agent {} for WebSocket connection", agent.id());

    // Create and run session
    let session = AgentSession::new();
    let session_id = session.session_id();

    if let Err(e) = session.run(socket, agent).await {
        tracing::error!("Session {} error: {}", session_id, e);
    }

    info!("WebSocket connection closed");
}
