# WebSocket 通信协议设计

**日期:** 2026-01-23
**版本:** 1.0
**状态:** 设计草案

## 概述

本文档定义了 Claude Agent Kit 中 WebSocket 服务器与客户端之间的通信协议。该协议采用扁平化的 JSON 消息格式，支持：

- 统一的消息格式
- 增强的权限控制能力
- 丰富的交互场景（会话管理、执行控制等）
- 清晰的错误处理机制
- 流式消息传输

## 核心设计原则

1. **扁平化设计** - 所有消息类型在同一层级，通过 `type` 字段区分
2. **请求-响应配对** - 每个请求有唯一 ID，响应包含 `request_id` 字段
3. **同步阻塞的权限请求** - 权限请求等待用户响应后才继续执行
4. **增量流式更新** - 流式消息通过增量方式传输，前端累积拼接
5. **简单错误处理** - 错误消息只包含必要信息：描述和可选的请求 ID

## 通用消息结构

所有消息都遵循以下基础结构：

```json
{
  "type": "消息类型",
  "id": "msg_唯一标识",
  "session_id": "会话ID",
  "timestamp": 1234567890
}
```

### 核心字段说明

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `type` | string | ✅ | 消息类型，如 `user_message`、`assistant_message` 等 |
| `id` | string | ✅ | 消息的唯一标识符，格式为 `msg_` + UUID |
| `session_id` | string | ✅ | 会话标识符，用于区分不同的对话会话 |
| `timestamp` | number | ❌ | Unix 时间戳（毫秒），用于调试和日志记录 |

### 请求-响应关联

响应类消息（如 `permission_response`、`tool_result`）包含 `request_id` 字段：

```json
{
  "type": "permission_response",
  "id": "msg_resp456",
  "request_id": "msg_perm123",
  ...
}
```

## 消息类型定义

### 1. 基础对话消息

#### 1.1 用户消息 (user_message)

**方向:** 客户端 → 服务器

```json
{
  "type": "user_message",
  "id": "msg_abc123",
  "session_id": "sess_xyz789",
  "content": "帮我列出当前目录的文件",
  "parent_tool_use_id": null
}
```

**字段说明:**

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `content` | string | ✅ | 用户输入的文本内容 |
| `parent_tool_use_id` | string \| null | ❌ | 如果是对工具调用的响应，指向工具调用的 ID |

#### 1.2 Assistant 消息（流式）

**方向:** 服务器 → 客户端

采用增量更新模式，分为三种消息类型：

**消息开始 (assistant_message_start)**

```json
{
  "type": "assistant_message_start",
  "id": "msg_def456",
  "session_id": "sess_xyz789",
  "model": "claude-sonnet-4-5"
}
```

**内容增量 (assistant_message_delta)**

```json
{
  "type": "assistant_message_delta",
  "id": "msg_def456",
  "session_id": "sess_xyz789",
  "delta": {
    "type": "text",
    "text": "当前目录包含以下文件："
  }
}
```

**消息完成 (assistant_message_complete)**

```json
{
  "type": "assistant_message_complete",
  "id": "msg_def456",
  "session_id": "sess_xyz789"
}
```

### 2. 工具调用相关消息

#### 2.1 工具调用 (tool_use)

**方向:** 服务器 → 客户端

```json
{
  "type": "tool_use",
  "id": "msg_tool123",
  "session_id": "sess_xyz789",
  "tool_use_id": "toolu_abc456",
  "tool_name": "Bash",
  "tool_input": {
    "command": "ls -la",
    "description": "列出当前目录的文件"
  }
}
```

**字段说明:**

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `tool_use_id` | string | ✅ | 工具调用的唯一标识（用于关联 tool_result） |
| `tool_name` | string | ✅ | 工具名称（如 Bash, Read, Write 等） |
| `tool_input` | object | ✅ | 工具的输入参数 |

#### 2.2 工具结果 (tool_result)

**方向:** 服务器 → 客户端

```json
{
  "type": "tool_result",
  "id": "msg_result789",
  "session_id": "sess_xyz789",
  "request_id": "msg_tool123",
  "tool_use_id": "toolu_abc456",
  "content": "total 48\ndrwxr-xr-x  12 user  staff   384 Jan 23 10:00 .\n...",
  "is_error": false
}
```

**字段说明:**

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `request_id` | string | ✅ | 指向对应的 tool_use 消息 |
| `tool_use_id` | string | ✅ | 工具调用的 ID |
| `content` | string | ✅ | 工具执行的输出结果 |
| `is_error` | boolean | ✅ | 是否为错误结果 |

### 3. 权限控制消息

#### 3.1 权限请求 (permission_request)

**方向:** 服务器 → 客户端

```json
{
  "type": "permission_request",
  "id": "msg_perm123",
  "session_id": "sess_xyz789",
  "tool_name": "Bash",
  "tool_input": {
    "command": "rm -rf /tmp/test",
    "description": "删除测试目录"
  },
  "context": {
    "suggestions": [
      {
        "type": "addRules",
        "rules": [
          {
            "toolName": "Bash",
            "ruleContent": "rm -rf /tmp/*"
          }
        ],
        "behavior": "allow",
        "destination": "session"
      }
    ]
  }
}
```

**字段说明:**

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `tool_name` | string | ✅ | 需要权限的工具名称 |
| `tool_input` | object | ✅ | 工具的输入参数 |
| `context` | object | ✅ | 权限上下文，包含 CLI 的建议 |

#### 3.2 权限响应 (permission_response)

**方向:** 客户端 → 服务器

```json
{
  "type": "permission_response",
  "id": "msg_resp456",
  "session_id": "sess_xyz789",
  "request_id": "msg_perm123",
  "decision": "allow",
  "updated_input": null,
  "updated_permissions": null
}
```

**字段说明:**

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `request_id` | string | ✅ | 指向对应的 permission_request |
| `decision` | string | ✅ | 决策结果：`"allow"` 或 `"deny"` |
| `updated_input` | object \| null | ❌ | 修改后的工具输入 |
| `updated_permissions` | array \| null | ❌ | 权限更新配置 |

#### 3.3 权限更新 (permission_update)

**方向:** 客户端 → 服务器

```json
{
  "type": "permission_update",
  "id": "msg_update789",
  "session_id": "sess_xyz789",
  "updates": [
    {
      "type": "addRules",
      "rules": [{"toolName": "Read"}],
      "behavior": "allow",
      "destination": "session"
    }
  ]
}
```

### 4. 会话管理消息

#### 4.1 会话开始 (session_start)

**方向:** 客户端 → 服务器

```json
{
  "type": "session_start",
  "id": "msg_start123",
  "session_id": "sess_xyz789",
  "config": {
    "model": "claude-sonnet-4-5",
    "max_turns": 100,
    "permission_mode": "default"
  }
}
```

**服务器响应 (session_started):**

```json
{
  "type": "session_started",
  "id": "msg_started456",
  "session_id": "sess_xyz789",
  "request_id": "msg_start123",
  "connection_id": "conn_abc789"
}
```

#### 4.2 会话结束 (session_end)

**方向:** 客户端 → 服务器

```json
{
  "type": "session_end",
  "id": "msg_end123",
  "session_id": "sess_xyz789"
}
```

**服务器响应 (session_ended):**

```json
{
  "type": "session_ended",
  "id": "msg_ended456",
  "session_id": "sess_xyz789",
  "request_id": "msg_end123"
}
```

#### 4.3 会话信息查询 (session_info)

**方向:** 客户端 → 服务器

```json
{
  "type": "session_info",
  "id": "msg_info123",
  "session_id": "sess_xyz789"
}
```

**服务器响应 (session_info_response):**

```json
{
  "type": "session_info_response",
  "id": "msg_info_resp456",
  "session_id": "sess_xyz789",
  "request_id": "msg_info123",
  "info": {
    "num_turns": 5,
    "total_cost_usd": 0.05,
    "model": "claude-sonnet-4-5",
    "permission_mode": "default"
  }
}
```

### 5. 执行控制消息

#### 5.1 中断执行 (interrupt)

**方向:** 客户端 → 服务器

```json
{
  "type": "interrupt",
  "id": "msg_interrupt123",
  "session_id": "sess_xyz789",
  "reason": "用户取消操作"
}
```

**服务器响应 (interrupted):**

```json
{
  "type": "interrupted",
  "id": "msg_interrupted456",
  "session_id": "sess_xyz789",
  "request_id": "msg_interrupt123"
}
```

#### 5.2 恢复执行 (resume)

**方向:** 客户端 → 服务器

```json
{
  "type": "resume",
  "id": "msg_resume123",
  "session_id": "sess_xyz789"
}
```

#### 5.3 取消任务 (cancel)

**方向:** 客户端 → 服务器

```json
{
  "type": "cancel",
  "id": "msg_cancel123",
  "session_id": "sess_xyz789",
  "target_id": "msg_tool456"
}
```

**服务器响应 (cancelled):**

```json
{
  "type": "cancelled",
  "id": "msg_cancelled789",
  "session_id": "sess_xyz789",
  "request_id": "msg_cancel123",
  "target_id": "msg_tool456"
}
```

### 6. 状态和元数据消息

#### 6.1 状态更新 (status_update)

**方向:** 服务器 → 客户端

```json
{
  "type": "status_update",
  "id": "msg_status123",
  "session_id": "sess_xyz789",
  "status": "thinking",
  "message": "正在思考如何解决问题..."
}
```

**状态类型:**

| 状态 | 说明 |
|------|------|
| `thinking` | 正在思考 |
| `executing_tool` | 正在执行工具 |
| `waiting_permission` | 等待权限授权 |
| `idle` | 空闲状态 |

#### 6.2 结果消息 (result)

**方向:** 服务器 → 客户端

```json
{
  "type": "result",
  "id": "msg_result123",
  "session_id": "sess_xyz789",
  "subtype": "success",
  "duration_ms": 5234,
  "duration_api_ms": 4100,
  "is_error": false,
  "num_turns": 3,
  "total_cost_usd": 0.012,
  "usage": {
    "input_tokens": 1500,
    "output_tokens": 800
  }
}
```

#### 6.3 思考过程 (thinking)

**方向:** 服务器 → 客户端

```json
{
  "type": "thinking",
  "id": "msg_think123",
  "session_id": "sess_xyz789",
  "content": "我需要先列出文件，然后分析哪些是重要的...",
  "signature": "sig_abc123"
}
```

### 7. 错误处理消息

#### 7.1 错误消息 (error)

**方向:** 服务器 → 客户端

```json
{
  "type": "error",
  "id": "msg_error123",
  "session_id": "sess_xyz789",
  "request_id": "msg_user456",
  "message": "权限被拒绝：无法执行 bash 命令"
}
```

**常见错误场景:**

- 权限被拒绝
- 消息格式错误
- 会话不存在
- CLI 进程崩溃
- 网络超时

#### 7.2 警告消息 (warning)

**方向:** 服务器 → 客户端

```json
{
  "type": "warning",
  "id": "msg_warn123",
  "session_id": "sess_xyz789",
  "message": "API 调用接近速率限制"
}
```

### 8. 配置管理消息

#### 8.1 配置更新 (config_update)

**方向:** 客户端 → 服务器

```json
{
  "type": "config_update",
  "id": "msg_config123",
  "session_id": "sess_xyz789",
  "config": {
    "model": "claude-opus-4",
    "max_turns": 200,
    "permission_mode": "plan"
  }
}
```

**服务器响应 (config_updated):**

```json
{
  "type": "config_updated",
  "id": "msg_config_resp456",
  "session_id": "sess_xyz789",
  "request_id": "msg_config123",
  "config": {
    "model": "claude-opus-4",
    "max_turns": 200,
    "permission_mode": "plan"
  }
}
```

#### 8.2 配置查询 (config_query)

**方向:** 客户端 → 服务器

```json
{
  "type": "config_query",
  "id": "msg_query123",
  "session_id": "sess_xyz789"
}
```

**服务器响应 (config_response):**

```json
{
  "type": "config_response",
  "id": "msg_query_resp456",
  "session_id": "sess_xyz789",
  "request_id": "msg_query123",
  "config": {
    "model": "claude-sonnet-4-5",
    "max_turns": 100,
    "permission_mode": "default",
    "working_directory": "/Users/arthur/project"
  }
}
```

**可配置项:**

| 配置项 | 类型 | 说明 |
|--------|------|------|
| `model` | string | 使用的模型 |
| `max_turns` | number | 最大轮次 |
| `permission_mode` | string | 权限模式 |
| `working_directory` | string | 工作目录 |

**权限模式:**

- `default`: 默认权限模式
- `acceptEdits`: 接受编辑模式
- `plan`: 计划模式
- `bypassPermissions`: 绕过权限模式

## 消息类型汇总表

| 消息类型 | 方向 | 说明 |
|----------|------|------|
| `user_message` | 客户端 → 服务器 | 用户发送消息 |
| `assistant_message_start` | 服务器 → 客户端 | Assistant 消息开始 |
| `assistant_message_delta` | 服务器 → 客户端 | Assistant 内容增量 |
| `assistant_message_complete` | 服务器 → 客户端 | Assistant 消息完成 |
| `tool_use` | 服务器 → 客户端 | 工具调用 |
| `tool_result` | 服务器 → 客户端 | 工具执行结果 |
| `permission_request` | 服务器 → 客户端 | 权限请求 |
| `permission_response` | 客户端 → 服务器 | 权限响应 |
| `permission_update` | 客户端 → 服务器 | 权限配置更新 |
| `session_start` | 客户端 → 服务器 | 会话开始 |
| `session_started` | 服务器 → 客户端 | 会话已开始 |
| `session_end` | 客户端 → 服务器 | 会话结束 |
| `session_ended` | 服务器 → 客户端 | 会话已结束 |
| `session_info` | 客户端 → 服务器 | 会话信息查询 |
| `session_info_response` | 服务器 → 客户端 | 会话信息响应 |
| `interrupt` | 客户端 → 服务器 | 中断执行 |
| `interrupted` | 服务器 → 客户端 | 已中断 |
| `resume` | 客户端 → 服务器 | 恢复执行 |
| `cancel` | 客户端 → 服务器 | 取消任务 |
| `cancelled` | 服务器 → 客户端 | 已取消 |
| `status_update` | 服务器 → 客户端 | 状态更新 |
| `result` | 服务器 → 客户端 | 最终结果 |
| `thinking` | 服务器 → 客户端 | 思考过程 |
| `error` | 服务器 → 客户端 | 错误消息 |
| `warning` | 服务器 → 客户端 | 警告消息 |
| `config_update` | 客户端 → 服务器 | 配置更新 |
| `config_updated` | 服务器 → 客户端 | 配置已更新 |
| `config_query` | 客户端 → 服务器 | 配置查询 |
| `config_response` | 服务器 → 客户端 | 配置响应 |

## 典型交互流程

### 流程 1: 基础对话

```
客户端                    服务器
  |                         |
  |----- user_message ------>|
  |                         |
  |<---- assistant_message_start ----|
  |<---- assistant_message_delta ----|
  |<---- assistant_message_delta ----|
  |<---- assistant_message_complete --|
  |<---- result ---------------------|
```

### 流程 2: 工具调用

```
客户端                    服务器
  |                         |
  |----- user_message ------>|
  |                         |
  |<---- assistant_message_start ----|
  |<---- assistant_message_delta ----|
  |<---- tool_use ------------------|
  |                         |
  |                         |--[执行工具]
  |                         |
  |<---- tool_result ---------------|
  |<---- assistant_message_delta ----|
  |<---- assistant_message_complete --|
  |<---- result ---------------------|
```

### 流程 3: 权限请求

```
客户端                    服务器
  |                         |
  |<---- assistant_message_start ----|
  |<---- assistant_message_delta ----|
  |<---- tool_use ------------------|
  |<---- permission_request ---------|
  |                         |
  |----- permission_response ------>|
  |                         |
  |                         |--[执行工具]
  |                         |
  |<---- tool_result ---------------|
  |<---- assistant_message_delta ----|
  |<---- assistant_message_complete --|
```

### 流程 4: 中断执行

```
客户端                    服务器
  |                         |
  |<---- assistant_message_start ----|
  |                         |
  |----- interrupt ----------->|
  |                         |
  |<---- interrupted ---------------|
```

## 实现建议

### 服务器端

1. **消息路由:** 使用 `type` 字段进行消息分发到不同的处理器
2. **会话管理:** 维护 `session_id` 到会话状态的映射
3. **消息序列化:** 使用 serde_json 进行高效的 JSON 序列化/反序列化
4. **流式控制:** 在发送 `assistant_message_delta` 时注意流量控制
5. **权限处理:** 实现同步阻塞的权限请求机制

### 客户端

1. **消息累积:** 对于流式消息，需要累积 `delta` 内容
2. **请求跟踪:** 维护 `request_id` 到请求的映射，用于关联响应
3. **超时处理:** 为权限请求等需要用户交互的消息设置合理的超时
4. **错误处理:** 统一处理 `error` 和 `warning` 消息
5. **状态管理:** 根据 `status_update` 更新 UI 状态

## 安全考虑

1. **输入验证:** 严格验证所有客户端输入的消息格式
2. **权限控制:** 所有工具调用都必须经过权限检查
3. **会话隔离:** 不同 `session_id` 之间必须完全隔离
4. **消息大小限制:** 限制单个消息的最大大小
5. **速率限制:** 对高频消息进行速率限制

## 扩展性

协议设计考虑了以下扩展点：

1. **新增消息类型:** 通过添加新的 `type` 值即可扩展
2. **新增字段:** 所有消息都支持添加可选字段，不影响现有客户端
3. **批量操作:** 可以添加批量消息类型
4. **多会话:** 协议支持多个并发会话

## 版本历史

| 版本 | 日期 | 变更说明 |
|------|------|----------|
| 1.0 | 2026-01-23 | 初始版本 |

## 参考资料

- [WebSocket Server Design](./2026-01-20-websocket-server-design.md)
- [Agent SDK WebSocket Integration](./2026-01-21-agent-sdk-websocket-integration.md)
