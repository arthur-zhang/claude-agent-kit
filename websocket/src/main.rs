use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use websocket::agent::AgentPoolConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "websocket=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Read pool config from environment
    let pool_config = AgentPoolConfig::default();

    // Create router with agent pool
    let app = websocket::server::create_router(pool_config).await?;

    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await?;

    tracing::info!("WebSocket server listening on: {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
