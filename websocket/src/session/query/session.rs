use super::{QueryError, QueryOptions, PermissionHandler, PermissionRequest, PermissionResponse};
use crate::protocol::types::{PermissionMode, SessionConfig};
use async_trait::async_trait;
use claude_agent_sdk::types::{
    CanUseTool, InputMessage, PermissionBehavior, PermissionResult, PermissionResultAllow,
    PermissionResultDeny, PermissionRuleValue, PermissionUpdate, PermissionUpdateDestination,
    ProtocolMessage, ToolPermissionContext,
};
use claude_agent_sdk::{ClaudeAgentOptions, ClaudeClient};
use futures::stream::{Stream, StreamExt};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

/// 权限处理器适配器 - 将 PermissionHandler 适配为 CanUseTool trait
struct PermissionHandlerAdapter {
    handler: PermissionHandler,
}

#[async_trait]
impl CanUseTool for PermissionHandlerAdapter {
    async fn can_use(
        &self,
        tool_name: &str,
        input: &serde_json::Value,
        _context: &ToolPermissionContext,
    ) -> claude_agent_sdk::types::error::Result<PermissionResult> {
        info!("🔐 PermissionHandlerAdapter::can_use called for tool: {}", tool_name);

        let request = PermissionRequest {
            tool_name: tool_name.to_string(),
            tool_use_id: None, // ToolPermissionContext 没有 tool_use_id 字段
            input: input.clone(),
        };

        info!("🔐 Calling permission handler...");
        let response = (self.handler)(request).await;
        info!("🔐 Permission handler returned: {:?}", response);

        match response {
            PermissionResponse::Allow => Ok(PermissionResult::Allow(PermissionResultAllow::default())),
            PermissionResponse::AllowAlways => {
                // 创建权限更新，将该工具添加到 Session 级别的允许列表
                let permission_update = PermissionUpdate::AddRules {
                    rules: Some(vec![PermissionRuleValue {
                        tool_name: tool_name.to_string(),
                        rule_content: None,
                    }]),
                    behavior: Some(PermissionBehavior::Allow),
                    destination: Some(PermissionUpdateDestination::Session),
                };

                let allow = PermissionResultAllow {
                    behavior: "allow".to_string(),
                    updated_input: None,
                    updated_permissions: Some(vec![permission_update]),
                };
                info!("🔐 AllowAlways: adding permission rule for tool: {}", tool_name);
                Ok(PermissionResult::Allow(allow))
            }
            PermissionResponse::Deny => Ok(PermissionResult::Deny(PermissionResultDeny::default())),
        }
    }
}

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

        let agent_options = Self::build_agent_options(&cwd, &config, options);

        let mut client = ClaudeClient::new(agent_options);
        client.connect(None).await?;

        Ok(Self {
            session_id,
            client: Arc::new(Mutex::new(client)),
            config,
            cwd,
        })
    }

    /// 恢复现有会话
    pub async fn resume(
        session_id: String,
        resume_id: String,
        cwd: PathBuf,
        config: SessionConfig,
        options: &QueryOptions,
    ) -> Result<Self, QueryError> {
        info!(
            "Resuming session {} from resume_id: {}",
            session_id, resume_id
        );

        let mut agent_options = Self::build_agent_options(&cwd, &config, options);
        agent_options.resume = Some(resume_id);

        let mut client = ClaudeClient::new(agent_options);
        client.connect(None).await?;

        Ok(Self {
            session_id,
            client: Arc::new(Mutex::new(client)),
            config,
            cwd,
        })
    }

    /// 获取会话 ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// 更新会话 ID（当 SDK 返回实际的 session_id 时使用）
    pub fn set_session_id(&mut self, session_id: String) {
        info!("Updating session_id from {} to {}", self.session_id, session_id);
        self.session_id = session_id;
    }

    /// 获取工作目录
    pub fn cwd(&self) -> &PathBuf {
        &self.cwd
    }

    /// 获取配置
    pub fn config(&self) -> &SessionConfig {
        &self.config
    }

    /// 获取客户端引用
    pub fn client(&self) -> &Arc<Mutex<ClaudeClient>> {
        &self.client
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

        let sdk_mode = match config.permission_mode {
            PermissionMode::Default => SdkMode::Default,
            PermissionMode::AcceptEdits => SdkMode::AcceptEdits,
            PermissionMode::BypassPermissions => SdkMode::BypassPermissions,
            PermissionMode::Plan => SdkMode::Plan,
            PermissionMode::Delegate => SdkMode::Delegate,
            PermissionMode::DontAsk => SdkMode::DontAsk,
        };
        agent_options.permission_mode = Some(sdk_mode);

        // 设置 dangerously_skip_permissions (通过 extra_args)
        if config.dangerously_skip_permissions == Some(true) {
            agent_options.extra_args.insert(
                "dangerously-skip-permissions".to_string(),
                None,
            );
            info!("⚠️ Setting dangerously_skip_permissions flag");
        }

        if let Some(max_turns) = config.max_turns {
            agent_options.max_turns = Some(max_turns);
        }

        // 设置 max_thinking_tokens
        if let Some(max_thinking_tokens) = config.max_thinking_tokens {
            agent_options.max_thinking_tokens = Some(max_thinking_tokens);
            info!("🧠 Setting max_thinking_tokens to {}", max_thinking_tokens);
        }

        if let Some(ref tools) = options.disallowed_tools {
            agent_options.disallowed_tools = tools.clone();
        }

        // 设置权限处理器回调
        if let Some(ref handler) = options.permission_handler {
            info!("🔐 Setting up permission handler adapter");
            let adapter = PermissionHandlerAdapter {
                handler: handler.clone(),
            };
            agent_options.can_use_tool = Some(Box::new(adapter));
        } else {
            info!("⚠️ No permission handler provided");
        }

        agent_options
    }

    /// 执行一轮对话，返回消息流
    ///
    /// # Arguments
    /// * `message` - 用户消息
    /// * `options` - 查询选项（当前未使用，保留用于未来扩展）
    /// * `cancel_token` - 取消令牌
    pub fn query(
        &self,
        message: String,
        _options: QueryOptions, // TODO: 将来用于超时、权限处理等
        cancel_token: CancellationToken,
    ) -> Pin<Box<dyn Stream<Item = Result<ProtocolMessage, QueryError>> + Send + '_>> {
        let session_id = self.session_id.clone();
        let client = self.client.clone();

        Box::pin(async_stream::stream! {
            info!("[{}] Starting query with message length: {}", session_id, message.len());

            let client_guard = client.lock().await;

            // 发送用户消息
            let input_msg = InputMessage::user(message, session_id.clone());
            if let Err(e) = client_guard.send_input_message(input_msg).await {
                error!("[{}] Failed to send input message: {:?}", session_id, e);
                yield Err(QueryError::from(e));
                return;
            }

            // 订阅协议消息
            let mut agent_stream = match client_guard.receive_protocol_messages().await {
                Ok(stream) => stream,
                Err(e) => {
                    error!("[{}] Failed to subscribe to protocol messages: {:?}", session_id, e);
                    yield Err(QueryError::from(e));
                    return;
                }
            };

            // 释放锁，让其他操作可以进行
            drop(client_guard);

            // 处理消息流
            loop {
                tokio::select! {
                    _ = cancel_token.cancelled() => {
                        info!("[{}] Query cancelled", session_id);
                        // 尝试中断，使用 try_lock 避免死锁
                        if let Ok(client_guard) = client.try_lock() {
                            let _ = client_guard.interrupt().await;
                        } else {
                            debug!("[{}] Could not acquire lock for interrupt, skipping", session_id);
                        }
                        yield Err(QueryError::Interrupted);
                        return;
                    }

                    result = agent_stream.next() => {
                        match result {
                            Some(Ok(msg)) => {
                                match &msg {
                                    ProtocolMessage::Result(r) if r.is_error => {
                                        let error_msg = r.errors.join("; ");
                                        error!("[{}] SDK returned error: {}", session_id, error_msg);
                                        yield Err(QueryError::ApiError(error_msg));
                                        return;
                                    }
                                    ProtocolMessage::Result(r) => {
                                        // 成功的 Result 消息表示这轮结束
                                        info!("[{}] Turn completed with subtype: {}", session_id, r.subtype);
                                        yield Ok(msg);
                                        return;
                                    }
                                    _ => {}
                                }
                                yield Ok(msg);
                            }
                            Some(Err(e)) => {
                                error!("[{}] Stream error: {:?}", session_id, e);
                                yield Err(QueryError::from(e));
                                return;
                            }
                            None => {
                                debug!("[{}] Stream ended naturally", session_id);
                                return;
                            }
                        }
                    }
                }
            }
        })
    }
}
