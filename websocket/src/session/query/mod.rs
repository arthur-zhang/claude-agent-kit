pub mod error;
pub mod session;

pub use error::QueryError;
pub use session::Session;

use crate::protocol::types::PermissionMode;
use std::collections::HashMap;
use std::sync::Arc;
use futures::future::BoxFuture;

/// 权限请求
#[derive(Debug, Clone)]
pub struct PermissionRequest {
    pub tool_name: String,
    pub tool_use_id: Option<String>,
    pub input: serde_json::Value,
}

/// 权限响应
#[derive(Debug, Clone)]
pub enum PermissionResponse {
    Allow,
    Deny,
    AllowAlways,
}

/// 权限处理器类型
pub type PermissionHandler = Arc<
    dyn Fn(PermissionRequest) -> BoxFuture<'static, PermissionResponse>
        + Send
        + Sync,
>;

/// Query 选项
#[derive(Clone)]
pub struct QueryOptions {
    /// 权限模式
    pub permission_mode: PermissionMode,
    /// 权限处理器回调
    pub permission_handler: Option<PermissionHandler>,
    /// 最大轮次数
    pub max_turns: Option<i32>,
    /// 额外的环境变量
    pub env: Option<HashMap<String, String>>,
    /// 禁用的工具列表
    pub disallowed_tools: Option<Vec<String>>,
}

impl Default for QueryOptions {
    fn default() -> Self {
        Self {
            permission_mode: PermissionMode::default(),
            permission_handler: None,
            max_turns: None,
            env: None,
            disallowed_tools: None,
        }
    }
}

impl std::fmt::Debug for QueryOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryOptions")
            .field("permission_mode", &self.permission_mode)
            .field("permission_handler", &self.permission_handler.as_ref().map(|_| "<handler>"))
            .field("max_turns", &self.max_turns)
            .field("env", &self.env)
            .field("disallowed_tools", &self.disallowed_tools)
            .finish()
    }
}
