# WebSocket Protocol Implementation Design

## Overview

本文档描述了 WebSocket 协议的实现设计，目标是创建一个作为 Claude Agent 前端代理的 WebSocket 服务器。

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    WebSocket Server                          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌──────────────┐    ┌───────────────┐  │
│  │  protocol/  │    │   session/   │    │  agent-sdk    │  │
│  │  types.rs   │←──→│  handler.rs  │←──→│  (existing)   │  │
│  │  (26种消息)  │    │  (消息路由)   │    │               │  │
│  └─────────────┘    └──────────────┘    └───────────────┘  │
│         ↑                  ↑                               │
│         │                  │                               │
│  ┌─────────────┐    ┌──────────────┐                       │
│  │  protocol/  │    │   session/   │                       │
│  │ converter.rs│    │   state.rs   │                       │
│  │  (格式转换)  │    │  (会话状态)   │                       │
│  └─────────────┘    └──────────────┘                       │
└─────────────────────────────────────────────────────────────┘
```

## Module Structure

### 1. protocol/types.rs

定义协议文档中的所有消息类型。

**ClientMessage (客户端 → 服务器):**
- `user_message` - 用户消息
- `permission_response` - 权限响应
- `session_start` - 会话开始
- `session_end` - 会话结束
- `interrupt` - 中断请求
- `resume` - 恢复执行
- `cancel` - 取消操作
- 等其他客户端消息类型

**ServerMessage (服务器 → 客户端):**
- `assistant_message_start` - 助手消息开始
- `assistant_message_delta` - 助手消息增量
- `assistant_message_complete` - 助手消息完成
- `tool_use` - 工具调用
- `tool_result` - 工具结果
- `permission_request` - 权限请求
- `result` - 最终结果
- `error` - 错误消息
- 等其他服务器消息类型

所有消息使用 `#[serde(tag = "type", rename_all = "snake_case")]` 实现扁平化的 JSON 格式。

### 2. protocol/converter.rs

实现 SDK 消息与协议消息的双向转换。

**SDK → Protocol:**
- `sdk::Message::Assistant` → `assistant_message_start` + 多个 `delta` + `tool_use` + `assistant_message_complete`
- `sdk::Message::Result` → `result`
- `sdk::Message::Stream` → `assistant_message_delta`
- `sdk::ContentBlock::ToolUse` → `tool_use`
- `sdk::ContentBlock::ToolResult` → `tool_result`

**Protocol → SDK:**
- `ClientMessage::UserMessage` → SDK query input
- `ClientMessage::PermissionResponse` → SDK permission decision

### 3. session/state.rs

会话状态管理。

```rust
pub struct SessionState {
    session_id: String,
    config: SessionConfig,
    status: SessionStatus,
    pending_permission: Option<PendingPermission>,
    message_id_counter: AtomicU64,
}

pub enum SessionStatus {
    Idle,
    Thinking,
    ExecutingTool,
    WaitingPermission,  // 等待权限响应
}

pub struct PendingPermission {
    request_id: String,
    tool_name: String,
    tool_input: Value,
    response_tx: oneshot::Sender<PermissionDecision>,
}
```

### 4. session/handler.rs

消息路由和处理逻辑。

**权限处理流程:**
1. 检测工具调用是否需要权限
2. 创建 `oneshot` channel 用于同步
3. 发送 `permission_request` 给前端
4. 等待前端响应（阻塞）
5. 根据决策执行或拒绝工具调用

**流式消息处理:**
- 维护 `current_message_id` 跟踪当前 assistant 消息
- 流式事件转换为 `delta` 消息
- 收到 `result` 消息时发送 `complete`

## Error Handling

- SDK 错误 → `ServerMessage::Error`
- 权限被拒绝 → `ServerMessage::Error` + 停止执行
- 网络超时 → `ServerMessage::Warning` + 重试逻辑
- 消息解析错误 → `ServerMessage::Error` + 关闭连接

## Implementation Phases

1. **Phase 1**: 实现 protocol/types.rs - 定义所有消息类型
2. **Phase 2**: 实现 protocol/converter.rs - 消息转换逻辑
3. **Phase 3**: 实现 session/state.rs - 会话状态管理
4. **Phase 4**: 重构 session/handler.rs - 整合所有组件
5. **Phase 5**: 测试和调试
