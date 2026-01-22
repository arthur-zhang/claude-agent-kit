//! Agent integration module for managing Claude Code CLI processes.

pub mod session;
pub mod session_manager;

pub use session::AgentSession;
pub use session_manager::SessionManager;
