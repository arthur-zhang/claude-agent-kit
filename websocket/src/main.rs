use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

    // Create router with session manager
    let app = websocket::server::create_router().await?;

    // Start server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await?;

    tracing::info!("WebSocket server listening on: {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
