//! Agent pool for managing ClaudeClient instances.

use crate::agent::client::PooledAgent;
use claude_agent_sdk::Error;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// Configuration for the agent pool.
#[derive(Debug, Clone)]
pub struct AgentPoolConfig {
    pub core_size: usize,
    // Phase 2: Dynamic pool sizing - not yet implemented
    pub max_size: usize,
    // Phase 2: Idle agent eviction - not yet implemented
    pub idle_timeout: Duration,
    // Phase 2: Acquire timeout with waiting - not yet implemented
    pub acquire_timeout: Duration,
}

impl Default for AgentPoolConfig {
    fn default() -> Self {
        Self {
            core_size: 2,
            max_size: 20,
            idle_timeout: Duration::from_secs(300), // 5 minutes
            acquire_timeout: Duration::from_secs(30),
        }
    }
}

/// Pool of agent instances.
pub struct AgentPool {
    config: AgentPoolConfig,
    inner: Arc<Mutex<AgentPoolInner>>,
}

struct AgentPoolInner {
    idle_agents: VecDeque<PooledAgent>,
    total_count: usize,
}

impl AgentPool {
    /// Create a new agent pool with the given configuration.
    pub async fn new(config: AgentPoolConfig) -> Result<Self, Error> {
        info!("Creating agent pool with core_size={}", config.core_size);

        let pool = Self {
            config,
            inner: Arc::new(Mutex::new(AgentPoolInner {
                idle_agents: VecDeque::new(),
                total_count: 0,
            })),
        };

        // Pre-create core pool agents
        pool.ensure_core_pool().await?;

        Ok(pool)
    }

    /// Ensure core pool agents are created.
    async fn ensure_core_pool(&self) -> Result<(), Error> {
        let mut inner = self.inner.lock().await;

        while inner.total_count < self.config.core_size {
            debug!("Creating core pool agent {}/{}", inner.total_count + 1, self.config.core_size);

            match PooledAgent::new().await {
                Ok(agent) => {
                    inner.idle_agents.push_back(agent);
                    inner.total_count += 1;
                }
                Err(e) => {
                    error!("Failed to create core pool agent: {}", e);
                    return Err(e);
                }
            }
        }

        info!("Core pool initialized with {} agents", inner.total_count);
        Ok(())
    }

    /// Acquire an agent from the pool.
    pub async fn acquire(&self) -> Result<PooledAgent, Error> {
        debug!("Acquiring agent from pool");

        let mut inner = self.inner.lock().await;

        // Try to get from idle agents
        if let Some(mut agent) = inner.idle_agents.pop_front() {
            debug!("Reusing idle agent {}", agent.id());
            agent.touch();
            return Ok(agent);
        }

        // For Phase 1, we only support fixed size pool
        // If no idle agents, we fail
        error!("No idle agents available in pool");
        Err(Error::Unknown("Pool exhausted".to_string()))
    }

    /// Release an agent back to the pool.
    ///
    /// Note: Phase 1 limitation - we don't perform health checks before returning
    /// agents to the pool. In Phase 2, we should verify the agent is still connected
    /// and functional before returning it to the pool.
    pub async fn release(&self, agent: PooledAgent) {
        let agent_id = agent.id();
        debug!("Releasing agent {} back to pool", agent_id);

        // Phase 1: Simple return to pool without health check
        // Phase 2 TODO: Add health check here to verify agent is still connected
        // If agent is disconnected or failed, we should:
        // 1. Disconnect the agent
        // 2. Decrement total_count
        // 3. Not return it to the pool

        let mut inner = self.inner.lock().await;
        inner.idle_agents.push_back(agent);
    }

    /// Get pool statistics.
    pub async fn stats(&self) -> PoolStats {
        let inner = self.inner.lock().await;

        PoolStats {
            total_count: inner.total_count,
            idle_count: inner.idle_agents.len(),
            active_count: inner.total_count - inner.idle_agents.len(),
        }
    }
}

impl Drop for AgentPool {
    fn drop(&mut self) {
        // We need to disconnect all agents when the pool is destroyed
        // However, we can't call async methods in Drop, so we spawn a task
        let inner = Arc::clone(&self.inner);

        tokio::spawn(async move {
            let mut inner = inner.lock().await;
            let agent_count = inner.idle_agents.len();

            if agent_count > 0 {
                warn!("AgentPool dropped with {} idle agents, disconnecting...", agent_count);

                while let Some(agent) = inner.idle_agents.pop_front() {
                    let agent_id = agent.id();
                    match agent.disconnect().await {
                        Ok(_) => debug!("Disconnected agent {} during pool cleanup", agent_id),
                        Err(e) => error!("Failed to disconnect agent {} during pool cleanup: {}", agent_id, e),
                    }
                }

                inner.total_count = 0;
            }
        });
    }
}

/// Pool statistics.
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_count: usize,
    pub idle_count: usize,
    pub active_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_creation() {
        let config = AgentPoolConfig {
            core_size: 1,
            ..Default::default()
        };

        // This will fail without claude code CLI, but tests structure and error handling
        let result = AgentPool::new(config).await;

        match result {
            Ok(pool) => {
                // If we have a real CLI available, verify pool was initialized correctly
                let stats = pool.stats().await;
                assert_eq!(stats.total_count, 1, "Pool should have 1 agent");
                assert_eq!(stats.idle_count, 1, "All agents should be idle initially");
                assert_eq!(stats.active_count, 0, "No agents should be active initially");
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

    #[tokio::test]
    async fn test_pool_acquire_and_release() {
        let config = AgentPoolConfig {
            core_size: 2,
            ..Default::default()
        };

        let result = AgentPool::new(config).await;

        match result {
            Ok(pool) => {
                // Verify initial state
                let stats = pool.stats().await;
                assert_eq!(stats.total_count, 2);
                assert_eq!(stats.idle_count, 2);
                assert_eq!(stats.active_count, 0);

                // Acquire an agent
                let agent = pool.acquire().await.expect("Should acquire agent");
                let agent_id = agent.id();

                // Verify stats after acquire
                let stats = pool.stats().await;
                assert_eq!(stats.total_count, 2);
                assert_eq!(stats.idle_count, 1);
                assert_eq!(stats.active_count, 1);

                // Release the agent
                pool.release(agent).await;

                // Verify stats after release
                let stats = pool.stats().await;
                assert_eq!(stats.total_count, 2);
                assert_eq!(stats.idle_count, 2);
                assert_eq!(stats.active_count, 0);

                debug!("Successfully acquired and released agent {}", agent_id);
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

    #[tokio::test]
    async fn test_pool_exhaustion() {
        let config = AgentPoolConfig {
            core_size: 1,
            ..Default::default()
        };

        let result = AgentPool::new(config).await;

        match result {
            Ok(pool) => {
                // Acquire the only agent
                let agent1 = pool.acquire().await.expect("Should acquire first agent");

                // Try to acquire another - should fail
                let result2 = pool.acquire().await;
                assert!(result2.is_err(), "Should fail when pool is exhausted");

                if let Err(Error::Unknown(msg)) = result2 {
                    assert_eq!(msg, "Pool exhausted");
                } else {
                    panic!("Expected Error::Unknown with 'Pool exhausted' message");
                }

                // Release and verify we can acquire again
                pool.release(agent1).await;
                let agent3 = pool.acquire().await.expect("Should acquire after release");
                pool.release(agent3).await;
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
    fn test_pool_config_defaults() {
        let config = AgentPoolConfig::default();
        assert_eq!(config.core_size, 2);
        assert_eq!(config.max_size, 20);
        assert_eq!(config.idle_timeout, Duration::from_secs(300));
        assert_eq!(config.acquire_timeout, Duration::from_secs(30));
    }
}
