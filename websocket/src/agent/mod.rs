//! Agent integration module for managing Claude Code CLI processes.

pub mod pool;
pub mod client;
pub mod session;

pub use pool::{AgentPool, AgentPoolConfig};
pub use client::PooledAgent;
pub use session::AgentSession;
