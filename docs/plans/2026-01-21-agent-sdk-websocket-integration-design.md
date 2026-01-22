# Agent SDK 与 WebSocket 集成设计文档

## 项目概述

将 `agent-sdk`（封装了 Claude Code CLI 的 stream-json 模式）集成到 WebSocket 服务器中，实现网页端可视化的 Claude Code chat 控制逻辑。

**核心目标**：
- 网页端通过 WebSocket 发送用户消息
- 服务器通过 agent-sdk 调用 Claude Code CLI 处理消息
- 实时流式返回 Claude 的响应给网页端
- 支持多轮对话和会话管理

**技术选型**：
- WebSocket 服务器：Axum + Tokio（已有）
- Agent SDK：claude-agent-sdk（已有）
- 集成方式：连接池 + 会话保持

## 整体架构

采用三层架构设计：

### 1. WebSocket 层（已有）
- 处理 WebSocket 连接的建立和断开
- 接收客户端消息，发送服务器响应
- 使用现有的 `connection.rs`、`handler.rs`、`message.rs`

### 2. 连接池管理层（新增）
- 管理 `ClaudeClient` 实例池
- 实现混合策略：核心池（2-3 个）+ 动态扩展（最多 20 个）+ 超时回收（5 分钟）
- 为每个 WebSocket 连接分配和回收 CLI 进程
- 跟踪进程状态（空闲、使用中、启动中）

### 3. Agent 集成层（新增）
- 封装 `agent-sdk` 的 `ClaudeClient`
- 处理 CLI 进程的生命周期（connect、disconnect）
- 将 WebSocket 消息转发给 `ClaudeClient`
- 将 `ClaudeClient` 的响应流转发回 WebSocket

### 关键设计决策
- **WebSocket 连接与 CLI 进程 1:1 绑定**（会话保持模式）
- **消息格式直接透传**（客户端使用 agent-sdk 的 JSON 格式）
- **使用 tokio::spawn 为每个连接创建独立的消息处理任务**

## 连接池实现细节

### 数据结构设计

```rust
struct AgentPool {
    // 核心配置
    core_size: usize,           // 核心池大小（2-3）
    max_size: usize,            // 最大池大小（20）
    idle_timeout: Duration,     // 空闲超时（5分钟）

    // 进程管理
    idle_agents: VecDeque<PooledAgent>,  // 空闲进程队列
    active_agents: HashMap<ConnectionId, PooledAgent>,  // 活跃进程映射

    // 状态跟踪
    total_count: usize,         // 当前总进程数
    creating_count: usize,      // 正在创建的进程数

    // 同步原语
    waiters: VecDeque<oneshot::Sender<PooledAgent>>,  // 等待进程的请求队列
}

struct PooledAgent {
    client: ClaudeClient,
    last_used: Instant,
    session_id: String,
}
```

### 核心操作

**1. 获取进程 (acquire)**：
- 优先从 `idle_agents` 获取
- 如果无空闲且未达 `max_size`，创建新进程
- 如果已达上限，加入 `waiters` 队列等待

**2. 释放进程 (release)**：
- 从 `active_agents` 移除
- 更新 `last_used` 时间戳
- 如果有等待者，直接分配；否则放入 `idle_agents`

**3. 后台清理任务**：
- 每分钟检查一次 `idle_agents`
- 超过 `idle_timeout` 的进程调用 `disconnect()` 并移除
- 保持至少 `core_size` 个进程

## 消息流转和数据流

### WebSocket 连接建立流程

1. 客户端连接到 `/ws`
2. 服务器从 `AgentPool` 获取一个 `ClaudeClient` 实例
3. 调用 `client.connect(None)` 启动 CLI 进程
4. 生成 `ConnectionId` 并注册到 `ConnectionManager`
5. 启动两个并发任务：
   - **接收任务**：监听 WebSocket 消息 → 转发给 `ClaudeClient`
   - **发送任务**：监听 `ClaudeClient` 响应 → 转发给 WebSocket

### 消息处理流程

```
客户端 → WebSocket → 解析 JSON → 验证格式 → ClaudeClient.query_string()
                                                        ↓
客户端 ← WebSocket ← 序列化 JSON ← Message 枚举 ← receive_messages()
```

### 具体实现伪代码

```rust
// 接收任务
while let Some(msg) = ws_receiver.recv().await {
    let json: serde_json::Value = serde_json::from_str(&msg)?;

    // 直接透传，提取必要字段
    let prompt = json["message"]["content"].as_str()?;
    let session_id = json["session_id"].as_str().unwrap_or("default");

    client.query_string(prompt, session_id).await?;
}

// 发送任务
let mut message_stream = client.receive_messages().await?;
while let Some(result) = message_stream.next().await {
    let message = result?;
    let json = serde_json::to_string(&message)?;
    ws_sender.send(json).await?;
}
```

## 错误处理和异常情况

### 错误类型和处理策略

**1. CLI 进程启动失败**：
- 场景：`claude code` 命令不存在或启动超时
- 处理：向客户端发送错误消息，关闭 WebSocket 连接
- 日志：记录详细错误信息供调试

**2. 消息解析错误**：
- 场景：客户端发送的 JSON 格式不正确
- 处理：发送错误响应，不断开连接
- 格式：`{"type": "error", "message": "Invalid JSON format"}`

**3. CLI 进程崩溃**：
- 场景：`ClaudeClient` 的 `receive_messages()` 流中断
- 处理：向客户端发送通知，关闭 WebSocket 连接
- 清理：从 `active_agents` 移除，不放回池中

**4. WebSocket 连接断开**：
- 场景：客户端主动断开或网络中断
- 处理：调用 `client.disconnect()`，释放进程回池
- 超时：如果 `disconnect()` 超过 5 秒，强制终止进程

**5. 连接池耗尽**：
- 场景：所有进程都在使用中，且已达 `max_size`
- 处理：将请求加入等待队列，设置超时（30 秒）
- 超时后：返回 503 错误给客户端

### 日志记录
- 使用 `tracing` 记录所有关键事件
- 级别：连接建立/断开（info）、错误（error）、池状态变化（debug）

## 项目结构和模块组织

### 新增模块

```
websocket/src/
├── lib.rs              # 现有
├── main.rs             # 现有
├── server.rs           # 现有
├── connection.rs       # 现有
├── message.rs          # 现有
├── handler.rs          # 需要修改：集成 agent pool
├── error.rs            # 现有
├── agent/              # 新增目录
│   ├── mod.rs          # 导出公共 API
│   ├── pool.rs         # AgentPool 实现
│   ├── client.rs       # PooledAgent 封装
│   └── session.rs      # WebSocket 会话管理
```

### 模块职责

**1. agent/pool.rs**：
- `AgentPool` 结构体和实现
- 进程的获取、释放、清理逻辑
- 后台清理任务

**2. agent/client.rs**：
- `PooledAgent` 封装
- 封装 `ClaudeClient` 的生命周期管理
- 提供简化的 API

**3. agent/session.rs**：
- `AgentSession` 结构体
- 管理单个 WebSocket 连接的完整生命周期
- 处理消息的接收和发送任务

**4. handler.rs 修改**：
- 在 WebSocket 握手时从池中获取 agent
- 创建 `AgentSession` 并启动消息处理
- 连接断开时释放 agent 回池

### 依赖更新

```toml
[dependencies]
claude-agent-sdk = { path = "../agent-sdk" }
# 现有依赖保持不变
```

## 测试策略

### 单元测试

**1. AgentPool 测试** (`agent/pool.rs`)：
- 测试核心池预创建
- 测试动态扩展（达到 max_size）
- 测试空闲超时回收
- 测试并发获取和释放
- 测试等待队列机制

**2. AgentSession 测试** (`agent/session.rs`)：
- 测试消息转发逻辑
- 测试错误处理
- 测试连接断开清理

### 集成测试

**1. 端到端流程测试**：
- 启动 WebSocket 服务器（使用测试端口）
- 使用 `tokio-tungstenite` 客户端连接
- 发送 agent-sdk 格式的消息
- 验证收到正确的响应流
- 测试多轮对话

**2. 并发连接测试**：
- 同时建立多个 WebSocket 连接
- 验证连接池正确分配进程
- 验证达到上限后的等待机制

**3. 异常场景测试**：
- 测试客户端突然断开
- 测试发送无效消息
- 测试 CLI 进程崩溃恢复

### 测试工具
- 更新 `static/index.html` 测试页面，支持发送 agent-sdk 格式的消息
- 添加 JavaScript 示例展示如何构造正确的消息格式

## 实现步骤

### Phase 1 - 基础集成
1. 实现 `agent/pool.rs` 的基本功能（固定大小池）
2. 实现 `agent/client.rs` 封装
3. 实现 `agent/session.rs` 消息转发
4. 修改 `handler.rs` 集成连接池
5. 基础测试验证

### Phase 2 - 动态管理
1. 实现动态扩展逻辑
2. 实现空闲超时回收
3. 实现等待队列机制
4. 添加完整的单元测试

### Phase 3 - 优化和监控
1. 添加池状态监控端点（`/pool/stats`）
2. 优化错误处理和日志
3. 性能测试和调优
4. 完善集成测试

## 配置项

通过环境变量配置：

```bash
AGENT_POOL_CORE_SIZE=3          # 核心池大小
AGENT_POOL_MAX_SIZE=20          # 最大池大小
AGENT_POOL_IDLE_TIMEOUT=300     # 空闲超时（秒）
AGENT_POOL_ACQUIRE_TIMEOUT=30   # 获取超时（秒）
```

## 后续扩展方向

- 支持多租户（不同用户使用不同的 API key）
- 添加消息队列持久化（Redis）
- 实现会话恢复（WebSocket 重连后恢复上下文）
- 添加速率限制和配额管理
- 支持自定义 agent 配置（model、max_turns 等）

## 消息协议示例

### 客户端发送消息

```json
{
  "type": "user",
  "message": {
    "role": "user",
    "content": "Hello, Claude!"
  },
  "parent_tool_use_id": null,
  "session_id": "default"
}
```

### 服务器响应消息

**文本消息**：
```json
{
  "content": [
    {
      "type": "text",
      "text": "Hello! How can I help you?"
    }
  ],
  "model": "claude-sonnet-4",
  "parent_tool_use_id": null
}
```

**工具调用消息**：
```json
{
  "content": [
    {
      "type": "tool_use",
      "id": "toolu_123",
      "name": "Bash",
      "input": {
        "command": "ls -la"
      }
    }
  ],
  "model": "claude-sonnet-4"
}
```

**结果消息**：
```json
{
  "subtype": "success",
  "duration_ms": 1234,
  "duration_api_ms": 1000,
  "is_error": false,
  "num_turns": 1,
  "session_id": "default",
  "total_cost_usd": 0.001
}
```

## 架构图

```
┌─────────────┐
│   Browser   │
│  (Client)   │
└──────┬──────┘
       │ WebSocket
       │
┌──────▼──────────────────────────────────┐
│         WebSocket Server (Axum)         │
│  ┌────────────────────────────────────┐ │
│  │      Connection Manager            │ │
│  └────────────────────────────────────┘ │
│  ┌────────────────────────────────────┐ │
│  │         Agent Pool                 │ │
│  │  ┌──────────┐  ┌──────────┐       │ │
│  │  │ Agent 1  │  │ Agent 2  │  ...  │ │
│  │  └──────────┘  └──────────┘       │ │
│  └────────────────────────────────────┘ │
│  ┌────────────────────────────────────┐ │
│  │       Agent Session                │ │
│  │  (Message forwarding)              │ │
│  └────────────────────────────────────┘ │
└──────┬──────────────────────────────────┘
       │
┌──────▼──────────────────────────────────┐
│         Agent SDK (ClaudeClient)        │
│  ┌────────────────────────────────────┐ │
│  │   SubprocessCLITransport           │ │
│  └────────────────────────────────────┘ │
└──────┬──────────────────────────────────┘
       │ stdin/stdout
┌──────▼──────────────────────────────────┐
│      Claude Code CLI Process            │
│      (stream-json mode)                 │
└─────────────────────────────────────────┘
```
