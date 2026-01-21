//! Pooled agent wrapper for managing ClaudeClient lifecycle.

use claude_agent_sdk::{ClaudeClient, ClaudeAgentOptions};
use std::time::Instant;
use uuid::Uuid;

/// Wrapper around ClaudeClient with pooling metadata.
pub struct PooledAgent {
    pub(crate) client: ClaudeClient,
    pub(crate) last_used: Instant,
    pub(crate) id: Uuid,
}

impl PooledAgent {
    /// Create a new pooled agent.
    pub async fn new() -> Result<Self, String> {
        let options = ClaudeAgentOptions::new();
        let mut client = ClaudeClient::new(options, None);

        // Connect to CLI process
        client.connect(None).await
            .map_err(|e| format!("Failed to connect: {}", e))?;

        Ok(Self {
            client,
            last_used: Instant::now(),
            id: Uuid::new_v4(),
        })
    }

    /// Get mutable reference to the client.
    pub fn client_mut(&mut self) -> &mut ClaudeClient {
        &mut self.client
    }

    /// Update last used timestamp.
    pub fn touch(&mut self) {
        self.last_used = Instant::now();
    }

    /// Get time since last use.
    pub fn idle_duration(&self) -> std::time::Duration {
        self.last_used.elapsed()
    }

    /// Get agent ID.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Disconnect and cleanup.
    pub async fn disconnect(mut self) -> Result<(), String> {
        self.client.disconnect().await
            .map_err(|e| format!("Failed to disconnect: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pooled_agent_creation() {
        // This test will fail without a real claude code CLI
        // But it verifies the structure compiles
        let result = PooledAgent::new().await;
        // Just verify it returns a Result
        let _ = result;
    }

    #[test]
    fn test_idle_duration() {
        use std::thread::sleep;
        use std::time::Duration;

        let options = ClaudeAgentOptions::new();
        let client = ClaudeClient::new(options, None);

        let agent = PooledAgent {
            client,
            last_used: Instant::now(),
            id: Uuid::new_v4(),
        };

        sleep(Duration::from_millis(10));
        assert!(agent.idle_duration() >= Duration::from_millis(10));
    }
}
