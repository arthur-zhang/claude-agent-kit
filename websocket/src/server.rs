use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::connection::ConnectionManager;
use crate::handler::handle_message;
use crate::message::{ClientMessage, ServerMessage};

/// 创建 Axum 应用路由
pub fn create_app(manager: ConnectionManager) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/health", get(health_handler))
        .route("/ws", get(websocket_handler))
        .with_state(manager)
}

/// 首页处理器
async fn index_handler() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

/// 健康检查处理器
async fn health_handler() -> impl IntoResponse {
    "OK"
}

/// WebSocket 升级处理器
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(manager): State<ConnectionManager>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, manager))
}

/// 处理 WebSocket 连接
async fn handle_socket(socket: WebSocket, manager: ConnectionManager) {
    let connection_id = Uuid::new_v4();
    info!("New WebSocket connection: {}", connection_id);

    // 分离 WebSocket 的发送和接收端
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // 创建消息通道
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();

    // 将连接添加到管理器
    manager.add_connection(connection_id, tx).await;

    // 发送欢迎消息
    let welcome_msg = ServerMessage::welcome(connection_id);
    if let Ok(json) = serde_json::to_string(&welcome_msg) {
        if let Err(e) = ws_sender.send(Message::Text(json)).await {
            error!("Failed to send welcome message: {}", e);
        }
    }

    // 创建发送任务
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match serde_json::to_string(&msg) {
                Ok(json) => {
                    if let Err(e) = ws_sender.send(Message::Text(json)).await {
                        error!("Failed to send message: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to serialize message: {}", e);
                }
            }
        }
    });

    // 创建接收任务
    let manager_clone = manager.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(result) = ws_receiver.next().await {
            match result {
                Ok(Message::Text(text)) => {
                    debug!("Received text message: {}", text);

                    // 解析客户端消息
                    match serde_json::from_str::<ClientMessage>(&text) {
                        Ok(client_msg) => {
                            // 处理消息
                            let response =
                                handle_message(&manager_clone, connection_id, client_msg).await;

                            // 发送响应
                            if let Err(e) = manager_clone.send_to(&connection_id, response).await {
                                error!("Failed to send response: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Failed to parse message: {}", e);
                            let error_msg = ServerMessage::error(
                                None,
                                format!("Invalid message format: {}", e),
                            );
                            let _ = manager_clone.send_to(&connection_id, error_msg).await;
                        }
                    }
                }
                Ok(Message::Binary(_)) => {
                    warn!("Received binary message (not supported)");
                }
                Ok(Message::Ping(_)) => {
                    debug!("Received ping");
                }
                Ok(Message::Pong(_)) => {
                    debug!("Received pong");
                }
                Ok(Message::Close(_)) => {
                    info!("Client closed connection: {}", connection_id);
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
            }
        }
    });

    // 等待任务完成
    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
        }
        _ = &mut recv_task => {
            send_task.abort();
        }
    }

    // 清理连接
    manager.remove_connection(&connection_id).await;
    info!("Connection closed: {}", connection_id);
}
