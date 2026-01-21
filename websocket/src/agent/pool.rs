//! Agent pool for managing ClaudeClient instances.

use crate::agent::client::PooledAgent;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

/// Configuration for the agent pool.
#[derive(Debug, Clone)]
pub struct AgentPoolConfig {
    pub core_size: usize,
    pub max_size: usize,
    pub idle_timeout: Duration,
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
    pub async fn new(config: AgentPoolConfig) -> Result<Self, String> {
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
    async fn ensure_core_pool(&self) -> Result<(), String> {
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
                    return Err(e.to_string());
                }
            }
        }

        info!("Core pool initialized with {} agents", inner.total_count);
        Ok(())
    }

    /// Acquire an agent from the pool.
    pub async fn acquire(&self) -> Result<PooledAgent, String> {
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
        Err("Pool exhausted".to_string())
    }

    /// Release an agent back to the pool.
    pub async fn release(&self, agent: PooledAgent) {
        debug!("Releasing agent {} back to pool", agent.id());

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

        // This will fail without claude code CLI, but tests structure
        let result = AgentPool::new(config).await;
        let _ = result;
    }

    #[tokio::test]
    async fn test_pool_stats() {
        // Mock test - in real test we'd use a mock transport
        // For now, just verify the structure compiles
        let config = AgentPoolConfig::default();
        assert_eq!(config.core_size, 2);
        assert_eq!(config.max_size, 20);
    }
}
