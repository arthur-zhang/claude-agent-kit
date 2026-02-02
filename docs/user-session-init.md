# UserSessionInit 协议

## 概述

`UserSessionInit` 是一个新的客户端消息类型，用于在 WebSocket 连接建立后初始化 Claude Agent 会话。这个消息允许客户端配置工作目录、模型、权限模式等参数。

## 设计动机

在之前的实现中，`ClaudeAgentOptions` 在 WebSocket 连接建立时就被硬编码创建，客户端无法动态配置这些参数。新的两阶段初始化流程解决了这个问题：

1. **阶段 1**: WebSocket 连接建立，服务器等待 `UserSessionInit` 消息
2. **阶段 2**: 收到 `UserSessionInit` 后，服务器使用配置参数创建 `ClaudeClient` 并开始会话

## 消息格式

### 客户端 -> 服务器: UserSessionInit

```json
{
  "type": "user_session_init",
  "id": "init-1234567890",
  "session_id": "session-abc123",
  "cwd": "/path/to/working/directory",
  "model": "claude-sonnet-4",
  "permission_mode": "manual",
  "max_turns": 50,
  "max_budget_usd": 1.0,
  "user": "username"
}
```

#### 字段说明

| 字段 | 类型 | 必选 | 说明 |
|------|------|------|------|
| `type` | string | ✅ | 固定值 `"user_session_init"` |
| `id` | string | ✅ | 消息唯一标识符 |
| `session_id` | string | ✅ | 会话 ID |
| `cwd` | string | ✅ | 工作目录路径 |
| `model` | string | ❌ | 模型名称（如 `"claude-sonnet-4"`） |
| `permission_mode` | string | ❌ | 权限模式：`"auto"`, `"manual"`, `"bypass"` |
| `max_turns` | number | ❌ | 最大轮次限制 |
| `max_budget_usd` | number | ❌ | 最大预算（美元） |
| `user` | string | ❌ | 用户标识符 |

### 服务器 -> 客户端: SessionInit

初始化成功后，服务器会发送 `SessionInit` 事件：

```json
{
  "type": "session_init",
  "id": "evt-1234567890",
  "session_id": "session-abc123",
  "cwd": "/path/to/working/directory",
  "model": "claude-sonnet-4",
  "tools": ["Task", "Bash", "Read", "Write"],
  "mcp_servers": [],
  "permissionMode": "manual",
  "agents": ["Bash", "Explore"],
  "skills": []
}
```

### 服务器 -> 客户端: Error

如果初始化失败，服务器会发送错误事件并关闭连接：

```json
{
  "type": "error",
  "id": "err-1234567890",
  "session_id": "session-abc123",
  "message": "Timeout waiting for UserSessionInit message",
  "is_fatal": true
}
```

## 错误处理

### 超时错误

如果客户端在连接后 30 秒内未发送 `UserSessionInit` 消息，服务器会：
1. 发送 `Error` 事件（`is_fatal: true`）
2. 关闭 WebSocket 连接

### 无效消息

如果服务器收到非 `UserSessionInit` 的消息，会：
1. 发送 `Error` 事件说明期望的消息类型
2. 关闭连接

### 客户端初始化失败

如果 `ClaudeClient` 初始化失败（如无法连接到 Claude API），服务器会：
1. 发送 `Error` 事件说明失败原因
2. 关闭连接

## 使用示例

### JavaScript/Node.js

```javascript
const WebSocket = require('ws');

const sessionId = 'my-session-123';
const ws = new WebSocket(`ws://localhost:3000/ws?session_id=${sessionId}`);

ws.on('open', () => {
  // 连接建立后立即发送 UserSessionInit
  const initMessage = {
    type: 'user_session_init',
    id: `init-${Date.now()}`,
    session_id: sessionId,
    cwd: process.cwd(),
    model: 'claude-sonnet-4',
    permission_mode: 'manual',
    max_turns: 50,
    max_budget_usd: 1.0,
    user: 'my-username'
  };

  ws.send(JSON.stringify(initMessage));
});

ws.on('message', (data) => {
  const message = JSON.parse(data.toString());

  if (message.type === 'session_init') {
    console.log('Session initialized!');
    // 现在可以发送用户消息了
    sendUserMessage('Hello, Claude!');
  } else if (message.type === 'error' && message.is_fatal) {
    console.error('Fatal error:', message.message);
    ws.close();
  }
});
```

### TypeScript/React

```typescript
import { useWebSocket } from './hooks/useWebSocket';

function App() {
  const {
    isConnected,
    messages,
    connect,
    sendMessage
  } = useWebSocket({
    url: 'ws://localhost:3000/ws',
    sessionId: 'my-session',
    cwd: '/path/to/project',
    model: 'claude-sonnet-4',
    permissionMode: 'manual'
  });

  useEffect(() => {
    connect();
  }, []);

  // UserSessionInit 会在连接建立后自动发送
}
```

## 时序图

```
Client                          Server
  |                               |
  |-- WebSocket Connect --------->|
  |<-- Connection Established ----|
  |                               |
  |-- UserSessionInit ----------->|
  |                               | (验证参数)
  |                               | (创建 ClaudeAgentOptions)
  |                               | (创建 ClaudeClient)
  |                               | (连接到 Claude API)
  |<-- SessionInit Event ---------|
  |                               |
  |-- UserMessage --------------->|
  |<-- AssistantMessage ----------|
  |                               |
```

## 权限模式映射

客户端的 `permission_mode` 会被映射到 SDK 的权限模式：

| 协议值 | SDK 值 | 说明 |
|--------|--------|------|
| `"auto"` | `Default` | 自动处理权限 |
| `"manual"` | `Default` | 手动处理权限（默认） |
| `"bypass"` | `BypassPermissions` | 绕过权限检查 |

## 默认值

如果客户端未提供可选参数，将使用以下默认值：

- `model`: 由 SDK 决定（通常是 `claude-sonnet-4`）
- `permission_mode`: `"manual"`
- `max_turns`: 无限制
- `max_budget_usd`: 无限制
- `user`: 无

## 测试

运行测试脚本验证功能：

```bash
# 启动服务器
cd websocket
cargo run

# 在另一个终端运行测试
node test-user-session-init.js
```

预期输出：
```
🧪 Testing UserSessionInit flow...

✅ WebSocket connected
📤 Sending UserSessionInit: {...}
📥 Received: session_init
✅ Session initialized successfully!
   Session ID: test-1234567890
   CWD: /path/to/project
   Model: claude-sonnet-4
   Tools: 15

📤 Sending test message...
💬 Assistant: Hello! I'm Claude...
✅ Turn completed
   Tokens used: 150

✅ Test completed successfully!
🔌 WebSocket disconnected
```

## 向后兼容性

这个改动**不向后兼容**。所有客户端必须在连接后发送 `UserSessionInit` 消息，否则连接会在 30 秒后超时。

## 实现细节

### 服务器端

- **文件**: `websocket/src/server.rs`
- **关键函数**:
  - `handle_socket()`: 主处理函数，实现两阶段初始化
  - `wait_for_init_message()`: 等待并解析 `UserSessionInit` 消息
  - `build_agent_options()`: 从初始化数据构建 `ClaudeAgentOptions`
  - `send_error_and_close()`: 发送错误并关闭连接

### 协议定义

- **文件**: `websocket/src/protocol/events.rs`
- **类型**: `ClientMessage::UserSessionInit`

### 前端

- **文件**:
  - `websocket/frontend/src/types.ts`: TypeScript 类型定义
  - `websocket/frontend/src/hooks/useWebSocket.ts`: React Hook 实现

## 相关文档

- [WebSocket 协议规范](./websocket-protocol.md)
- [统一事件系统](./unified-events.md)
- [权限系统](./permissions.md)
