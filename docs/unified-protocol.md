# 统一 WebSocket 协议规范

基于 conduit 的事件系统重新设计的 WebSocket 通信协议。

## 概述

本协议提供了一个统一的事件驱动架构，用于前端和 WebSocket 服务器之间的通信。它结合了：

- **Conduit 的细粒度事件系统**：提供详细的执行状态跟踪
- **流式消息支持**：实时传输 AI 响应和工具执行结果
- **权限管理**：灵活的工具执行权限控制
- **上下文窗口跟踪**：实时监控 token 使用情况

## 架构

```
┌─────────────┐                    ┌──────────────┐                    ┌─────────────┐
│   前端      │ ◄──WebSocket──────► │ WebSocket    │ ◄──────────────► │ Claude      │
│   Client    │                    │ Server       │                    │ Agent SDK   │
└─────────────┘                    └──────────────┘                    └─────────────┘
      │                                    │                                   │
      │ ClientMessage                      │ AgentEvent                        │
      ├───────────────────────────────────►│                                   │
      │                                    ├──────────────────────────────────►│
      │                                    │                                   │
      │ AgentEvent                         │ SDK Events                        │
      │◄───────────────────────────────────┤                                   │
      │                                    │◄──────────────────────────────────┤
```

## 消息类型

### 1. 服务器到客户端事件 (AgentEvent)

所有事件都包含以下基础字段：
- `type`: 事件类型（snake_case）
- `id`: 唯一事件 ID
- `session_id`: 会话 ID

#### 1.1 会话生命周期事件

**SessionInit** - 会话初始化
```json
{
  "type": "session_init",
  "id": "evt-123",
  "session_id": "session-456",
  "model": "claude-sonnet-4"
}
```

**SessionInfo** - 会话状态更新
```json
{
  "type": "session_info",
  "id": "evt-124",
  "session_id": "session-456",
  "status": "active"  // active | paused | completed | error
}
```

#### 1.2 Turn 生命周期事件

**TurnStarted** - 开始新的对话轮次
```json
{
  "type": "turn_started",
  "id": "evt-125",
  "session_id": "session-456"
}
```

**TurnCompleted** - 对话轮次完成
```json
{
  "type": "turn_completed",
  "id": "evt-126",
  "session_id": "session-456",
  "usage": {
    "input_tokens": 1000,
    "output_tokens": 500,
    "cached_tokens": 200,
    "total_tokens": 1500
  }
}
```

**TurnFailed** - 对话轮次失败
```json
{
  "type": "turn_failed",
  "id": "evt-127",
  "session_id": "session-456",
  "error": "API rate limit exceeded"
}
```

#### 1.3 助手消息事件

**AssistantMessage** - 助手文本消息（流式）
```json
{
  "type": "assistant_message",
  "id": "evt-128",
  "session_id": "session-456",
  "text": "Let me help you with that...",
  "is_final": false
}
```

**AssistantReasoning** - 助手推理过程（流式）
```json
{
  "type": "assistant_reasoning",
  "id": "evt-129",
  "session_id": "session-456",
  "text": "First, I need to check the file structure..."
}
```

#### 1.4 工具执行事件

**ToolStarted** - 工具开始执行
```json
{
  "type": "tool_started",
  "id": "evt-130",
  "session_id": "session-456",
  "tool_name": "Bash",
  "tool_id": "tool-789",
  "arguments": {
    "command": "ls -la"
  }
}
```

**ToolCompleted** - 工具执行完成
```json
{
  "type": "tool_completed",
  "id": "evt-131",
  "session_id": "session-456",
  "tool_id": "tool-789",
  "success": true,
  "result": "total 48\ndrwxr-xr-x  12 user  staff  384 Jan 25 10:00 .",
  "error": null
}
```

#### 1.5 权限控制事件

**ControlRequest** - 请求执行权限
```json
{
  "type": "control_request",
  "id": "evt-132",
  "session_id": "session-456",
  "request_id": "req-001",
  "tool_name": "Bash",
  "tool_use_id": "tool-789",
  "input": {
    "command": "rm -rf /tmp/cache"
  },
  "context": {
    "description": "Delete temporary cache files",
    "risk_level": "medium"  // low | medium | high
  }
}
```

#### 1.6 文件操作事件

**FileChanged** - 文件变更通知
```json
{
  "type": "file_changed",
  "id": "evt-133",
  "session_id": "session-456",
  "path": "/path/to/file.rs",
  "operation": "update"  // create | update | delete
}
```

#### 1.7 命令输出事件

**CommandOutput** - 命令执行输出（流式）
```json
{
  "type": "command_output",
  "id": "evt-134",
  "session_id": "session-456",
  "command": "cargo build",
  "output": "Compiling websocket v0.1.0...",
  "exit_code": null,
  "is_streaming": true
}
```

#### 1.8 Token 使用事件

**TokenUsage** - Token 使用情况更新
```json
{
  "type": "token_usage",
  "id": "evt-135",
  "session_id": "session-456",
  "usage": {
    "input_tokens": 5000,
    "output_tokens": 2000,
    "cached_tokens": 1000,
    "total_tokens": 7000
  },
  "context_window": 200000,
  "usage_percent": 0.035
}
```

**ContextCompaction** - 上下文压缩事件
```json
{
  "type": "context_compaction",
  "id": "evt-136",
  "session_id": "session-456",
  "reason": "Context window approaching limit",
  "tokens_before": 180000,
  "tokens_after": 90000
}
```

#### 1.9 交互式事件

**AskUserQuestion** - 询问用户问题
```json
{
  "type": "ask_user_question",
  "id": "evt-137",
  "session_id": "session-456",
  "tool_id": "tool-890",
  "questions": [
    {
      "header": "Approach",
      "question": "Which implementation approach would you prefer?",
      "options": [
        {
          "label": "Option A: Fast but less flexible",
          "description": "Uses hardcoded values for quick implementation"
        },
        {
          "label": "Option B: Flexible but slower",
          "description": "Uses configuration-based approach"
        }
      ],
      "multiSelect": false
    }
  ]
}
```

**ExitPlanMode** - 退出计划模式
```json
{
  "type": "exit_plan_mode",
  "id": "evt-138",
  "session_id": "session-456",
  "tool_id": "tool-891",
  "plan_file_path": "/path/to/plan.md"
}
```

#### 1.10 错误和心跳事件

**Error** - 错误事件
```json
{
  "type": "error",
  "id": "evt-139",
  "session_id": "session-456",
  "message": "Failed to execute tool: permission denied",
  "is_fatal": false
}
```

**Heartbeat** - 心跳保活
```json
{
  "type": "heartbeat",
  "id": "evt-140",
  "session_id": "session-456",
  "timestamp": 1706169600
}
```

### 2. 客户端到服务器消息 (ClientMessage)

所有消息都包含以下基础字段：
- `type`: 消息类型（snake_case）
- `id`: 唯一消息 ID
- `session_id`: 会话 ID

#### 2.1 会话控制消息

**SessionStart** - 启动会话
```json
{
  "type": "session_start",
  "id": "msg-001",
  "session_id": "session-456",
  "permission_mode": "manual",  // auto | manual | bypass
  "max_turns": 10,
  "metadata": {
    "user_id": "user-123",
    "project": "my-project"
  }
}
```

**SessionEnd** - 结束会话
```json
{
  "type": "session_end",
  "id": "msg-002",
  "session_id": "session-456"
}
```

**SetPermissionMode** - 动态修改权限模式
```json
{
  "type": "set_permission_mode",
  "id": "msg-003",
  "session_id": "session-456",
  "mode": "auto"
}
```

#### 2.2 用户交互消息

**UserMessage** - 用户消息
```json
{
  "type": "user_message",
  "id": "msg-004",
  "session_id": "session-456",
  "content": "Please help me debug this code",
  "parent_tool_use_id": null
}
```

**PermissionResponse** - 权限响应
```json
{
  "type": "permission_response",
  "id": "msg-005",
  "session_id": "session-456",
  "request_id": "req-001",
  "decision": "allow",  // allow | deny | allow_always
  "explanation": "This operation is safe"
}
```

**UserQuestionResponse** - 用户问题回答
```json
{
  "type": "user_question_response",
  "id": "msg-006",
  "session_id": "session-456",
  "tool_id": "tool-890",
  "answers": [
    {
      "question_index": 0,
      "selected": ["Option A: Fast but less flexible"]
    }
  ]
}
```

**PlanApprovalResponse** - 计划批准响应
```json
{
  "type": "plan_approval_response",
  "id": "msg-007",
  "session_id": "session-456",
  "tool_id": "tool-891",
  "approved": true,
  "feedback": "Looks good, proceed with implementation"
}
```

#### 2.3 执行控制消息

**Interrupt** - 中断执行
```json
{
  "type": "interrupt",
  "id": "msg-008",
  "session_id": "session-456",
  "reason": "User requested stop"
}
```

**Resume** - 恢复执行
```json
{
  "type": "resume",
  "id": "msg-009",
  "session_id": "session-456"
}
```

**Cancel** - 取消特定请求
```json
{
  "type": "cancel",
  "id": "msg-010",
  "session_id": "session-456",
  "target_id": "msg-004"
}
```

## 上下文窗口状态跟踪

客户端可以使用 `ContextWindowState` 来跟踪上下文使用情况：

```typescript
interface ContextWindowState {
  current_tokens: number;
  max_tokens: number;
  has_compacted: boolean;
  compaction_count: number;
}

enum ContextWarningLevel {
  Normal,    // < 80%
  Medium,    // 80-89%
  High,      // 90-94%
  Critical   // >= 95%
}
```

## 典型消息流程

### 场景 1: 简单查询

```
Client → Server: UserMessage
Server → Client: TurnStarted
Server → Client: AssistantMessage (streaming)
Server → Client: AssistantMessage (is_final: true)
Server → Client: TurnCompleted (with token usage)
```

### 场景 2: 工具执行（需要权限）

```
Client → Server: UserMessage
Server → Client: TurnStarted
Server → Client: AssistantMessage
Server → Client: ToolStarted
Server → Client: ControlRequest
Client → Server: PermissionResponse (allow)
Server → Client: CommandOutput (streaming)
Server → Client: ToolCompleted
Server → Client: AssistantMessage (final response)
Server → Client: TurnCompleted
```

### 场景 3: 交互式问答

```
Client → Server: UserMessage
Server → Client: TurnStarted
Server → Client: AssistantReasoning
Server → Client: AskUserQuestion
Client → Server: UserQuestionResponse
Server → Client: AssistantMessage
Server → Client: TurnCompleted
```

### 场景 4: 上下文压缩

```
Server → Client: TokenUsage (usage_percent: 0.92)
Server → Client: ContextCompaction
Server → Client: TokenUsage (usage_percent: 0.45)
```

## 实现建议

### 前端实现

1. **事件监听器**：为每种 `AgentEvent` 类型注册处理器
2. **状态管理**：维护会话状态、token 使用情况、待处理的权限请求
3. **UI 更新**：
   - 流式显示 `AssistantMessage` 和 `CommandOutput`
   - 显示 token 使用进度条（基于 `TokenUsage` 事件）
   - 弹出权限确认对话框（`ControlRequest`）
   - 显示交互式问题表单（`AskUserQuestion`）

### 后端实现

1. **事件转换**：使用 `event_converter` 模块创建标准化事件
2. **会话管理**：跟踪每个会话的状态和配置
3. **权限处理**：根据 `permission_mode` 自动批准或请求用户确认
4. **错误处理**：捕获异常并发送 `Error` 事件

## 优势

1. **细粒度跟踪**：每个执行阶段都有对应的事件
2. **实时反馈**：流式消息提供即时的用户体验
3. **灵活权限**：支持自动、手动和绕过三种模式
4. **资源监控**：实时跟踪 token 使用和上下文窗口状态
5. **向前兼容**：`Raw` 事件类型支持未来扩展
6. **类型安全**：强类型定义减少运行时错误

## 迁移指南

从旧协议迁移到新协议：

### 旧协议 → 新协议映射

| 旧事件 | 新事件 |
|--------|--------|
| `assistant_message_start` | `TurnStarted` |
| `assistant_message_delta` (text) | `AssistantMessage` |
| `assistant_message_delta` (thinking) | `AssistantReasoning` |
| `assistant_message_complete` | `TurnCompleted` |
| `tool_use` | `ToolStarted` |
| `tool_result` | `ToolCompleted` |
| `permission_request` | `ControlRequest` |
| `error` | `Error` |

### 代码示例

**Rust (服务器端)**
```rust
use websocket::protocol::event_converter::*;
use websocket::protocol::events::*;

// 创建事件
let event = create_assistant_message(
    "session-123",
    "Hello, how can I help?".to_string(),
    false
);

// 序列化并发送
let json = serde_json::to_string(&event)?;
ws_sender.send(Message::Text(json)).await?;
```

**TypeScript (客户端)**
```typescript
interface AgentEventHandler {
  onTurnStarted: (event: TurnStartedEvent) => void;
  onAssistantMessage: (event: AssistantMessageEvent) => void;
  onToolStarted: (event: ToolStartedEvent) => void;
  onTokenUsage: (event: TokenUsageEvent) => void;
  // ... 其他事件处理器
}

class WebSocketClient {
  private handlers: AgentEventHandler;

  handleMessage(data: string) {
    const event = JSON.parse(data);

    switch (event.type) {
      case 'turn_started':
        this.handlers.onTurnStarted(event);
        break;
      case 'assistant_message':
        this.handlers.onAssistantMessage(event);
        break;
      // ... 其他事件类型
    }
  }
}
```

## 参考

- [Conduit Events 源码](../conduit/src/agent/events.rs)
- [WebSocket Protocol Events](../websocket/src/protocol/events.rs)
- [Event Converter](../websocket/src/protocol/event_converter.rs)
