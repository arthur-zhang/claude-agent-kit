//! Pooled agent wrapper for managing ClaudeClient lifecycle.

use claude_agent_sdk::{ClaudeClient, ClaudeAgentOptions, Error};
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
    pub async fn new() -> Result<Self, Error> {
        let options = ClaudeAgentOptions::new();
        let mut client = ClaudeClient::new(options, None);

        // Connect to CLI process
        client.connect(None).await?;

        Ok(Self {
            client,
            last_used: Instant::now(),
            id: Uuid::new_v4(),
        })
    }

    /// Get immutable reference to the client.
    pub fn client(&self) -> &ClaudeClient {
        &self.client
    }

    /// Get mutable reference to the client.
    /// Automatically updates the last used timestamp.
    pub fn client_mut(&mut self) -> &mut ClaudeClient {
        self.touch();
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
    pub async fn disconnect(mut self) -> Result<(), Error> {
        self.client.disconnect().await
    }
}

impl Drop for PooledAgent {
    fn drop(&mut self) {
        // Note: We can't check if client is connected directly, but ClaudeClient's
        // own Drop implementation will warn if it wasn't properly disconnected.
        // This is intentional - we rely on ClaudeClient's cleanup warnings.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_pooled_agent_creation() {
        // This test verifies the structure compiles and returns proper error type
        let result = PooledAgent::new().await;

        match result {
            Ok(agent) => {
                // If we have a real CLI available, verify the agent is properly initialized
                assert_ne!(agent.id(), Uuid::nil(), "Agent should have a valid UUID");
                assert!(
                    agent.idle_duration() < Duration::from_secs(1),
                    "Newly created agent should have minimal idle time"
                );

                // Clean up
                let _ = agent.disconnect().await;
            }
            Err(e) => {
                // Without a real CLI, should get a connection-related error
                assert!(
                    matches!(e, Error::CLIConnection(_) | Error::Process(_) | Error::CLINotFound(_)),
                    "Expected connection-related error, got: {:?}", e
                );
            }
        }
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
        let idle = agent.idle_duration();
        assert!(
            idle >= Duration::from_millis(10),
            "Expected idle duration >= 10ms, got {:?}",
            idle
        );
    }

    #[test]
    fn test_touch_updates_timestamp() {
        use std::thread::sleep;
        use std::time::Duration;

        let options = ClaudeAgentOptions::new();
        let client = ClaudeClient::new(options, None);

        let mut agent = PooledAgent {
            client,
            last_used: Instant::now(),
            id: Uuid::new_v4(),
        };

        sleep(Duration::from_millis(10));
        let idle_before = agent.idle_duration();

        agent.touch();
        let idle_after = agent.idle_duration();

        assert!(
            idle_after < idle_before,
            "Expected idle duration to decrease after touch(), before: {:?}, after: {:?}",
            idle_before,
            idle_after
        );
    }

    #[test]
    fn test_client_mut_calls_touch() {
        use std::thread::sleep;
        use std::time::Duration;

        let options = ClaudeAgentOptions::new();
        let client = ClaudeClient::new(options, None);

        let mut agent = PooledAgent {
            client,
            last_used: Instant::now(),
            id: Uuid::new_v4(),
        };

        sleep(Duration::from_millis(10));
        let idle_before = agent.idle_duration();

        // Call client_mut() which should automatically call touch()
        let _client = agent.client_mut();
        let idle_after = agent.idle_duration();

        assert!(
            idle_after < idle_before,
            "Expected client_mut() to automatically call touch(), before: {:?}, after: {:?}",
            idle_before,
            idle_after
        );
    }

    #[test]
    fn test_client_immutable_access() {
        let options = ClaudeAgentOptions::new();
        let client = ClaudeClient::new(options, None);

        let agent = PooledAgent {
            client,
            last_used: Instant::now(),
            id: Uuid::new_v4(),
        };

        // Verify we can get immutable reference
        let _client_ref = agent.client();

        // Verify we can still access other methods
        let _id = agent.id();
        let _idle = agent.idle_duration();
    }

    #[test]
    fn test_agent_id_is_unique() {
        let options1 = ClaudeAgentOptions::new();
        let client1 = ClaudeClient::new(options1, None);
        let agent1 = PooledAgent {
            client: client1,
            last_used: Instant::now(),
            id: Uuid::new_v4(),
        };

        let options2 = ClaudeAgentOptions::new();
        let client2 = ClaudeClient::new(options2, None);
        let agent2 = PooledAgent {
            client: client2,
            last_used: Instant::now(),
            id: Uuid::new_v4(),
        };

        assert_ne!(
            agent1.id(),
            agent2.id(),
            "Expected unique IDs for different agents"
        );
    }
}
