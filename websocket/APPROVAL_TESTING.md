# Approval 功能测试指南

本文档说明如何测试 WebSocket 服务器的工具审批功能。

## 功能概述

我们实现了一个完整的工具审批系统，参考了 vibe-kanban 项目的设计：

### 后端功能

1. **ApprovalService** - 审批服务抽象层
   - `WebSocketApprovalService` - 基于 WebSocket 的实现
   - 支持 Approved/Denied/TimedOut/Pending 状态
   - 使用 oneshot channel 进行异步通知

2. **PermissionHandler** - 权限处理器
   - 集成 ApprovalService
   - 支持 Auto/Manual/Bypass 三种权限模式
   - 特殊处理：ExitPlanMode 批准后自动切换到 Bypass 模式
   - 风险评估：High/Medium/Low

3. **Session Handler** - 会话处理器
   - 处理客户端的 permission_response 消息
   - 将响应传递给 ApprovalService
   - 支持动态权限模式切换

### 前端功能

1. **PermissionDialog** - 权限对话框组件
   - 显示工具名称、描述、风险级别
   - 显示工具输入参数（JSON 格式）
   - Allow/Deny 按钮
   - 风险级别颜色编码（红/黄/绿）

2. **useWebSocket Hook** - WebSocket 管理
   - 处理 permission_request 消息
   - 发送 permission_response 消息
   - 管理 pendingPermission 状态

## 测试步骤

### 1. 启动后端服务器

```bash
cd websocket
cargo run
```

服务器将在 `http://localhost:3000` 启动。

### 2. 启动前端开发服务器

```bash
cd websocket/frontend
bun install  # 如果还没安装依赖
bun dev
```

前端将在 `http://localhost:5173` 启动。

### 3. 连接到服务器

1. 打开浏览器访问 `http://localhost:5173`
2. 在顶部输入框中输入 Session ID（默认 "default"）
3. 点击 "Connect" 按钮
4. 等待连接状态变为绿色 "Connected"

### 4. 测试工具审批流程

#### 测试场景 1：基本审批流程

1. 在聊天输入框中输入：
   ```
   List all files in the current directory
   ```

2. Claude 会尝试使用 `Bash` 工具执行 `ls` 命令

3. 权限对话框应该弹出，显示：
   - Tool: `Bash`
   - Description: "Allow Bash to execute?"
   - Risk Level: `MEDIUM`（黄色）
   - Tool Input: `{"command": "ls -la", "description": "List files in current directory"}`

4. 点击 "✅ Allow" 按钮

5. 工具应该执行，结果显示在聊天中

#### 测试场景 2：拒绝工具执行

1. 输入：
   ```
   Delete all temporary files
   ```

2. Claude 可能会尝试使用 `Bash` 工具执行 `rm` 命令

3. 权限对话框弹出，显示：
   - Tool: `Bash`
   - Risk Level: `HIGH`（红色，因为包含 `rm` 命令）

4. 点击 "❌ Deny" 按钮

5. Claude 应该收到拒绝消息，不会执行工具

#### 测试场景 3：读取文件（低风险）

1. 输入：
   ```
   Read the README.md file
   ```

2. Claude 会尝试使用 `Read` 工具

3. 权限对话框显示：
   - Tool: `Read`
   - Risk Level: `LOW`（绿色）

4. 点击 "✅ Allow"

5. 文件内容应该显示在聊天中

#### 测试场景 4：ExitPlanMode 特殊处理

1. 输入：
   ```
   Create a plan to implement a new feature
   ```

2. Claude 进入 Plan 模式

3. 当 Claude 完成计划并调用 `ExitPlanMode` 工具时：
   - 权限对话框弹出
   - 点击 "✅ Allow"
   - 服务器应该自动切换到 `BypassPermissions` 模式
   - 后续工具调用不再需要审批

#### 测试场景 5：审批超时

1. 输入一个会触发工具使用的消息

2. 当权限对话框弹出时，**不要点击任何按钮**

3. 等待 5 分钟（超时时间）

4. 应该看到超时错误消息

### 5. 验证功能

#### 检查后端日志

后端应该输出类似的日志：

```
INFO  Sent permission request req-123 for tool Bash
INFO  Responding to approval request req-123: Approved
INFO  Permission granted for tool Bash
```

或拒绝时：

```
INFO  Sent permission request req-456 for tool Bash
INFO  Responding to approval request req-456: Denied
INFO  Permission denied for tool Bash: User denied permission to use Bash
```

#### 检查前端控制台

前端控制台应该显示：

```javascript
Received message: {
  type: "permission_request",
  id: "req-123",
  session_id: "default",
  tool_name: "Bash",
  tool_input: {...},
  context: {
    description: "Allow Bash to execute?",
    risk_level: "medium"
  }
}
```

发送响应时：

```javascript
Sending: {
  type: "permission_response",
  id: "resp-456",
  session_id: "default",
  request_id: "req-123",
  decision: "allow"
}
```

## 架构说明

### 审批流程

```
1. Claude SDK 请求工具权限
   ↓
2. PermissionHandler.can_use()
   ↓
3. 检查权限模式 (Auto/Manual/Bypass)
   ↓
4. Manual 模式：创建 ApprovalRequest
   ↓
5. ApprovalService.request_approval()
   - 创建 oneshot channel
   - 存储到 pending map
   ↓
6. 发送 permission_request 到 WebSocket 客户端
   ↓
7. 前端显示 PermissionDialog
   ↓
8. 用户点击 Allow/Deny
   ↓
9. 前端发送 permission_response
   ↓
10. SessionHandler 接收响应
   ↓
11. ApprovalService.respond_to_approval()
   - 从 pending map 移除
   - 通过 oneshot channel 发送结果
   ↓
12. PermissionHandler 收到结果
   ↓
13. 返回 PermissionResult 给 Claude SDK
   ↓
14. Claude SDK 执行或拒绝工具
```

### 关键组件

#### 后端

- `websocket/src/session/approval.rs` - ApprovalService trait 和实现
- `websocket/src/session/permission.rs` - PermissionHandler
- `websocket/src/session/handler.rs` - 处理 permission_response 消息
- `websocket/src/server.rs` - 创建和连接各组件

#### 前端

- `frontend/src/components/PermissionDialog.tsx` - 权限对话框 UI
- `frontend/src/hooks/useWebSocket.ts` - WebSocket 通信逻辑
- `frontend/src/types.ts` - TypeScript 类型定义

## 故障排除

### 权限对话框不显示

1. 检查后端日志，确认发送了 permission_request
2. 检查前端控制台，确认收到了消息
3. 检查 `pendingPermission` 状态是否正确设置

### 点击 Allow/Deny 没有反应

1. 检查前端控制台，确认发送了 permission_response
2. 检查后端日志，确认收到了响应
3. 检查 request_id 是否匹配

### 工具执行失败

1. 检查 ApprovalService 是否正确响应
2. 检查 PermissionHandler 是否返回了正确的 PermissionResult
3. 检查 Claude SDK 是否收到了权限结果

### 超时问题

1. 默认超时时间是 5 分钟（300 秒）
2. 可以在 `permission.rs` 中修改超时时间
3. 超时后会自动调用 `cancel_approval()`

## 下一步改进

1. **持久化权限规则** - 记住用户的审批决策
2. **批量审批** - 一次性批准多个相似的工具调用
3. **审批历史** - 显示所有审批记录
4. **自定义风险评估** - 允许用户配置风险规则
5. **审批通知** - 当需要审批时发送通知
6. **审批统计** - 显示审批通过率、拒绝率等

## 参考

- vibe-kanban 项目的 approval 实现
- Claude Agent SDK 文档
- WebSocket 协议文档：`docs/websocket-protocol.md`
