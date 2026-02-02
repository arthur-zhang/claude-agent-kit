//! Session management.

// Old modules - no longer used after query-based refactoring
// pub mod approval;
// pub mod event_handler;
// pub mod handler;
// pub mod permission;
// pub mod state;

pub mod query;

// Re-export query module types
pub use query::{PermissionHandler, PermissionRequest, PermissionResponse, QueryError, QueryOptions, Session};
