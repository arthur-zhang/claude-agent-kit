use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::error::{Result, WebSocketError};
use crate::protocol::types::ServerMessage;

/// 连接唯一标识符
pub type ConnectionId = Uuid;

/// 连接管理器，负责管理所有活跃的 WebSocket 连接
#[derive(Clone)]
pub struct ConnectionManager {
    /// 存储所有连接的发送通道
    connections: Arc<RwLock<HashMap<ConnectionId, mpsc::UnboundedSender<ServerMessage>>>>,
}

impl ConnectionManager {
    /// 创建新的连接管理器
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 添加新连接
    pub async fn add_connection(
        &self,
        id: ConnectionId,
        sender: mpsc::UnboundedSender<ServerMessage>,
    ) {
        let mut connections = self.connections.write().await;
        connections.insert(id, sender);
        info!("Connection added: {} (total: {})", id, connections.len());
    }

    /// 移除连接
    pub async fn remove_connection(&self, id: &ConnectionId) {
        let mut connections = self.connections.write().await;
        if connections.remove(id).is_some() {
            info!("Connection removed: {} (total: {})", id, connections.len());
        } else {
            warn!("Attempted to remove non-existent connection: {}", id);
        }
    }

    /// 发送消息给指定连接
    pub async fn send_to(&self, id: &ConnectionId, message: ServerMessage) -> Result<()> {
        let connections = self.connections.read().await;

        if let Some(sender) = connections.get(id) {
            sender.send(message).map_err(|e| {
                WebSocketError::InternalError(format!("Failed to send message: {}", e))
            })?;
            debug!("Message sent to connection: {}", id);
            Ok(())
        } else {
            Err(WebSocketError::ConnectionNotFound(id.to_string()))
        }
    }

    /// 广播消息给所有连接
    pub async fn broadcast(&self, message: ServerMessage) {
        let connections = self.connections.read().await;
        let count = connections.len();

        for (id, sender) in connections.iter() {
            if let Err(e) = sender.send(message.clone()) {
                warn!("Failed to broadcast to connection {}: {}", id, e);
            }
        }

        info!("Broadcast message to {} connections", count);
    }

    /// 广播消息给所有连接，排除指定连接
    pub async fn broadcast_except(&self, exclude_id: &ConnectionId, message: ServerMessage) {
        let connections = self.connections.read().await;
        let mut count = 0;

        for (id, sender) in connections.iter() {
            if id != exclude_id {
                if let Err(e) = sender.send(message.clone()) {
                    warn!("Failed to broadcast to connection {}: {}", id, e);
                } else {
                    count += 1;
                }
            }
        }

        debug!(
            "Broadcast message to {} connections (excluding {})",
            count, exclude_id
        );
    }

    /// 获取当前连接数
    pub async fn connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    /// 获取所有连接 ID
    pub async fn get_connection_ids(&self) -> Vec<ConnectionId> {
        let connections = self.connections.read().await;
        connections.keys().copied().collect()
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::types::{ResultSubtype, ServerMessage};

    #[tokio::test]
    async fn test_add_and_remove_connection() {
        let manager = ConnectionManager::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let id = ConnectionId::new_v4();

        manager.add_connection(id, tx).await;
        assert_eq!(manager.connection_count().await, 1);

        manager.remove_connection(&id).await;
        assert_eq!(manager.connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_send_to_connection() {
        let manager = ConnectionManager::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        let id = ConnectionId::new_v4();

        manager.add_connection(id, tx).await;

        let msg = ServerMessage::Result {
            id: "msg-1".to_string(),
            session_id: "sess-1".to_string(),
            subtype: ResultSubtype::Success,
            duration_ms: 100,
            duration_api_ms: 50,
            num_turns: 1,
            is_error: false,
            error: None,
            total_cost_usd: Some(0.01),
        };
        manager.send_to(&id, msg.clone()).await.unwrap();

        let received = rx.recv().await.unwrap();
        match received {
            ServerMessage::Result { subtype, .. } => {
                assert_eq!(subtype, ResultSubtype::Success);
            }
            _ => panic!("Expected Result message"),
        }
    }

    #[tokio::test]
    async fn test_broadcast() {
        let manager = ConnectionManager::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();
        let id1 = ConnectionId::new_v4();
        let id2 = ConnectionId::new_v4();

        manager.add_connection(id1, tx1).await;
        manager.add_connection(id2, tx2).await;

        let msg = ServerMessage::Warning {
            id: "warn-1".to_string(),
            session_id: "sess-1".to_string(),
            message: "This is a broadcast".to_string(),
        };
        manager.broadcast(msg).await;

        assert!(rx1.recv().await.is_some());
        assert!(rx2.recv().await.is_some());
    }
}
