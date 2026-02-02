//! WebSocket protocol message types and conversion.

pub mod common;
pub mod converter;
pub mod event_converter;
pub mod events;
pub mod sdk_converter;
pub mod types;

// Re-export common types
pub use common::{Decision, PermissionContext, PermissionMode, RiskLevel};
pub use events::{AgentEvent, ClientMessage};
pub use sdk_converter::sdk_to_events;
pub use types::ServerMessage;
