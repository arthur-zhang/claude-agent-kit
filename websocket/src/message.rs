use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 连接唯一标识符
pub type ConnectionId = Uuid;

/// 客户端发送的消息
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClientMessage {
    /// 消息唯一标识
    pub id: String,
    /// 消息类型
    #[serde(rename = "type")]
    pub msg_type: String,
    /// 操作类型
    pub action: String,
    /// 消息负载
    #[serde(default)]
    pub payload: serde_json::Value,
}

/// 客户端操作类型
#[derive(Debug, Clone)]
pub enum ClientAction {
    /// 回显消息
    Echo { message: String },
    /// 广播消息
    Broadcast { message: String },
    /// 获取连接数
    GetConnections,
}

impl ClientAction {
    /// 从客户端消息解析操作
    pub fn from_message(msg: &ClientMessage) -> Result<Self, String> {
        match msg.action.as_str() {
            "echo" => {
                let message = msg
                    .payload
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Ok(ClientAction::Echo { message })
            }
            "broadcast" => {
                let message = msg
                    .payload
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Ok(ClientAction::Broadcast { message })
            }
            "get_connections" => Ok(ClientAction::GetConnections),
            _ => Err(format!("Unknown action: {}", msg.action)),
        }
    }
}

/// 服务器发送的消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMessage {
    /// 对应请求的 id（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// 消息类型
    #[serde(rename = "type")]
    pub msg_type: ServerMessageType,
    /// 消息数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// 错误信息（仅在 type 为 error 时使用）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 服务器消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerMessageType {
    Response,
    Error,
    Notification,
}

impl ServerMessage {
    /// 创建响应消息
    pub fn response(id: String, data: serde_json::Value) -> Self {
        Self {
            id: Some(id),
            msg_type: ServerMessageType::Response,
            data: Some(data),
            error: None,
        }
    }

    /// 创建错误消息
    pub fn error(id: Option<String>, error: String) -> Self {
        Self {
            id,
            msg_type: ServerMessageType::Error,
            data: None,
            error: Some(error),
        }
    }

    /// 创建通知消息
    pub fn notification(data: serde_json::Value) -> Self {
        Self {
            id: None,
            msg_type: ServerMessageType::Notification,
            data: Some(data),
            error: None,
        }
    }

    /// 创建欢迎消息
    pub fn welcome(connection_id: ConnectionId) -> Self {
        Self::notification(serde_json::json!({
            "event": "connected",
            "connection_id": connection_id.to_string(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_deserialization() {
        let json = r#"{
            "id": "123",
            "type": "request",
            "action": "echo",
            "payload": {"message": "hello"}
        }"#;

        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.id, "123");
        assert_eq!(msg.action, "echo");
    }

    #[test]
    fn test_server_message_serialization() {
        let msg = ServerMessage::response("123".to_string(), serde_json::json!({"result": "ok"}));

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"response\""));
        assert!(json.contains("\"id\":\"123\""));
    }

    #[test]
    fn test_client_action_parsing() {
        let msg = ClientMessage {
            id: "1".to_string(),
            msg_type: "request".to_string(),
            action: "echo".to_string(),
            payload: serde_json::json!({"message": "test"}),
        };

        let action = ClientAction::from_message(&msg).unwrap();
        match action {
            ClientAction::Echo { message } => assert_eq!(message, "test"),
            _ => panic!("Expected Echo action"),
        }
    }
}
