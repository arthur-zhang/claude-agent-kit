# Query-Based 架构重构实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将当前的事件循环架构重构为基于 query 的架构，每轮对话返回一个 Stream，Stream 结束代表这轮完成。

**架构:**
- `Session` 管理 `ClaudeClient` 生命周期
- `query()` 执行一轮对话，返回 `Stream<Item = Result<SDKMessage, QueryError>>`
- 权限处理通过回调集成
- 取消机制通过 `CancellationToken` 实现

**Tech Stack:** Rust, tokio, async-stream, futures, claude_agent_sdk

---

## Task 1: 创建 QueryError 和相关基础类型

**Files:**
- Create: `websocket/src/session/query/error.rs`
- Modify: `websocket/src/session/query/mod.rs`

**Step 1: 创建 error.rs 文件**

```rust
// websocket/src/session/query/error.rs

use std::fmt;

/// Query 过程中的错误
#[derive(Debug)]
pub enum QueryError {
    /// API 错误
    ApiError(String),
    /// 连接丢失
    ConnectionLost,
    /// 超时
    Timeout,
    /// 被中断
    Interrupted,
    /// SDK 错误
    SdkError(claude_agent_sdk::Error),
    /// 权限被拒绝
    PermissionDenied(String),
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryError::ApiError(msg) => write!(f, "API Error: {}", msg),
            QueryError::ConnectionLost => write!(f, "Connection lost"),
            QueryError::Timeout => write!(f, "Operation timed out"),
            QueryError::Interrupted => write!(f, "Query interrupted"),
            QueryError::SdkError(e) => write!(f, "SDK Error: {}", e),
            QueryError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
        }
    }
}

impl std::error::Error for QueryError {}

impl From<claude_agent_sdk::Error> for QueryError {
    fn from(e: claude_agent_sdk::Error) -> Self {
        QueryError::SdkError(e)
    }
}
```

**Step 2: 创建 mod.rs 导出**

```rust
// websocket/src/session/query/mod.rs

pub mod error;

pub use error::QueryError;
```

**Step 3: 更新 session/mod.rs**

```rust
// websocket/src/session/mod.rs

pub mod query;
// ... 其他已有的 mod
```

**Step 4: 验证编译**

Run: `cargo check -p websocket`
Expected: 编译通过

**Step 5: Commit**

```bash
git add websocket/src/session/query/
git commit -m "feat(query): add QueryError type definition"
```

---

## Task 2: 创建 QueryOptions 结构

**Files:**
- Modify: `websocket/src/session/query/mod.rs`

**Step 1: 添加 QueryOptions 结构**

```rust
// websocket/src/session/query/mod.rs

use crate::protocol::types::PermissionMode;
use std::collections::HashMap;
use std::sync::Arc;

/// 权限请求
#[derive(Debug, Clone)]
pub struct PermissionRequest {
    pub tool_name: String,
    pub tool_use_id: Option<String>,
    pub input: serde_json::Value,
}

/// 权限响应
#[derive(Debug, Clone)]
pub enum PermissionResponse {
    Allow,
    Deny,
    AllowAlways,
}

/// 权限处理器类型
pub type PermissionHandler = Arc<
    dyn Fn(PermissionRequest) -> futures::future::BoxFuture<'static, PermissionResponse>
        + Send
        + Sync,
>;

/// Query 选项
#[derive(Clone)]
pub struct QueryOptions {
    /// 权限模式
    pub permission_mode: PermissionMode,
    /// 权限处理器回调
    pub permission_handler: Option<PermissionHandler>,
    /// 最大轮次数
    pub max_turns: Option<i32>,
    /// 额外的环境变量
    pub env: Option<HashMap<String, String>>,
    /// 是否跳过 AskUserQuestion 工具
    pub disallowed_tools: Option<Vec<String>>,
}
```

**Step 2: 验证编译**

Run: `cargo check -p websocket`
Expected: 编译通过

**Step 3: Commit**

```bash
git add websocket/src/session/query/mod.rs
git commit -m "feat(query): add QueryOptions and PermissionHandler types"
```

---

## Task 3: 创建 Session 结构管理 ClaudeClient

**Files:**
- Create: `websocket/src/session/query/session.rs`
- Modify: `websocket/src/session/query/mod.rs`

**Step 1: 创建 session.rs**

```rust
// websocket/src/session/query/session.rs

use super::{QueryError, QueryOptions, PermissionHandler, PermissionRequest, PermissionResponse};
use crate::protocol::types::SessionConfig;
use claude_agent_sdk::{ClaudeAgentOptions, ClaudeClient};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

/// Session 管理一个 ClaudeClient 连接
pub struct Session {
    /// 会话 ID
    session_id: String,
    /// Claude 客户端
    client: Arc<Mutex<ClaudeClient>>,
    /// 会话配置
    config: SessionConfig,
    /// 工作目录
    cwd: PathBuf,
}

impl Session {
    /// 创建新会话
    pub async fn new(
        session_id: String,
        cwd: PathBuf,
        config: SessionConfig,
        options: &QueryOptions,
    ) -> Result<Self, QueryError> {
        info!("Creating new session {} with cwd: {:?}", session_id, cwd);

        // 构建 ClaudeAgentOptions
        let agent_options = Self::build_agent_options(&cwd, &config, options);

        // 创建并连接客户端
        let mut client = ClaudeClient::new(agent_options);
        client.connect(None).await?;

        let session = Self {
            session_id,
            client: Arc::new(Mutex::new(client)),
            config,
            cwd,
        };

        Ok(session)
    }

    /// 恢复现有会话
    pub async fn resume(
        session_id: String,
        resume_id: String,
        cwd: PathBuf,
        config: SessionConfig,
        options: &QueryOptions,
    ) -> Result<Self, QueryError> {
        info!("Resuming session {} from resume_id: {}", session_id, resume_id);

        let mut agent_options = Self::build_agent_options(&cwd, &config, options);
        agent_options.resume = Some(resume_id);

        let mut client = ClaudeClient::new(agent_options);
        client.connect(None).await?;

        let session = Self {
            session_id,
            client: Arc::new(Mutex::new(client)),
            config,
            cwd,
        };

        Ok(session)
    }

    /// 获取会话 ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// 获取工作目录
    pub fn cwd(&self) -> &PathBuf {
        &self.cwd
    }

    /// 获取配置
    pub fn config(&self) -> &SessionConfig {
        &self.config
    }

    /// 构建 ClaudeAgentOptions
    fn build_agent_options(
        cwd: &PathBuf,
        config: &SessionConfig,
        options: &QueryOptions,
    ) -> ClaudeAgentOptions {
        let mut agent_options = ClaudeAgentOptions::new();
        agent_options.cwd = Some(cwd.clone());

        // 转换权限模式
        use claude_agent_sdk::PermissionMode as SdkMode;
        use crate::protocol::types::PermissionMode as ProtoMode;

        let sdk_mode = match config.permission_mode {
            ProtoMode::Auto => SdkMode::Default,
            ProtoMode::Manual => SdkMode::Default,
            ProtoMode::Bypass => SdkMode::BypassPermissions,
        };
        agent_options.permission_mode = Some(sdk_mode);

        if let Some(max_turns) = config.max_turns {
            agent_options.max_turns = Some(max_turns);
        }

        if let Some(ref tools) = options.disallowed_tools {
            agent_options.disallowed_tools = tools.clone();
        }

        agent_options
    }
}
```

**Step 2: 更新 mod.rs**

```rust
// websocket/src/session/query/mod.rs

pub mod error;
pub mod session;

pub use error::QueryError;
pub use session::Session;
```

**Step 3: 验证编译**

Run: `cargo check -p websocket`
Expected: 编译通过，可能有未使用警告（正常）

**Step 4: Commit**

```bash
git add websocket/src/session/query/
git commit -m "feat(query): add Session struct to manage ClaudeClient"
```

---

## Task 4: 实现 query 函数返回 Stream

**Files:**
- Modify: `websocket/src/session/query/session.rs`

**Step 1: 添加 query 方法**

```rust
// websocket/src/session/query/session.rs

use super::{QueryError, QueryOptions, PermissionHandler, PermissionRequest, PermissionResponse};
use crate::protocol::types::SessionConfig;
use claude_agent_sdk::{ClaudeAgentOptions, ClaudeClient, types::InputMessage};
use futures::stream::{Stream, StreamExt};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

// 在 Session impl 中添加

/// 执行一轮对话，返回消息流
///
/// 返回的 Stream 产出该轮对话的所有消息
/// Stream 结束代表这轮对话完成
pub fn query<'a>(
    &'a self,
    message: String,
    options: QueryOptions,
    cancel_token: CancellationToken,
) -> Pin<Box<dyn Stream<Item = Result<claude_agent_sdk::types::ProtocolMessage, QueryError>> + Send + 'a>> {
    use async_stream::try_stream;

    Box::pin(async_stream::stream! {
        info!("[{}] Starting query with message length: {}", self.session_id, message.len());

        let client = self.client.lock().await;

        // 发送用户消息
        let input_msg = InputMessage::user(message, "default".to_string());
        if let Err(e) = client.send_input_message(input_msg).await {
            error!("[{}] Failed to send input message: {:?}", self.session_id, e);
            yield Err(QueryError::from(e));
            return;
        }

        // 订阅协议消息
        let mut agent_stream = match client.receive_protocol_messages().await {
            Ok(stream) => stream,
            Err(e) => {
                error!("[{}] Failed to subscribe to protocol messages: {:?}", self.session_id, e);
                yield Err(QueryError::from(e));
                return;
            }
        };

        // 处理消息流
        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    info!("[{}] Query cancelled", self.session_id);
                    // 尝试中断
                    let _ = client.interrupt().await;
                    yield Err(QueryError::Interrupted);
                    return;
                }

                result = agent_stream.next() => {
                    match result {
                        Some(Ok(msg)) => {
                            // 检查是否是 Result 消息且表示轮次结束
                            use claude_agent_sdk::types::ProtocolMessage;
                            match &msg {
                                ProtocolMessage::Result(r) if !r.is_error => {
                                    // 检查是否有 status 字段表示完成
                                    if r.result.as_ref().map(|s| s.contains("completed")).unwrap_or(false) {
                                        debug!("[{}] Turn completed", self.session_id);
                                        yield Ok(msg);
                                        return;
                                    }
                                }
                                ProtocolMessage::Result(r) if r.is_error => {
                                    let error_msg = r.errors.join("; ");
                                    error!("[{}] SDK returned error: {}", self.session_id, error_msg);
                                    yield Err(QueryError::ApiError(error_msg));
                                    return;
                                }
                                _ => {}
                            }
                            yield Ok(msg);
                        }
                        Some(Err(e)) => {
                            error!("[{}] Stream error: {:?}", self.session_id, e);
                            yield Err(QueryError::from(e));
                            return;
                        }
                        None => {
                            info!("[{}] Stream ended naturally", self.session_id);
                            return;
                        }
                    }
                }
            }
        }
    })
}
```

**Step 2: 添加 async-stream 依赖**

在 `websocket/Cargo.toml` 中添加：

```toml
[dependencies]
async-stream = "0.3"
tokio-util = { version = "0.7", features = ["sync"] }
```

**Step 3: 验证编译**

Run: `cargo check -p websocket`
Expected: 编译通过

**Step 4: Commit**

```bash
git add websocket/src/session/query/ websocket/Cargo.toml
git commit -m "feat(query): implement query method returning Stream"
```

---

## Task 5: 在 server.rs 中集成 Session

**Files:**
- Modify: `websocket/src/server.rs`

**Step 1: 简化 handle_socket，使用 Session**

将当前复杂的事件循环替换为基于 query 的简单循环：

```rust
// websocket/src/server.rs

use crate::session::query::{Session, QueryOptions, QueryError, PermissionRequest, PermissionResponse};
use crate::protocol::events::{AgentEvent, ClientMessage};
use futures::{SinkExt, StreamExt};
use tokio_util::sync::CancellationToken;
use std::sync::Arc;

// ... 其他 imports

async fn handle_socket(socket: WebSocket, _state: AppState, session_id: String) {
    info!("New WebSocket connection for session {}", session_id);

    let (mut ws_sender, mut ws_receiver) = socket.split();

    // 创建 writer task
    let (tx, mut rx) = tokio::sync::mpsc::channel::<WsMessage>(100);
    let session_id_clone = session_id.clone();
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
        let _ = ws_sender.close().await;
    });

    let ws_sender = tx;

    // Phase 1: 等待 UserSessionInit
    let init_data = match wait_for_init_message(&mut ws_receiver, &session_id).await {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to receive init message: {}", e);
            send_error_and_close(ws_sender.clone(), &session_id, e).await;
            return;
        }
    };

    let cwd = PathBuf::from(&init_data.cwd);
    let config = SessionConfig {
        permission_mode: convert_permission_mode(init_data.permission_mode.as_ref()),
        max_turns: init_data.max_turns,
        metadata: Default::default(),
    };

    let options = QueryOptions {
        permission_mode: config.permission_mode.clone(),
        permission_handler: None, // 后续添加
        max_turns: init_data.max_turns,
        env: None,
        disallowed_tools: init_data.disallowed_tools,
    };

    // Phase 2: 创建 Session
    let session = match init_data.resume {
        Some(resume_id) => {
            match Session::resume(session_id.clone(), resume_id, cwd, config, &options).await {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to resume session: {}", e);
                    send_init_error(ws_sender.clone(), &session_id, e.to_string()).await;
                    return;
                }
            }
        }
        None => {
            match Session::new(session_id.clone(), cwd, config, &options).await {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to create session: {}", e);
                    send_init_error(ws_sender.clone(), &session_id, e.to_string()).await;
                    return;
                }
            }
        }
    };

    // 发送 SessionInit 成功消息
    let init_event = AgentEvent::SessionInit {
        success: true,
        session_id: session.session_id().to_string(),
        error: None,
        data: SessionInitData::default(), // 从 SDK 获取实际数据
    };
    send_event(&ws_sender, init_event).await;

    // Phase 3: 消息循环 - 每条用户消息 = 一轮 query
    let mut current_cancel: Option<CancellationToken> = None;

    loop {
        tokio::select! {
            msg_result = ws_receiver.next() => {
                match msg_result {
                    Some(Ok(WsMessage::Text(text))) => {
                        if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                            match client_msg {
                                ClientMessage::UserMessage { content, .. } => {
                                    // 创建新的取消 token
                                    let cancel_token = CancellationToken::new();
                                    current_cancel = Some(cancel_token.clone());

                                    // 发送 TurnStarted
                                    send_event(&ws_sender, AgentEvent::TurnStarted {
                                        session_id: session.session_id().to_string(),
                                    }).await;

                                    // 执行 query
                                    let stream = session.query(content, options.clone(), cancel_token);
                                    pin_mut!(stream);

                                    let mut turn_completed = false;

                                    while let Some(result) = stream.next().await {
                                        match result {
                                            Ok(msg) => {
                                                // 转换 ProtocolMessage 为 AgentEvent
                                                if let Some(event) = convert_message_to_event(&session, msg) {
                                                    send_event(&ws_sender, event).await;

                                                    // 检查是否是 TurnCompleted
                                                    if matches!(event, AgentEvent::TurnCompleted { .. }) {
                                                        turn_completed = true;
                                                    }
                                                }
                                            }
                                            Err(QueryError::Interrupted) => {
                                                send_event(&ws_sender, AgentEvent::TurnFailed {
                                                    session_id: session.session_id().to_string(),
                                                    error: "Interrupted by user".to_string(),
                                                }).await;
                                                break;
                                            }
                                            Err(e) => {
                                                send_event(&ws_sender, AgentEvent::TurnFailed {
                                                    session_id: session.session_id().to_string(),
                                                    error: e.to_string(),
                                                }).await;
                                                break;
                                            }
                                        }
                                    }

                                    current_cancel = None;

                                    // 如果没有自然完成，发送 TurnCompleted
                                    if !turn_completed {
                                        send_event(&ws_sender, AgentEvent::TurnCompleted {
                                            session_id: session.session_id().to_string(),
                                            usage: TokenUsage::default(),
                                            duration_ms: None,
                                            duration_api_ms: None,
                                            num_turns: None,
                                            total_cost_usd: None,
                                        }).await;
                                    }
                                }
                                ClientMessage::ControlRequest { subtype, .. } => {
                                    match subtype {
                                        crate::protocol::events::ControlSubtype::Interrupt => {
                                            if let Some(token) = current_cancel.take() {
                                                token.cancel();
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }

    info!("WebSocket connection closed for session {}", session_id);
}

// 辅助函数

async fn send_init_error(
    ws_sender: tokio::sync::mpsc::Sender<WsMessage>,
    session_id: &str,
    error: String,
) {
    let event = AgentEvent::SessionInit {
        success: false,
        session_id: session_id.to_string(),
        error: Some(error),
        data: SessionInitData::default(),
    };
    send_event(ws_sender, event).await;
}

async fn send_event(
    ws_sender: &tokio::sync::mpsc::Sender<WsMessage>,
    event: AgentEvent,
) {
    if let Ok(json) = serde_json::to_string(&event) {
        let _ = ws_sender.send(WsMessage::Text(json)).await;
    }
}

fn convert_message_to_event(
    session: &Session,
    msg: claude_agent_sdk::types::ProtocolMessage,
) -> Option<AgentEvent> {
    use claude_agent_sdk::types::ProtocolMessage;

    match msg {
        ProtocolMessage::User(user) => Some(AgentEvent::AssistantMessage {
            session_id: session.session_id().to_string(),
            text: user.content.unwrap_or_default(),
            is_final: true,
        }),
        ProtocolMessage::Assistant(assistant) => Some(AgentEvent::AssistantMessage {
            session_id: session.session_id().to_string(),
            text: assistant.content.unwrap_or_default(),
            is_final: true,
        }),
        ProtocolMessage::ToolUse(tool) => Some(AgentEvent::ToolStarted {
            session_id: session.session_id().to_string(),
            tool_name: tool.name,
            tool_id: tool.id,
            arguments: tool.input,
            parent_tool_use_id: tool.parent_tool_use_id,
        }),
        ProtocolMessage::Result(result) => {
            if result.is_error {
                Some(AgentEvent::Error {
                    session_id: session.session_id().to_string(),
                    message: result.errors.join("; "),
                    is_fatal: false,
                })
            } else if result.result.as_ref().map(|s| s.contains("completed")).unwrap_or(false) {
                Some(AgentEvent::TurnCompleted {
                    session_id: session.session_id().to_string(),
                    usage: TokenUsage::default(),
                    duration_ms: None,
                    duration_api_ms: None,
                    num_turns: None,
                    total_cost_usd: None,
                })
            } else {
                None
            }
        }
        _ => None,
    }
}
```

**Step 2: 验证编译**

Run: `cargo check -p websocket`
Expected: 编译可能有错误，需要根据实际情况调整

**Step 3: 根据编译错误修复代码**

Run: `cargo check -p websocket 2>&1 | head -50`
根据错误信息调整代码

**Step 4: Commit**

```bash
git add websocket/src/server.rs
git commit -m "refactor(server): integrate Session-based query flow"
```

---

## Task 6: 添加权限处理集成

**Files:**
- Modify: `websocket/src/server.rs`
- Modify: `websocket/src/session/query/mod.rs`

**Step 1: 在 Session 中添加 permission_handler 支持**

修改 Session::query 方法支持权限处理：

```rust
// 在 server.rs 中创建权限处理器

let (perm_req_tx, mut perm_req_rx) = tokio::sync::mpsc::channel::<PermissionRequest>(10);
let perm_req_tx_clone = perm_req_tx.clone();

let options = QueryOptions {
    permission_mode: config.permission_mode.clone(),
    permission_handler: Some(Arc::new(move |req| {
        let tx = perm_req_tx_clone.clone();
        Box::pin(async move {
            let (resp_tx, mut resp_rx) = tokio::sync::oneshot::channel();
            tx.send(req).await.ok();
            resp_rx.recv().await.unwrap_or(PermissionResponse::Deny)
        })
    })),
    max_turns: init_data.max_turns,
    env: None,
    disallowed_tools: init_data.disallowed_tools,
};
```

**Step 2: 在消息循环中处理权限请求**

```rust
loop {
    tokio::select! {
        msg_result = ws_receiver.next() => {
            // 处理客户端消息...
        }
        Some(req) = perm_req_rx.recv() => {
            // 发送权限请求事件
            let event = AgentEvent::ControlRequest {
                session_id: session.session_id().to_string(),
                request_id: uuid::Uuid::new_v4().to_string(),
                tool_name: req.tool_name.clone(),
                tool_use_id: req.tool_use_id,
                input: req.input,
                context: PermissionContext {
                    description: format!("Tool use: {}", req.tool_name),
                    risk_level: RiskLevel::Medium,
                },
            };
            send_event(&ws_sender, event).await;

            // 等待响应... (需要存储 request_id 和响应 channel 的映射)
        }
    }
}
```

**Step 3: 验证编译**

Run: `cargo check -p websocket`

**Step 4: Commit**

```bash
git add websocket/src/
git commit -m "feat(query): integrate permission handling with callbacks"
```

---

## Task 7: 清理和测试

**Files:**
- Modify: 删除不再需要的文件

**Step 1: 删除旧的 event_handler.rs**

如果新的 query 流程已经替代了 event_handler：

Run: `rm websocket/src/session/event_handler.rs`

**Step 2: 更新 session/mod.rs**

移除对 event_handler 的引用

**Step 3: 运行测试**

Run: `cargo test -p websocket`

**Step 4: 手动测试**

1. 启动 WebSocket 服务器
2. 连接并发送 UserSessionInit
3. 发送 UserMessage
4. 验证收到 TurnStarted -> 消息 -> TurnCompleted

**Step 5: Commit**

```bash
git add -A
git commit -m "refactor(query): remove old event_handler, clean up unused code"
```

---

## 验证清单

- [ ] Session 可以正确创建和恢复
- [ ] query 返回的 Stream 正确产出消息
- [ ] Stream 在一轮结束时自然结束
- [ ] 取消机制 (CancellationToken) 正常工作
- [ ] 权限请求通过回调正确处理
- [ ] 错误情况返回 QueryError
- [ ] WebSocket 连接可以处理多轮对话
