use tracing::{debug, error};

use crate::connection::ConnectionManager;
use crate::message::{ClientAction, ClientMessage, ConnectionId, ServerMessage};

/// 处理客户端消息
pub async fn handle_message(
    manager: &ConnectionManager,
    connection_id: ConnectionId,
    msg: ClientMessage,
) -> ServerMessage {
    debug!("Handling message from {}: {:?}", connection_id, msg);

    // 解析客户端操作
    let action = match ClientAction::from_message(&msg) {
        Ok(action) => action,
        Err(e) => {
            error!("Failed to parse action: {}", e);
            return ServerMessage::error(Some(msg.id), e);
        }
    };

    // 处理不同的操作
    match action {
        ClientAction::Echo { message } => handle_echo(msg.id, message),
        ClientAction::Broadcast { message } => {
            handle_broadcast(manager, connection_id, msg.id, message).await
        }
        ClientAction::GetConnections => handle_get_connections(manager, msg.id).await,
    }
}

/// 处理 echo 操作
fn handle_echo(msg_id: String, message: String) -> ServerMessage {
    debug!("Echo: {}", message);
    ServerMessage::response(
        msg_id,
        serde_json::json!({
            "echo": message
        }),
    )
}

/// 处理 broadcast 操作
async fn handle_broadcast(
    manager: &ConnectionManager,
    sender_id: ConnectionId,
    msg_id: String,
    message: String,
) -> ServerMessage {
    debug!("Broadcasting message from {}: {}", sender_id, message);

    // 创建广播通知
    let notification = ServerMessage::notification(serde_json::json!({
        "event": "broadcast",
        "from": sender_id.to_string(),
        "message": message
    }));

    // 广播给所有其他连接
    manager.broadcast_except(&sender_id, notification).await;

    // 返回成功响应给发送者
    ServerMessage::response(
        msg_id,
        serde_json::json!({
            "status": "broadcasted"
        }),
    )
}

/// 处理获取连接数操作
async fn handle_get_connections(manager: &ConnectionManager, msg_id: String) -> ServerMessage {
    let count = manager.connection_count().await;
    let ids = manager.get_connection_ids().await;

    debug!("Current connections: {}", count);

    ServerMessage::response(
        msg_id,
        serde_json::json!({
            "count": count,
            "connections": ids.iter().map(|id| id.to_string()).collect::<Vec<_>>()
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_echo() {
        let msg_id = "test-123".to_string();
        let message = "Hello, World!".to_string();

        let response = handle_echo(msg_id.clone(), message.clone());

        assert_eq!(response.id, Some(msg_id));
        assert!(matches!(
            response.msg_type,
            crate::message::ServerMessageType::Response
        ));

        let data = response.data.unwrap();
        assert_eq!(data["echo"], message);
    }

    #[tokio::test]
    async fn test_handle_get_connections() {
        let manager = ConnectionManager::new();
        let (tx1, _rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, _rx2) = tokio::sync::mpsc::unbounded_channel();

        let id1 = ConnectionId::new_v4();
        let id2 = ConnectionId::new_v4();

        manager.add_connection(id1, tx1).await;
        manager.add_connection(id2, tx2).await;

        let response = handle_get_connections(&manager, "test-123".to_string()).await;

        let data = response.data.unwrap();
        assert_eq!(data["count"], 2);
    }

    #[tokio::test]
    async fn test_handle_broadcast() {
        let manager = ConnectionManager::new();
        let (tx1, mut rx1) = tokio::sync::mpsc::unbounded_channel();
        let (tx2, mut rx2) = tokio::sync::mpsc::unbounded_channel();

        let sender_id = ConnectionId::new_v4();
        let receiver_id = ConnectionId::new_v4();

        manager.add_connection(sender_id, tx1).await;
        manager.add_connection(receiver_id, tx2).await;

        let response = handle_broadcast(
            &manager,
            sender_id,
            "test-123".to_string(),
            "Hello everyone!".to_string(),
        )
        .await;

        // 发送者应该收到成功响应
        assert!(matches!(
            response.msg_type,
            crate::message::ServerMessageType::Response
        ));

        // 接收者应该收到广播通知
        let notification = rx2.recv().await.unwrap();
        assert!(matches!(
            notification.msg_type,
            crate::message::ServerMessageType::Notification
        ));

        // 发送者不应该收到自己的广播
        assert!(rx1.try_recv().is_err());
    }
}
