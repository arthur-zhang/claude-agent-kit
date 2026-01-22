//! Claude Agent SDK - Rust implementation
//!
//! This crate provides type definitions and utilities for building Claude agents in Rust.
//! It is a complete port of the Python Claude Agent SDK types, maintaining full compatibility
//! with the JSON protocol.
//!
//! # Features
//!
//! - **Type-safe**: Leverages Rust's type system for compile-time safety
//! - **Async support**: Built on tokio for async operations
//! - **Serialization**: Full serde support for JSON serialization/deserialization
//! - **Modular**: Organized into logical modules (permissions, hooks, messages, etc.)
//!
//! # Example
//!
//! ```rust
//! use claude_agent_sdk_ng::types::{ClaudeAgentOptions, PermissionMode};
//!
//! let options = ClaudeAgentOptions::new()
//!     .with_model("claude-sonnet-4")
//!     .with_max_turns(10)
//!     .with_permission_mode(PermissionMode::Plan);
//! ```
//!
//! # Modules
//!
//! - [`types`] - All type definitions (permissions, hooks, messages, etc.)
//! - [`internal`] - Internal implementation (transport, query, client)
//! - [`client`] - High-level client API

pub mod types;
pub mod internal;
pub mod client;

// Re-export all public types at the crate root for convenience
pub use types::*;
pub use internal::InternalClient;
pub use client::ClaudeClient;
