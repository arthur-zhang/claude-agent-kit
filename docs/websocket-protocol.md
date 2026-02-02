# WebSocket 通信协议设计

## 概述

本文档定义了前端页面与 WebSocket 服务器之间的通信协议,用于实现可视化的 Claude Code 聊天界面。

**协议特性**:
- 基于 JSON 的双向通信
- 支持实时流式响应
- 支持工具权限管理
- 支持动态配置(model、permission mode)
- 支持中断操作

## 消息结构

所有消息都是 JSON 格式,包含一个 `type` 字段用于区分消息类型。

---

## 客户端 → 服务器消息

### 1. 用户消息 (User Message)

发送用户输入的文本消息给 Claude。

```json
{
  "type": "user_message",
  "session_id": "default",
  "message": {
    "content": "Hello, Claude! Can you help me?",
    "parent_tool_use_id": null
  },
  "uuid": "msg-123"
}
```

**字段说明**:
- `type`: 固定为 `"user_message"`
- `session_id`: 会话 ID,用于区分不同的对话上下文
- `message.content`: 用户输入的文本内容
- `message.parent_tool_use_id`: 可选,如果是回复工具结果,填写对应的 tool use ID
- `uuid`: 可选,消息的唯一标识符

---

### 2. 权限响应 (Permission Response)

响应服务器的工具使用权限请求。

```json
{
  "type": "permission_response",
  "request_id": "req-456",
  "decision": "allow",
  "updated_input": null,
  "updated_permissions": null
}
```

**字段说明**:
- `type`: 固定为 `"permission_response"`
- `request_id`: 对应权限请求的 ID
- `decision`: 决策,`"allow"` 或 `"deny"`
- `updated_input`: 可选,修改后的工具输入参数
- `updated_permissions`: 可选,权限更新配置数组

**拒绝示例**:
```json
{
  "type": "permission_response",
  "request_id": "req-456",
  "decision": "deny",
  "message": "不允许删除文件",
  "interrupt": false
}
```

---

### 3. 中断请求 (Interrupt Request)

中断当前正在执行的操作。

```json
{
  "type": "interrupt"
}
```

**字段说明**:
- `type`: 固定为 `"interrupt"`

---

### 4. 设置模型 (Set Model)

动态切换 Claude 使用的模型。

```json
{
  "type": "set_model",
  "model": "claude-opus-4"
}
```

**字段说明**:
- `type`: 固定为 `"set_model"`
- `model`: 模型名称,可选值:
  - `"claude-sonnet-4"` (默认)
  - `"claude-opus-4"`
  - `"claude-haiku-4"`

---

### 5. 设置权限模式 (Set Permission Mode)

切换权限模式。

```json
{
  "type": "set_permission_mode",
  "mode": "plan"
}
```

**字段说明**:
- `type`: 固定为 `"set_permission_mode"`
- `mode`: 权限模式,可选值:
  - `"default"`: 默认模式,每次询问
  - `"acceptEdits"`: 自动接受编辑操作
  - `"plan"`: 计划模式,需要确认执行计划
  - `"bypassPermissions"`: 跳过所有权限检查

---

### 6. Hook 回调响应 (Hook Callback Response)

响应 Hook 回调请求(用于高级集成)。

```json
{
  "type": "hook_callback_response",
  "callback_id": "hook-789",
  "output": {
    "continue": true,
    "hookSpecificOutput": {
      "hookEventName": "PreToolUse",
      "permissionDecision": "allow"
    }
  }
}
```

---

## 服务器 → 客户端消息

### 1. 助手消息 (Assistant Message)

Claude 的响应消息,包含文本、思考过程或工具调用。

#### 1.1 文本消息

```json
{
  "type": "assistant",
  "content": [
    {
      "type": "text",
      "text": "Hello! I'd be happy to help you."
    }
  ],
  "model": "claude-sonnet-4",
  "parent_tool_use_id": null
}
```

#### 1.2 思考消息

```json
{
  "type": "assistant",
  "content": [
    {
      "type": "thinking",
      "thinking": "Let me analyze the user's request...",
      "signature": "sig-abc123"
    }
  ],
  "model": "claude-sonnet-4"
}
```

#### 1.3 工具调用消息

```json
{
  "type": "assistant",
  "content": [
    {
      "type": "tool_use",
      "id": "toolu_01ABC123",
      "name": "Bash",
      "input": {
        "command": "ls -la",
        "description": "List files in current directory"
      }
    }
  ],
  "model": "claude-sonnet-4"
}
```

#### 1.4 工具结果消息

```json
{
  "type": "assistant",
  "content": [
    {
      "type": "tool_result",
      "tool_use_id": "toolu_01ABC123",
      "content": "total 48\ndrwxr-xr-x  12 user  staff   384 Jan 23 10:00 .\n...",
      "is_error": false
    }
  ],
  "model": "claude-sonnet-4"
}
```

**字段说明**:
- `type`: 固定为 `"assistant"`
- `content`: 内容块数组,可包含多个不同类型的块
- `model`: 使用的模型名称
- `parent_tool_use_id`: 可选,如果是工具结果的响应
- `error`: 可选,如果发生错误,包含错误类型

---

### 2. 流式事件 (Stream Event)

在流式响应期间,发送部分更新(用于实时显示打字效果)。

```json
{
  "type": "stream",
  "uuid": "msg-456",
  "session_id": "default",
  "event": {
    "type": "content_block_delta",
    "index": 0,
    "delta": {
      "type": "text_delta",
      "text": "Hello"
    }
  }
}
```

**字段说明**:
- `type`: 固定为 `"stream"`
- `uuid`: 消息的唯一标识符
- `session_id`: 会话 ID
- `event`: 流式事件数据(遵循 Messages API 的 streaming 格式)

**常见事件类型**:
- `message_start`: 消息开始
- `content_block_start`: 内容块开始
- `content_block_delta`: 内容块增量更新
- `content_block_stop`: 内容块结束
- `message_stop`: 消息结束

---

### 3. 系统消息 (System Message)

系统状态、元数据或通知信息。

```json
{
  "type": "system",
  "subtype": "notification",
  "data": {
    "message": "Connected to Claude Code CLI",
    "level": "info"
  }
}
```

**字段说明**:
- `type`: 固定为 `"system"`
- `subtype`: 子类型,如 `"notification"`, `"status"`, `"metadata"`
- `data`: 系统消息的具体数据

**常见 subtype**:
- `"notification"`: 通知消息
- `"session_start"`: 会话开始
- `"session_metadata"`: 会话元数据

---

### 4. 结果消息 (Result Message)

对话轮次完成后的统计信息。

```json
{
  "type": "result",
  "subtype": "success",
  "session_id": "default",
  "duration_ms": 2345,
  "duration_api_ms": 1890,
  "num_turns": 3,
  "total_cost_usd": 0.0234,
  "is_error": false,
  "usage": {
    "input_tokens": 1234,
    "output_tokens": 567
  }
}
```

**字段说明**:
- `type`: 固定为 `"result"`
- `subtype`: `"success"` 或 `"error"`
- `session_id`: 会话 ID
- `duration_ms`: 总耗时(毫秒)
- `duration_api_ms`: API 调用耗时(毫秒)
- `num_turns`: 对话轮次数
- `total_cost_usd`: 总成本(美元)
- `is_error`: 是否发生错误
- `usage`: Token 使用统计

---

### 5. 权限请求 (Permission Request)

请求用户授权工具使用。

```json
{
  "type": "permission_request",
  "request_id": "req-789",
  "tool_name": "Edit",
  "input": {
    "file_path": "/path/to/file.js",
    "old_string": "const x = 1",
    "new_string": "const x = 2"
  },
  "permission_suggestions": [
    {
      "type": "addRules",
      "rules": [
        {
          "toolName": "Edit",
          "ruleContent": "/path/to/file.js"
        }
      ],
      "behavior": "allow",
      "destination": "session"
    }
  ],
  "blocked_path": null
}
```

**字段说明**:
- `type`: 固定为 `"permission_request"`
- `request_id`: 请求 ID,用于关联响应
- `tool_name`: 工具名称
- `input`: 工具输入参数
- `permission_suggestions`: 可选,建议的权限配置
- `blocked_path`: 可选,被阻止的路径

**前端展示建议**:
显示一个对话框,展示:
1. 工具名称和操作描述
2. 具体参数(如文件路径、命令等)
3. 提供"允许"、"拒绝"、"始终允许"按钮
4. 如果有 `permission_suggestions`,提供"记住此选择"选项

---

### 6. Hook 回调请求 (Hook Callback Request)

请求执行自定义 Hook(用于高级集成)。

```json
{
  "type": "hook_callback",
  "callback_id": "hook-123",
  "hook_event": "PreToolUse",
  "input": {
    "session_id": "default",
    "transcript_path": "/path/to/transcript.json",
    "cwd": "/current/working/dir",
    "tool_name": "Bash",
    "tool_input": {
      "command": "rm -rf /"
    }
  },
  "tool_use_id": "toolu_01XYZ"
}
```

---

### 7. 错误消息 (Error Message)

错误信息。

```json
{
  "type": "error",
  "error": "Failed to execute command",
  "details": {
    "code": "EXECUTION_FAILED",
    "message": "Command 'ls' exited with code 1",
    "stderr": "ls: cannot access '/nonexistent': No such file or directory"
  }
}
```

**字段说明**:
- `type`: 固定为 `"error"`
- `error`: 错误简述
- `details`: 可选,详细错误信息

---

## 前端界面设计建议

### 1. 聊天区域

- **用户消息**: 右对齐,蓝色气泡
- **助手文本**: 左对齐,灰色气泡
- **思考过程**: 可折叠的灰色框,默认折叠
- **工具调用**: 特殊样式显示工具名称和参数
- **工具结果**: 代码块样式显示输出

### 2. 控制面板

```
┌─────────────────────────────────────┐
│ Model: [claude-sonnet-4 ▼]          │
│ Permission: [default ▼]             │
│ [Interrupt] [Clear] [Settings]      │
└─────────────────────────────────────┘
```

### 3. 权限对话框

```
┌──────────────────────────────────────┐
│ Permission Required                  │
├──────────────────────────────────────┤
│ Tool: Edit                           │
│ File: /path/to/file.js               │
│                                      │
│ Changes:                             │
│ - const x = 1                        │
│ + const x = 2                        │
│                                      │
│ [Always Allow] [Allow] [Deny]        │
└──────────────────────────────────────┘
```

### 4. Slash Commands 展示

在输入框下方显示可用命令:

```
Available Commands:
/commit - Create a git commit
/review-pr - Review a pull request
/help - Show help information
```

**实现方式**:
- 客户端硬编码常用命令列表
- 或通过一个特殊消息类型从服务器获取:

```json
{
  "type": "system",
  "subtype": "available_commands",
  "data": {
    "commands": [
      {
        "name": "/commit",
        "description": "Create a git commit",
        "args": "[message]"
      },
      {
        "name": "/review-pr",
        "description": "Review a pull request",
        "args": "<pr_number>"
      }
    ]
  }
}
```

---

## 连接流程

### 1. 建立连接

```
Client -> Server: WebSocket Connection to ws://localhost:8080/ws?session_id=abc123
Server -> Client: Connection established
Server -> Client: System message (session_start)
```

### 2. 初始化

```json
{
  "type": "system",
  "subtype": "session_start",
  "data": {
    "session_id": "abc123",
    "model": "claude-sonnet-4",
    "permission_mode": "default",
    "available_tools": ["Bash", "Read", "Edit", "Write", "Grep", "Glob"]
  }
}
```

### 3. 对话流程

```
Client -> Server: User message
Server -> Client: Stream events (typing effect)
Server -> Client: Assistant message (complete)
Server -> Client: Tool use
Server -> Client: Tool result
Server -> Client: Result message (turn complete)
```

### 4. 权限流程

```
Client -> Server: User message: "Delete all .tmp files"
Server -> Client: Permission request
Client -> Server: Permission response (allow)
Server -> Client: Tool use (Bash)
Server -> Client: Tool result
Server -> Client: Result message
```

---

## 错误处理

### 1. 连接错误

```json
{
  "type": "error",
  "error": "Connection failed",
  "details": {
    "code": "CONNECTION_FAILED",
    "message": "Unable to connect to Claude Code CLI"
  }
}
```

### 2. 解析错误

```json
{
  "type": "error",
  "error": "Invalid message format",
  "details": {
    "code": "PARSE_ERROR",
    "message": "Expected 'type' field in message"
  }
}
```

### 3. 执行错误

```json
{
  "type": "error",
  "error": "Tool execution failed",
  "details": {
    "code": "TOOL_ERROR",
    "tool": "Bash",
    "message": "Command exited with non-zero status"
  }
}
```

---

## 完整示例对话

### 示例 1: 简单对话

```javascript
// Client -> Server
{
  "type": "user_message",
  "session_id": "demo",
  "message": {
    "content": "What files are in the current directory?"
  }
}

// Server -> Client (Stream)
{
  "type": "stream",
  "uuid": "msg-001",
  "session_id": "demo",
  "event": {
    "type": "content_block_delta",
    "index": 0,
    "delta": { "type": "text_delta", "text": "Let me check" }
  }
}

// Server -> Client (Tool Use)
{
  "type": "assistant",
  "content": [
    {
      "type": "text",
      "text": "Let me check the current directory."
    },
    {
      "type": "tool_use",
      "id": "toolu_01ABC",
      "name": "Bash",
      "input": {
        "command": "ls -la",
        "description": "List files in current directory"
      }
    }
  ],
  "model": "claude-sonnet-4"
}

// Server -> Client (Tool Result)
{
  "type": "assistant",
  "content": [
    {
      "type": "tool_result",
      "tool_use_id": "toolu_01ABC",
      "content": "total 48\n-rw-r--r--  1 user  staff  1234 Jan 23 10:00 README.md\n...",
      "is_error": false
    }
  ],
  "model": "claude-sonnet-4"
}

// Server -> Client (Final Response)
{
  "type": "assistant",
  "content": [
    {
      "type": "text",
      "text": "The current directory contains: README.md, package.json, src/, and other files."
    }
  ],
  "model": "claude-sonnet-4"
}

// Server -> Client (Result)
{
  "type": "result",
  "subtype": "success",
  "session_id": "demo",
  "duration_ms": 1234,
  "duration_api_ms": 890,
  "num_turns": 1,
  "total_cost_usd": 0.0012,
  "is_error": false
}
```

### 示例 2: 权限请求

```javascript
// Client -> Server
{
  "type": "user_message",
  "session_id": "demo",
  "message": {
    "content": "Delete all .log files"
  }
}

// Server -> Client (Permission Request)
{
  "type": "permission_request",
  "request_id": "req-123",
  "tool_name": "Bash",
  "input": {
    "command": "rm *.log",
    "description": "Delete all .log files"
  },
  "permission_suggestions": [
    {
      "type": "addRules",
      "rules": [{ "toolName": "Bash", "ruleContent": "rm" }],
      "behavior": "allow",
      "destination": "session"
    }
  ]
}

// Client -> Server (Permission Response)
{
  "type": "permission_response",
  "request_id": "req-123",
  "decision": "allow"
}

// Server -> Client (Tool Use & Result)
{
  "type": "assistant",
  "content": [
    {
      "type": "tool_use",
      "id": "toolu_01XYZ",
      "name": "Bash",
      "input": {
        "command": "rm *.log",
        "description": "Delete all .log files"
      }
    }
  ],
  "model": "claude-sonnet-4"
}

// ... (tool result and final response)
```

### 示例 3: 中断操作

```javascript
// Client -> Server
{
  "type": "user_message",
  "session_id": "demo",
  "message": {
    "content": "Run a very long operation..."
  }
}

// Server -> Client (Processing)
{
  "type": "assistant",
  "content": [
    {
      "type": "tool_use",
      "id": "toolu_01ABC",
      "name": "Bash",
      "input": { "command": "sleep 100" }
    }
  ],
  "model": "claude-sonnet-4"
}

// Client -> Server (Interrupt)
{
  "type": "interrupt"
}

// Server -> Client (Interrupted)
{
  "type": "system",
  "subtype": "interrupted",
  "data": {
    "message": "Operation interrupted by user"
  }
}
```

---

## 实现建议

### 服务端 (Rust)

```rust
// 定义消息枚举
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    UserMessage {
        session_id: String,
        message: UserMessageContent,
        uuid: Option<String>,
    },
    PermissionResponse {
        request_id: String,
        decision: String,
        updated_input: Option<Value>,
        updated_permissions: Option<Vec<PermissionUpdate>>,
    },
    Interrupt,
    SetModel {
        model: String,
    },
    SetPermissionMode {
        mode: String,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMessage {
    Assistant(AssistantMessage),
    Stream(StreamEvent),
    System(SystemMessage),
    Result(ResultMessage),
    PermissionRequest(PermissionRequestMessage),
    Error(ErrorMessage),
}
```

### 客户端 (TypeScript)

```typescript
// 消息类型定义
interface UserMessage {
  type: 'user_message';
  session_id: string;
  message: {
    content: string;
    parent_tool_use_id?: string;
  };
  uuid?: string;
}

interface PermissionResponse {
  type: 'permission_response';
  request_id: string;
  decision: 'allow' | 'deny';
  updated_input?: any;
  updated_permissions?: PermissionUpdate[];
}

// WebSocket 客户端
class ClaudeWebSocketClient {
  private ws: WebSocket;

  constructor(url: string) {
    this.ws = new WebSocket(url);
    this.ws.onmessage = this.handleMessage.bind(this);
  }

  sendUserMessage(content: string, sessionId: string = 'default') {
    this.ws.send(JSON.stringify({
      type: 'user_message',
      session_id: sessionId,
      message: { content }
    }));
  }

  respondToPermission(requestId: string, allow: boolean) {
    this.ws.send(JSON.stringify({
      type: 'permission_response',
      request_id: requestId,
      decision: allow ? 'allow' : 'deny'
    }));
  }

  interrupt() {
    this.ws.send(JSON.stringify({ type: 'interrupt' }));
  }

  private handleMessage(event: MessageEvent) {
    const message = JSON.parse(event.data);
    switch (message.type) {
      case 'assistant':
        this.handleAssistantMessage(message);
        break;
      case 'permission_request':
        this.handlePermissionRequest(message);
        break;
      // ... 其他消息类型
    }
  }
}
```

---

## 安全考虑

1. **权限验证**: 所有工具调用都应通过权限检查
2. **输入验证**: 验证客户端发送的消息格式
3. **速率限制**: 限制消息发送频率,防止滥用
4. **会话隔离**: 确保不同会话的数据隔离
5. **敏感信息**: 不要在响应中包含系统敏感信息(如完整路径、环境变量等)

---

## 版本控制

**当前版本**: v1.0

**变更日志**:
- v1.0 (2026-01-23): 初始版本

**向后兼容性**:
- 添加新的消息类型不会破坏现有客户端
- 修改现有消息格式需要版本号升级
