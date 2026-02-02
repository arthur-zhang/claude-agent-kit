use std::fmt;

/// Query 过程中的错误
#[derive(Debug)]
pub enum QueryError {
    /// API 错误
    ApiError(String),
    /// 连接丢失
    ConnectionLost,
    /// 超时
    Timeout,
    /// 被中断
    Interrupted,
    /// SDK 错误
    SdkError(claude_agent_sdk::Error),
    /// 权限被拒绝
    PermissionDenied(String),
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryError::ApiError(msg) => write!(f, "API Error: {}", msg),
            QueryError::ConnectionLost => write!(f, "Connection lost"),
            QueryError::Timeout => write!(f, "Operation timed out"),
            QueryError::Interrupted => write!(f, "Query interrupted"),
            QueryError::SdkError(e) => write!(f, "SDK Error: {}", e),
            QueryError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
        }
    }
}

impl std::error::Error for QueryError {}

impl From<claude_agent_sdk::Error> for QueryError {
    fn from(e: claude_agent_sdk::Error) -> Self {
        QueryError::SdkError(e)
    }
}
