# WebSocket 统一事件系统 - 快速开始

## ✅ 已完成集成

新的统一事件系统现在已经完全集成到 WebSocket 服务器中！

## 快速验证

### 1. 启动服务器

```bash
cd websocket
cargo run
```

预期输出：
```
WebSocket server listening on: 127.0.0.1:3000
```

### 2. 运行测试脚本

**方式 A：使用 Node.js 测试脚本**

```bash
# 安装依赖
npm install ws

# 运行测试
node test-unified-events.js
```

**方式 B：使用 websocat**

```bash
# 安装 websocat
brew install websocat  # macOS
# 或
cargo install websocat

# 连接并测试
websocat ws://localhost:3000/ws?session_id=test-123
```

然后发送测试消息：
```json
{"type":"user_message","id":"msg-1","session_id":"test-123","content":"Hello!","parent_tool_use_id":null}
```

### 3. 预期结果

你应该看到以下事件序列：

```
1. session_init      - 会话初始化
2. turn_started      - Turn 开始
3. assistant_message - 助手响应（可能多次，流式）
4. token_usage       - Token 使用统计
5. turn_completed    - Turn 完成
```

## 事件类型说明

### 服务器 → 客户端事件

| 事件类型 | 说明 | 何时触发 |
|---------|------|---------|
| `session_init` | 会话初始化 | 连接建立后立即发送 |
| `turn_started` | Turn 开始 | 收到用户消息后 |
| `turn_completed` | Turn 完成 | AI 响应完成后 |
| `turn_failed` | Turn 失败 | 处理出错时 |
| `assistant_message` | 助手消息 | AI 生成文本时（流式） |
| `assistant_reasoning` | 助手推理 | AI 思考过程（thinking） |
| `tool_started` | 工具开始 | 工具开始执行 |
| `tool_completed` | 工具完成 | 工具执行完成 |
| `control_request` | 权限请求 | 需要用户批准工具执行 |
| `token_usage` | Token 使用 | 每次 API 调用后 |
| `context_compaction` | 上下文压缩 | 上下文窗口接近限制时 |
| `error` | 错误 | 发生错误时 |

### 客户端 → 服务器消息

| 消息类型 | 说明 | 何时发送 |
|---------|------|---------|
| `user_message` | 用户消息 | 用户输入时 |
| `permission_response` | 权限响应 | 响应 control_request |
| `session_start` | 会话开始 | 初始化会话配置 |
| `session_end` | 会话结束 | 关闭会话 |
| `interrupt` | 中断 | 停止当前执行 |
| `resume` | 恢复 | 恢复执行 |

## 代码示例

### JavaScript/TypeScript 客户端

```typescript
const ws = new WebSocket('ws://localhost:3000/ws?session_id=my-session');

// 处理接收到的事件
ws.onmessage = (event) => {
  const agentEvent = JSON.parse(event.data);

  switch (agentEvent.type) {
    case 'session_init':
      console.log('Session initialized:', agentEvent.session_id);
      break;

    case 'turn_started':
      showLoadingIndicator();
      break;

    case 'assistant_message':
      appendMessage(agentEvent.text);
      if (agentEvent.is_final) {
        hideLoadingIndicator();
      }
      break;

    case 'token_usage':
      updateTokenDisplay(agentEvent.usage);
      break;

    case 'control_request':
      showPermissionDialog(agentEvent);
      break;

    case 'error':
      showError(agentEvent.message);
      break;
  }
};

// 发送用户消息
function sendMessage(text) {
  ws.send(JSON.stringify({
    type: 'user_message',
    id: 'msg-' + Date.now(),
    session_id: 'my-session',
    content: text,
    parent_tool_use_id: null
  }));
}

// 响应权限请求
function approvePermission(requestId) {
  ws.send(JSON.stringify({
    type: 'permission_response',
    id: 'resp-' + Date.now(),
    session_id: 'my-session',
    request_id: requestId,
    decision: 'allow',
    explanation: 'User approved'
  }));
}
```

### Rust 服务器端

服务器端已经自动处理事件转换，你只需要：

```rust
// 在 server.rs 中已经配置好
use crate::session::event_handler::handle_session_with_events;

// 自动将 SDK 消息转换为统一事件
handle_session_with_events(
    ws_sender,
    ws_receiver,
    state,
    session_id,
    send_timeout_secs,
    client,
    approval_service,
).await?;
```

## Token 使用监控

新系统提供完整的 Token 使用监控：

```typescript
interface TokenUsage {
  input_tokens: number;
  output_tokens: number;
  cached_tokens: number;
  total_tokens: number;
}

// 监控上下文窗口
class ContextMonitor {
  private currentTokens = 0;
  private maxTokens = 200000;

  handleTokenUsage(event: TokenUsageEvent) {
    this.currentTokens = event.usage.total_tokens;

    if (event.context_window) {
      this.maxTokens = event.context_window;
    }

    const percent = this.currentTokens / this.maxTokens;

    if (percent >= 0.95) {
      showWarning('Context window nearly full!');
    } else if (percent >= 0.80) {
      showInfo('Context window at 80%');
    }

    updateProgressBar(percent);
  }
}
```

## 权限管理

处理工具执行权限请求：

```typescript
ws.onmessage = (event) => {
  const agentEvent = JSON.parse(event.data);

  if (agentEvent.type === 'control_request') {
    // 显示权限对话框
    const approved = await showPermissionDialog({
      toolName: agentEvent.tool_name,
      description: agentEvent.context.description,
      riskLevel: agentEvent.context.risk_level,
      input: agentEvent.input
    });

    // 发送响应
    ws.send(JSON.stringify({
      type: 'permission_response',
      id: 'resp-' + Date.now(),
      session_id: agentEvent.session_id,
      request_id: agentEvent.request_id,
      decision: approved ? 'allow' : 'deny',
      explanation: approved ? 'User approved' : 'User denied'
    }));
  }
};
```

## 调试

### 启用详细日志

```bash
RUST_LOG=websocket=debug cargo run
```

### 查看事件流

使用测试脚本可以看到完整的事件流：

```bash
node test-unified-events.js
```

输出示例：
```
📥 [session_init]
   Session ID: test-1234567890
   Model: N/A
────────────────────────────────────────

📥 [turn_started]
   🔄 Turn 开始
────────────────────────────────────────

📥 [assistant_message]
   💬 Hello! 2+2 equals 4.
   Final: true
────────────────────────────────────────

📥 [token_usage]
   📊 Token: 150
   使用率: 0.1%
────────────────────────────────────────

📥 [turn_completed]
   ✅ Turn 完成
   Token 使用: 150 (输入: 100, 输出: 50)
────────────────────────────────────────
```

## 故障排除

### 问题：连接失败

**解决方案：**
1. 确保服务器正在运行：`cargo run`
2. 检查端口 3000 是否被占用：`lsof -i :3000`
3. 检查防火墙设置

### 问题：没有收到事件

**解决方案：**
1. 检查 session_id 是否正确
2. 查看服务器日志：`RUST_LOG=debug cargo run`
3. 确认 WebSocket 连接状态

### 问题：事件格式错误

**解决方案：**
1. 确保使用最新版本的代码
2. 检查 JSON 格式是否正确
3. 参考文档中的示例

## 性能优化

### 1. 批量发送事件

```rust
// 未来可以实现事件批处理
let events = vec![event1, event2, event3];
send_events_batch(&ws_sender, &events).await?;
```

### 2. 事件过滤

```typescript
// 客户端可以订阅特定事件类型
const subscription = {
  type: 'subscribe',
  event_types: ['assistant_message', 'token_usage']
};
ws.send(JSON.stringify(subscription));
```

### 3. 压缩

对于大型事件，可以启用 WebSocket 压缩：

```rust
// 在 Axum 配置中启用
.layer(CompressionLayer::new())
```

## 下一步

1. **前端集成**：实现完整的 UI 组件
2. **监控仪表板**：创建实时事件监控页面
3. **性能测试**：压力测试和基准测试
4. **文档完善**：添加更多示例和最佳实践

## 相关文档

- [完整协议规范](../docs/unified-protocol.md)
- [集成验证](../docs/integration-verification.md)
- [重构总结](../docs/protocol-refactor-complete.md)

## 支持

如有问题，请查看：
- 服务器日志：`RUST_LOG=debug cargo run`
- 测试脚本输出：`node test-unified-events.js`
- 文档：`docs/` 目录

---

**状态：✅ 已完成并可用**

新的统一事件系统已经完全集成到 WebSocket 服务器中，所有测试通过，可以投入使用！
