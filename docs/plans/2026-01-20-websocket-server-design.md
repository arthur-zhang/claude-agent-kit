# WebSocket 服务器设计文档

## 项目概述

为网页版 Claude Code 构建一个高性能 WebSocket 服务器，支持双向实时通信。

**核心需求**：
- 双向实时通信
- JSON 消息格式
- 支持 100+ 并发连接
- 暂不集成 Claude API（先构建基础框架）

**技术选型**：Axum + Tokio-Tungstenite

## 整体架构

采用三层架构设计：

### 1. HTTP 层 (Axum Router)
- 处理 WebSocket 握手升级
- 提供健康检查端点 `/health`
- 提供 WebSocket 端点 `/ws`

### 2. 连接管理层 (Connection Manager)
- 维护所有活跃的 WebSocket 连接
- 使用 `Arc<RwLock<HashMap<ConnectionId, Sender>>>` 存储连接
- 提供广播、单播、多播功能
- 处理连接的添加和移除

### 3. 消息处理层 (Message Handler)
- 解析 JSON 消息
- 路由到对应的处理器
- 构造响应消息

### 并发模型
- 每个 WebSocket 连接运行在独立的 Tokio 任务中
- 使用 `mpsc` 通道进行任务间通信
- 连接管理器使用读写锁保证线程安全

## 数据结构和消息格式

### 消息协议

**客户端发送的消息**：
```json
{
  "id": "uuid",           // 消息唯一标识
  "type": "request",      // 消息类型
  "action": "echo",       // 具体操作
  "payload": {}           // 业务数据
}
```

**服务器响应的消息**：
```json
{
  "id": "uuid",           // 对应请求的 id
  "type": "response",     // 或 "error", "notification"
  "data": {}              // 响应数据
}
```

### 核心数据结构

1. **ConnectionId**: 使用 UUID 唯一标识每个连接

2. **ClientMessage**: 客户端消息的枚举类型
   - Echo: 回显消息（测试用）
   - Broadcast: 广播消息给所有连接
   - GetConnections: 获取当前连接数

3. **ServerMessage**: 服务器消息的枚举类型
   - Response: 正常响应
   - Error: 错误响应
   - Notification: 服务器主动推送

4. **ConnectionManager**: 管理所有连接的状态
   ```rust
   struct ConnectionManager {
       connections: Arc<RwLock<HashMap<ConnectionId, mpsc::Sender<ServerMessage>>>>
   }
   ```

   提供方法：
   - `add_connection(id, sender)`
   - `remove_connection(id)`
   - `send_to(id, message)`
   - `broadcast(message)`

### 初始支持的操作
- `echo`: 回显消息（测试用）
- `broadcast`: 广播消息给所有连接
- `get_connections`: 获取当前连接数

## 连接生命周期和错误处理

### 连接生命周期

**1. 连接建立**：
- 客户端发起 WebSocket 握手
- 服务器升级连接，生成 ConnectionId
- 创建 mpsc 通道用于发送消息
- 将连接注册到 ConnectionManager
- 发送欢迎消息（包含 connection_id）

**2. 消息处理循环**：
- 同时监听两个源：WebSocket 接收和 mpsc 通道
- 使用 `tokio::select!` 处理并发事件
- 接收到客户端消息时解析并处理
- 接收到通道消息时发送给客户端

**3. 连接关闭**：
- 检测到连接断开或错误
- 从 ConnectionManager 移除连接
- 清理资源，关闭通道

### 错误处理策略

- **消息解析错误**: 返回 Error 消息，不断开连接
- **未知操作**: 返回 "unknown action" 错误
- **网络错误**: 记录日志，优雅关闭连接
- **内部错误**: 记录详细日志，返回通用错误消息给客户端

### 日志记录
使用 `tracing` 库记录关键事件：
- 连接建立和断开
- 消息收发
- 错误信息

## 项目结构

```
websocket/
├── Cargo.toml
└── src/
    ├── lib.rs              // 库入口，导出公共 API
    ├── main.rs             // 可执行文件入口
    ├── server.rs           // Axum 服务器设置
    ├── connection.rs       // ConnectionManager 实现
    ├── message.rs          // 消息类型定义和序列化
    ├── handler.rs          // 消息处理逻辑
    └── error.rs            // 错误类型定义
```

## 依赖项

```toml
[dependencies]
axum = { version = "0.7", features = ["ws"] }
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = "0.24"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

## 配置

- **监听地址**: 默认 `0.0.0.0:3000`（通过环境变量 `WEBSOCKET_ADDR` 可配置）
- **最大消息大小**: 16MB（可配置）
- **心跳间隔**: 30秒（可选功能，后续添加）

## 测试策略

1. **单元测试**:
   - 消息序列化/反序列化
   - ConnectionManager 的各项操作

2. **集成测试**:
   - 使用 `tokio-tungstenite` 客户端测试完整流程
   - 测试并发连接
   - 测试广播功能

3. **测试工具**:
   - 提供简单的 HTML 测试页面
   - 可在浏览器中快速测试 WebSocket 连接

## 后续扩展方向

- 身份认证机制
- 房间/频道管理
- 消息持久化
- 心跳检测和自动重连
- 集成 Claude API
- 性能监控和指标收集
