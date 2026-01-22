use claude_agent_sdk::{ClaudeClient, Error};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// Manages agent sessions, mapping session IDs to Claude clients.
/// Each session gets its own dedicated client instance.
#[derive(Clone)]
pub struct SessionManager {
    /// Map from session_id to client
    sessions: Arc<DashMap<String, Arc<Mutex<ClaudeClient>>>>,
}

impl SessionManager {
    /// Create a new session manager.
    pub fn new() -> Self {
        info!("Created session manager");
        Self {
            sessions: Arc::new(DashMap::new()),
        }
    }

    /// Get an existing client for the given session ID.
    pub fn get(&self, session_id: &str) -> Option<Arc<Mutex<ClaudeClient>>> {
        self.sessions.get(session_id).map(|entry| entry.value().clone())
    }

    /// Register a client for a session ID.
    /// This is used when the session_id is extracted from the init message.
    pub fn register(&self, session_id: String, client: Arc<Mutex<ClaudeClient>>) {
        info!("Registering client for session {}", session_id);
        self.sessions.insert(session_id, client);
    }

    /// Remove a session and disconnect its client.
    pub async fn remove(&self, session_id: &str) -> Result<(), Error> {
        if let Some((_, client)) = self.sessions.remove(session_id) {
            info!("Removing session {} and disconnecting client", session_id);
            let mut client_guard = client.lock().await;
            client_guard.disconnect().await?;
        }
        Ok(())
    }

    /// Get the number of active sessions.
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Check if a session exists.
    pub fn has_session(&self, session_id: &str) -> bool {
        self.sessions.contains_key(session_id)
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_manager_creation() {
        let manager = SessionManager::new();
        assert_eq!(manager.session_count(), 0);
    }

    #[tokio::test]
    async fn test_has_session() {
        let manager = SessionManager::new();
        assert!(!manager.has_session("test-session"));
    }

    #[tokio::test]
    async fn test_session_count() {
        let manager = SessionManager::new();
        assert_eq!(manager.session_count(), 0);
    }
}
