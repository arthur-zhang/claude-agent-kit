//! Session state management.

use crate::protocol::types::SessionConfig;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{Mutex, oneshot};

/// Session state.
pub struct SessionState {
    pub session_id: String,
    pub config: SessionConfig,
    pub status: Arc<Mutex<AgentState>>,
    pub pending_permission: Arc<Mutex<Option<PendingPermission>>>,
    message_id_counter: AtomicU64,
}

impl SessionState {
    /// Create a new session state.
    pub fn new(session_id: String, config: SessionConfig) -> Self {
        Self {
            session_id,
            config,
            status: Arc::new(Mutex::new(AgentState::Idle)),
            pending_permission: Arc::new(Mutex::new(None)),
            message_id_counter: AtomicU64::new(0),
        }
    }

    /// Generate next message ID.
    pub fn next_message_id(&self) -> String {
        let id = self.message_id_counter.fetch_add(1, Ordering::SeqCst);
        format!("msg-{}", id)
    }

    /// Set session status.
    pub async fn set_status(&self, status: AgentState) {
        *self.status.lock().await = status;
    }

    /// Get current session status.
    pub async fn status(&self) -> AgentState {
        self.status.lock().await.clone()
    }
}

/// Agent execution state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentState {
    Idle,
    Thinking,
    ExecutingTool,
    WaitingPermission,
}

/// Pending permission request.
pub struct PendingPermission {
    pub request_id: String,
    pub tool_name: String,
    pub tool_input: serde_json::Value,
    pub response_tx: oneshot::Sender<PermissionDecision>,
}

/// Permission decision type (placeholder for now).
pub type PermissionDecision = bool;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::types::PermissionMode;

    #[test]
    fn test_session_state_creation() {
        let config = SessionConfig {
            permission_mode: PermissionMode::Manual,
            max_turns: None,
            metadata: Default::default(),
        };
        let state = SessionState::new("test-session".to_string(), config);
        assert_eq!(state.session_id, "test-session");
    }

    #[test]
    fn test_message_id_generation() {
        let config = SessionConfig {
            permission_mode: PermissionMode::Manual,
            max_turns: None,
            metadata: Default::default(),
        };
        let state = SessionState::new("test-session".to_string(), config);
        assert_eq!(state.next_message_id(), "msg-0");
        assert_eq!(state.next_message_id(), "msg-1");
        assert_eq!(state.next_message_id(), "msg-2");
    }
}
