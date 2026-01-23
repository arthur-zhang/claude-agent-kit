//! WebSocket protocol message types.
//!
//! This module defines all message types according to the protocol specification.

use serde::{Deserialize, Serialize};

/// Messages sent from client to server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Placeholder - will be implemented in Task 2
    #[doc(hidden)]
    _Placeholder,
}

/// Messages sent from server to client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Placeholder - will be implemented in Task 3
    #[doc(hidden)]
    _Placeholder,
}
