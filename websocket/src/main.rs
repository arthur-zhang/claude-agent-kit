use std::env;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use websocket::{create_app, ConnectionManager};

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "websocket=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 获取监听地址
    let addr = env::var("WEBSOCKET_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());

    info!("Starting WebSocket server on {}", addr);

    // 创建连接管理器
    let manager = ConnectionManager::new();

    // 创建应用
    let app = create_app(manager);

    // 启动服务器
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind address");

    info!("WebSocket server listening on {}", addr);

    axum::serve(listener, app)
        .await
        .expect("Server error");
}
