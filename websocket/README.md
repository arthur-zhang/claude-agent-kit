# WebSocket 服务器

一个基于 Rust + Axum 的高性能 WebSocket 服务器，支持双向实时通信。

## 功能特性

- ✅ 双向实时通信
- ✅ JSON 消息格式
- ✅ 支持 100+ 并发连接
- ✅ 连接管理（添加、移除、广播）
- ✅ 内置测试页面
- ✅ 完整的单元测试

## 技术栈

- **Axum** - Web 框架
- **Tokio** - 异步运行时
- **Serde** - JSON 序列化
- **Tracing** - 日志记录

## 快速开始

### 1. 运行服务器

```bash
cd websocket
cargo run
```

服务器将在 `http://0.0.0.0:3000` 启动。

### 2. 访问测试页面

在浏览器中打开：`http://localhost:3000`

### 3. 自定义监听地址

```bash
WEBSOCKET_ADDR=127.0.0.1:8080 cargo run
```

## API 端点

- `GET /` - 测试页面
- `GET /health` - 健康检查
- `GET /ws` - WebSocket 连接端点

## 消息协议

### 客户端发送消息

```json
{
  "id": "msg-123",
  "type": "request",
  "action": "echo",
  "payload": {
    "message": "Hello"
  }
}
```

### 服务器响应消息

```json
{
  "id": "msg-123",
  "type": "response",
  "data": {
    "echo": "Hello"
  }
}
```

### 服务器通知消息

```json
{
  "type": "notification",
  "data": {
    "event": "connected",
    "connection_id": "uuid"
  }
}
```

## 支持的操作

### 1. Echo（回显）

```json
{
  "id": "1",
  "type": "request",
  "action": "echo",
  "payload": {
    "message": "test"
  }
}
```

### 2. Broadcast（广播）

```json
{
  "id": "2",
  "type": "request",
  "action": "broadcast",
  "payload": {
    "message": "Hello everyone!"
  }
}
```

### 3. Get Connections（获取连接数）

```json
{
  "id": "3",
  "type": "request",
  "action": "get_connections",
  "payload": {}
}
```

## 开发

### 运行测试

```bash
cargo test
```

### 构建 Release 版本

```bash
cargo build --release
```

### 查看日志

```bash
RUST_LOG=debug cargo run
```

## 项目结构

```
websocket/
├── src/
│   ├── lib.rs          # 库入口
│   ├── main.rs         # 可执行文件入口
│   ├── server.rs       # Axum 服务器
│   ├── connection.rs   # 连接管理器
│   ├── message.rs      # 消息类型定义
│   ├── handler.rs      # 消息处理逻辑
│   └── error.rs        # 错误类型
├── static/
│   └── index.html      # 测试页面
└── Cargo.toml
```

## 架构设计

详细的架构设计文档请参考：[docs/plans/2026-01-20-websocket-server-design.md](../docs/plans/2026-01-20-websocket-server-design.md)

## 后续扩展

- [ ] 身份认证机制
- [ ] 房间/频道管理
- [ ] 消息持久化
- [ ] 心跳检测和自动重连
- [ ] 集成 Claude API
- [ ] 性能监控和指标收集

## 许可证

MIT
