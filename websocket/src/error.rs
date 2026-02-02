use std::fmt;

#[derive(Debug)]
pub enum WebSocketError {
    /// JSON 序列化/反序列化错误
    JsonError(serde_json::Error),
    /// 未知的消息操作
    UnknownAction(String),
    /// 连接不存在
    ConnectionNotFound(String),
    /// 内部错误
    InternalError(String),
}

impl fmt::Display for WebSocketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WebSocketError::JsonError(e) => write!(f, "JSON error: {}", e),
            WebSocketError::UnknownAction(action) => write!(f, "Unknown action: {}", action),
            WebSocketError::ConnectionNotFound(id) => write!(f, "Connection not found: {}", id),
            WebSocketError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for WebSocketError {}

impl From<serde_json::Error> for WebSocketError {
    fn from(err: serde_json::Error) -> Self {
        WebSocketError::JsonError(err)
    }
}

pub type Result<T> = std::result::Result<T, WebSocketError>;
