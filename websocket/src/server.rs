use crate::connection::ConnectionManager;
use axum::{
    Router,
    extract::{Query, State, WebSocketUpgrade, ws::WebSocket},
    response::{Response, IntoResponse, Html},
    routing::get,
    http::{StatusCode, header},
};
use axum::extract::ws::Message;
use claude_agent_sdk::internal::transport::{PromptInput, SubprocessCLITransport};
use claude_agent_sdk::{
    ClaudeAgentOptions,  SDKControlRequest, SDKControlRequestType,
};
use futures::{SinkExt, StreamExt};
use rust_embed::RustEmbed;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::spawn;
use tokio::sync::{Mutex, mpsc};
use tracing::{debug, error, info};
use claude_agent_sdk::messages::InputMessage;

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

async fn handle_socket(socket: WebSocket, state: AppState, session_id: String) {
    info!("New WebSocket connection for session {}", session_id);

    // Check if client already exists for this session_id
    // let client = if let Some(existing_client) = state.session_manager.get(&session_id) {
    //     info!("Reusing existing client for session {}", session_id);
    //     existing_client
    // } else {
    info!("Creating new client for session {}", session_id);

    // Create new client
    let options = ClaudeAgentOptions::new();
    // let mut new_client = ClaudeClient::new(options);
    let (_tx, rx) = mpsc::channel::<serde_json::Value>(1);
    let prompt = PromptInput::Stream(rx);
    let mut t = SubprocessCLITransport::new(prompt, options).unwrap();
    t.connect().await.unwrap();

    let (r, mut w, _, _process_handle) = t.split().unwrap();

    let (tx, mut rx) = mpsc::channel::<String>(100);
    tokio::spawn(async move {
        while let Some(it) = rx.recv().await {
            println!("send stdin <<<<<{:?}", it);
            let _ = w.write_with_newline(&it).await;
        }
        println!("closing transport");
    });

    let init_req = SDKControlRequest {
        type_: "control_request".to_string(),
        request_id: uuid::Uuid::new_v4().to_string(),
        request: SDKControlRequestType::Initialize {hooks:None},
    };
    let payload = serde_json::to_string(&init_req).unwrap();

    tx.send(payload).await.unwrap();

    let (mut ws_sender, mut ws_receiver) = socket.split();

    let (ws_tx, mut ws_rx) = mpsc::channel::<String>(100);
    tokio::spawn(async move {
        while let Some(it) = ws_rx.recv().await {
            ws_sender.send(Message::Text(it)).await.unwrap();
        }
    });
    tokio::spawn({
        let session_id = session_id.clone();
        async move {
            while let Some(Ok(it)) = ws_receiver.next().await {
                match it {
                    Message::Text(text) => {
                        info!("Received WebSocket message {text}");

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
                        let payload = InputMessage::user(prompt.to_string(), session_id.clone());
                        let payload = serde_json::to_string(&payload).unwrap();

                        if let Err(e) = tx.send(payload).await {
                            error!("Failed to query agent: {}", e);
                            // Send error message to client

                            break;
                        }
                    }
                    Message::Binary(_) => {}
                    Message::Ping(_) => {}
                    Message::Pong(_) => {}
                    Message::Close(_) => {
                        break;
                    }
                }

            }
        }
    });



    let mut m = r.read_messages();
    while let Some(msg) = m.recv().await {
        println!("recv stdout >>>>>{:?}", msg);
        let payload = serde_json::to_string(&msg).unwrap();
        ws_tx.send(payload).await.unwrap();
    }

    info!("WebSocket connection closed for session {}", session_id);
}
