// pub mod agent;
pub mod connection;
pub mod error;
pub mod protocol;
pub mod server;
pub mod session;

pub use connection::{ConnectionId, ConnectionManager};
pub use error::{Result, WebSocketError};
pub use protocol::types::{ClientMessage, ServerMessage};
pub use server::create_router;



#[cfg(test)]
mod tests {
    use std::time::Duration;

    use claude_agent_sdk::{ClaudeAgentOptions, ClaudeClient, InputMessage, client::ClientPromptInput};
    use futures::StreamExt as _;
    use tracing::info;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    use super::*;

    #[tokio::test]
    async fn test_it_compiles() {
        
        tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "websocket=debug,tower_http=debug".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)  // Disable ANSI color codes
                .with_target(true)  // Show target module
                .with_level(true)   // Show log level
        )
        .init();

        info!("Starting test");

        let options = ClaudeAgentOptions::new();
        let mut client = ClaudeClient::new(options);
        // let _ = client.stderr_receiver();
        // let _ = client.process_handle();
        client.connect(None).await.unwrap();
        client.send_input_message(InputMessage::user("".to_string(), "default".to_string())).await.unwrap();

        client.interrupt().await.unwrap();

        let mut stream = client.receive_messages_from_cc_stdout().await.unwrap();
        while let Some(message) = stream.next().await {
            info!("Received message: {:?}", message);
        }

        tokio::time::sleep(Duration::from_secs(100000)).await;

        
    }
}
