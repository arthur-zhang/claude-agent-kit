# Disallowed Tools 功能设计

**日期**: 2026-01-27
**目的**: 在测试/调试场景下，允许用户临时禁用某些工具

## 功能概述

在 WebSocket 会话初始化时，允许用户通过前端配置 `disallowed_tools` 参数，该参数会通过 `UserSessionInit` 消息传递到后端，最终传递给 Claude Agent SDK 来限制可用工具。

## 使用场景

- 测试/调试时临时禁用某些工具
- 验证特定工具的行为
- 开发时快速切换工具配置

## 整体架构和数据流

### 数据流

1. **前端配置** → 用户在前端界面的文本输入框中输入要禁用的工具名称（逗号分隔，如 `Bash,Edit,Write`）
2. **数据解析** → 前端将输入字符串按逗号分割，去除空格，转换为字符串数组
3. **消息发送** → 前端通过 WebSocket 发送 `UserSessionInit` 消息，包含 `disallowed_tools` 字段
4. **后端接收** → Rust 后端的 `wait_for_init_message` 函数接收并解析该消息
5. **SDK 传递** → 后端将 `disallowed_tools` 传递给 `ClaudeAgentOptions`，初始化 Claude 客户端

### 类型定义变更

- 前端 `UserSessionInit` 接口添加 `disallowed_tools?: string[] | null`
- 后端 `UserSessionInitData` 结构体添加 `disallowed_tools: Option<Vec<String>>`

## 前端实现细节

### UI 组件

在 `ChatInterface.tsx` 中添加新的输入框，位置在现有配置项（cwd、model、permission_mode 等）附近。

### 输入框配置

- **标签**: `Disallowed Tools`（或中文：`禁用工具`）
- **占位符**: `例如: Bash,Edit,Write`
- **类型**: 文本输入框
- **默认值**: 空字符串 `""`

### 数据处理逻辑

```typescript
// 将用户输入的字符串转换为数组
const disallowedToolsArray = disallowedToolsInput
  .split(',')
  .map(tool => tool.trim())
  .filter(tool => tool.length > 0);

// 在 UserSessionInit 消息中包含该字段
const initMessage: UserSessionInit = {
  type: 'user_session_init',
  id: generateId(),
  session_id: sessionId,
  cwd: currentCwd,
  model: selectedModel || null,
  permission_mode: permissionMode || null,
  max_turns: maxTurns || null,
  max_budget_usd: maxBudget || null,
  user: userName || null,
  disallowed_tools: disallowedToolsArray.length > 0 ? disallowedToolsArray : null
};
```

### 状态管理

使用 React 的 `useState` 管理输入框的值，初始值为空字符串。

## 后端实现细节

### 类型定义修改

在 `websocket/src/server.rs` 中修改 `UserSessionInitData` 结构体：

```rust
#[derive(Debug, Clone)]
struct UserSessionInitData {
    cwd: String,
    model: Option<String>,
    permission_mode: Option<crate::protocol::events::PermissionMode>,
    max_turns: Option<i32>,
    max_budget_usd: Option<f64>,
    user: Option<String>,
    disallowed_tools: Option<Vec<String>>,  // 新增字段
}
```

### 消息解析

在 `wait_for_init_message` 函数中解析前端发送的 `disallowed_tools` 字段，从 JSON 中提取该数组。

### 传递给 SDK

在创建 `ClaudeAgentOptions` 时，将 `disallowed_tools` 传递给 SDK：

```rust
let mut options = ClaudeAgentOptions {
    cwd: Some(PathBuf::from(&init_data.cwd)),
    model: init_data.model.clone(),
    permission_mode: init_data.permission_mode,
    max_turns: init_data.max_turns,
    max_budget_usd: init_data.max_budget_usd,
    user: init_data.user.clone(),
    disallowed_tools: init_data.disallowed_tools.unwrap_or_default(),  // 新增
    ..Default::default()
};
```

### 错误处理

- 如果 JSON 解析失败，返回 `SessionError::ParseError`
- 如果 `disallowed_tools` 字段不存在或为 null，使用空数组作为默认值

## 测试和验证

### 功能测试

1. **基本测试** - 在前端输入 `Bash,Edit`，验证后端收到正确的数组 `["Bash", "Edit"]`
2. **空值测试** - 不输入任何内容，验证后端收到 `null` 或空数组
3. **格式测试** - 输入带空格的字符串（如 `Bash, Edit , Write`），验证能正确去除空格
4. **单个工具** - 输入单个工具名（如 `Bash`），验证能正确解析

### 验证方法

- 在后端添加日志，打印接收到的 `disallowed_tools` 值
- 尝试使用被禁用的工具，观察 Claude Agent 是否拒绝执行
- 检查 `ClaudeAgentOptions` 是否正确传递了该参数

### 边界情况处理

- 空字符串输入 → 转换为空数组或 null
- 只有逗号和空格 → 过滤后得到空数组
- 工具名大小写 → 保持用户输入的原样（由 SDK 处理）
- 无效的工具名 → 不在前端验证，由 SDK 处理

### 调试建议

可以在前端添加一个显示区域，实时显示解析后的 `disallowed_tools` 数组，方便调试。

## 实现文件清单

### 前端文件
- `websocket/frontend/src/types.ts` - 添加 `disallowed_tools` 字段到 `UserSessionInit` 接口
- `websocket/frontend/src/components/ChatInterface.tsx` - 添加输入框和状态管理

### 后端文件
- `websocket/src/server.rs` - 修改 `UserSessionInitData` 结构体和消息解析逻辑

## 默认值

- 前端输入框默认值：空字符串 `""`
- 后端默认值：空数组 `vec![]`
- SDK 接收到的默认值：空数组（不禁用任何工具）
