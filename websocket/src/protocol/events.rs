//! Unified event types based on conduit's event system.
//!
//! This module defines a comprehensive event system that maps conduit's AgentEvent
//! to WebSocket protocol messages, providing fine-grained event tracking.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Import shared types from common module
pub use super::common::{Decision, PermissionContext, PermissionMode, RiskLevel};

// ============================================================================
// Helper functions
// ============================================================================

fn default_true() -> bool {
    true
}

// ============================================================================
// Sidecar Message Types (for WebSocket protocol)
// ============================================================================

/// Sidecar message - wraps SDK raw messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidecarMessage {
    pub id: String,
    #[serde(rename = "type")]
    pub msg_type: String, // Fixed as "message"
    #[serde(rename = "agentType")]
    pub agent_type: String, // Fixed as "claude"
    pub data: serde_json::Value, // Raw ProtocolMessage
}

/// Sidecar error message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidecarError {
    pub id: String,
    #[serde(rename = "type")]
    pub msg_type: String, // Fixed as "error"
    #[serde(rename = "agentType")]
    pub agent_type: String,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Permission request message - sent to frontend to request tool permission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlRequestMessage {
    #[serde(rename = "type")]
    pub msg_type: String, // Fixed as "permission_request"
    pub id: String,       // session_id
    #[serde(rename = "agentType")]
    pub agent_type: String, // Fixed as "claude"
    #[serde(rename = "toolName")]
    pub tool_name: String,
    #[serde(rename = "toolUseId", skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
    pub input: serde_json::Value,
    pub context: PermissionContext,
}

// ============================================================================
// Core Event Types (from conduit)
// ============================================================================

/// Unified event type emitted by the agent runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// Session initialized with session ID and system information
    /// Also used as response to UserSessionInit - success/error fields indicate initialization result
    SessionInit {
        /// Whether initialization succeeded
        #[serde(default = "default_true")]
        success: bool,
        /// Session ID (empty string if initialization failed)
        session_id: String,
        /// Error message (only present on failure)
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
        #[serde(flatten)]
        data: SessionInitData,
    },

    /// Turn/task started
    TurnStarted { session_id: String },

    /// Turn/task completed
    TurnCompleted {
        session_id: String,
        usage: TokenUsage,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_api_ms: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        num_turns: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        total_cost_usd: Option<f64>,
    },

    /// Turn failed with error
    TurnFailed { session_id: String, error: String },

    /// Assistant text message (streaming)
    AssistantMessage {
        session_id: String,
        text: String,
        is_final: bool,
    },

    /// Assistant reasoning/thinking (streaming)
    AssistantReasoning { session_id: String, text: String },

    /// Tool use started
    ToolStarted {
        session_id: String,
        tool_name: String,
        tool_id: String,
        arguments: serde_json::Value,
        /// If present, indicates this is a SubAgent tool call
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_tool_use_id: Option<String>,
    },

    /// Tool use completed
    ToolCompleted {
        session_id: String,
        tool_id: String,
        success: bool,
        result: Option<String>,
        error: Option<String>,
        /// If present, indicates this result is from a SubAgent
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_tool_use_id: Option<String>,
    },

    /// Control request (permission prompt) from agent runtime
    ControlRequest {
        session_id: String,
        request_id: String,
        tool_name: String,
        tool_use_id: Option<String>,
        input: serde_json::Value,
        context: PermissionContext,
    },

    /// File operation
    FileChanged {
        session_id: String,
        path: String,
        operation: FileOperation,
    },

    /// Command execution output
    CommandOutput {
        session_id: String,
        command: String,
        output: String,
        exit_code: Option<i32>,
        is_streaming: bool,
    },

    /// Token usage update
    TokenUsage {
        session_id: String,
        usage: TokenUsage,
        context_window: Option<i64>,
        usage_percent: Option<f32>,
    },

    /// Context compaction triggered
    ContextCompaction {
        session_id: String,
        reason: String,
        tokens_before: i64,
        tokens_after: i64,
    },

    /// Error event
    Error {
        session_id: String,
        message: String,
        is_fatal: bool,
    },

    /// AskUserQuestion - agent is asking user for input
    AskUserQuestion {
        session_id: String,
        /// Request ID for correlating question with response (not a real tool_use_id)
        request_id: String,
        questions: Vec<UserQuestion>,
    },

    /// ExitPlanMode - agent is exiting plan mode
    ExitPlanMode {
        session_id: String,
        /// Request ID for correlating approval with response (not a real tool_use_id)
        request_id: String,
        plan_file_path: Option<String>,
    },

    /// Session info - general session status update
    SessionInfo {
        session_id: String,
        status: SessionStatus,
    },

    /// Control response - response to a control_request
    ControlResponse {
        session_id: String,
        /// The request_id from the original control_request
        request_id: String,
        /// The subtype of the original control_request
        subtype: ControlSubtype,
        /// Whether the request succeeded
        success: bool,
        /// Error message if failed
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },

    /// Heartbeat - keep-alive message
    Heartbeat { session_id: String, timestamp: u64 },

    /// Raw/unknown event (for forward compatibility)
    Raw {
        session_id: String,
        data: serde_json::Value,
    },
}

// ============================================================================
// Client Messages (from client to server)
// ============================================================================

/// Messages sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// User message - sends a text message to the agent
    UserMessage {
        session_id: String,
        content: String,
        parent_tool_use_id: Option<String>,
    },

    /// Permission response - responds to a control request (conductor-bundle protocol format)
    PermissionResponse {
        id: String, // session_id
        #[serde(rename = "agentType")]
        agent_type: String,
        decision: Decision,
    },

    /// User question response - responds to AskUserQuestion
    UserQuestionResponse {
        session_id: String,
        /// Request ID matching the original AskUserQuestion
        request_id: String,
        answers: Vec<QuestionAnswer>,
    },

    /// Plan approval response - responds to ExitPlanMode
    PlanApprovalResponse {
        session_id: String,
        /// Request ID matching the original ExitPlanMode
        request_id: String,
        approved: bool,
        feedback: Option<String>,
    },

    /// Session start - initialize a new session
    SessionStart {
        session_id: String,
        #[serde(flatten)]
        config: SessionConfig,
    },

    /// Session end - terminate the session
    SessionEnd { session_id: String },

    /// Control request - unified control message (interrupt, resume, cancel, etc.)
    ControlRequest {
        session_id: String,
        /// Client-generated request_id for correlating with response
        request_id: String,
        subtype: ControlSubtype,
        /// Optional reason (for interrupt)
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },

    /// Set permission mode - dynamically change permission mode
    SetPermissionMode {
        session_id: String,
        mode: PermissionMode,
    },

    /// User session init - initialize agent with configuration
    UserSessionInit {
        cwd: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        permission_mode: Option<PermissionMode>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_turns: Option<i32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_budget_usd: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        user: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        disallowed_tools: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_thinking_tokens: Option<i32>,
        /// Resume a previous session by its session ID
        #[serde(skip_serializing_if = "Option::is_none")]
        resume: Option<String>,
        /// Allow bypassing permission checks (required for bypassPermissions mode)
        #[serde(
            rename = "dangerouslySkipPermissions",
            skip_serializing_if = "Option::is_none"
        )]
        dangerously_skip_permissions: Option<bool>,
    },

    /// Workspace init - conductor-bundle protocol format
    #[serde(rename = "workspace_init")]
    WorkspaceInit {
        id: String, // session_id
        #[serde(rename = "agentType")]
        agent_type: String,
        options: WorkspaceInitOptions,
    },

    /// Cancel - conductor-bundle protocol format for interrupting
    Cancel {
        id: String, // session_id
        #[serde(rename = "agentType")]
        agent_type: String,
    },

    /// Query - conductor-bundle protocol format for user messages
    Query {
        id: String, // session_id
        #[serde(rename = "agentType")]
        agent_type: String,
        prompt: String,
        options: QueryOptions,
    },
}

// ============================================================================
// Supporting Data Structures
// ============================================================================

/// Options for workspace_init message (conductor-bundle protocol)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInitOptions {
    pub cwd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(
        rename = "permissionMode",
        skip_serializing_if = "Option::is_none"
    )]
    pub permission_mode: Option<PermissionMode>,
    #[serde(
        rename = "disallowedTools",
        skip_serializing_if = "Option::is_none"
    )]
    pub disallowed_tools: Option<Vec<String>>,
    #[serde(
        rename = "maxThinkingTokens",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_thinking_tokens: Option<i32>,
    /// Allow bypassing permission checks (required for bypassPermissions mode)
    #[serde(
        rename = "dangerouslySkipPermissions",
        skip_serializing_if = "Option::is_none"
    )]
    pub dangerously_skip_permissions: Option<bool>,
    /// Resume a previous session by its session ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume: Option<String>,
}

/// Options for query message (conductor-bundle protocol)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueryOptions {
    pub cwd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(rename = "turnId", skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    #[serde(
        rename = "permissionMode",
        skip_serializing_if = "Option::is_none"
    )]
    pub permission_mode: Option<PermissionMode>,
    #[serde(rename = "maxTurns", skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume: Option<String>,
}

/// Response for workspace_init (conductor-bundle protocol)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInitResponse {
    pub id: String, // session_id
    #[serde(rename = "type")]
    pub msg_type: String, // "workspace_init_output"
    #[serde(rename = "agentType")]
    pub agent_type: String,
    #[serde(
        rename = "slashCommands",
        skip_serializing_if = "Option::is_none"
    )]
    pub slash_commands: Option<Vec<SlashCommandInfo>>,
    #[serde(rename = "mcpServers", skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agents: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugins: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(
        rename = "claudeCodeVersion",
        skip_serializing_if = "Option::is_none"
    )]
    pub claude_code_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Slash command info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashCommandInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    #[serde(default)]
    pub cached_tokens: i64,
    pub total_tokens: i64,
}

/// SDK usage data (from Result message)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SdkUsage {
    #[serde(default)]
    pub input_tokens: i64,
    #[serde(default)]
    pub output_tokens: i64,
    #[serde(default)]
    pub cache_read_input_tokens: i64,
    #[serde(default)]
    pub cache_creation_input_tokens: i64,
}

/// File operation type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileOperation {
    Create,
    Update,
    Delete,
}

/// Session status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Paused,
    Completed,
    Interrupted,
    Error,
}

/// Control request subtypes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlSubtype {
    Interrupt,
}

/// Session configuration (for ClientMessage::SessionStart)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    #[serde(default)]
    pub permission_mode: PermissionMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<i32>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

/// A single question in an AskUserQuestion event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserQuestion {
    /// Short label for the question (max 12 chars), used in tab bar
    #[serde(default)]
    pub header: String,
    /// The full question text
    pub question: String,
    /// Available options to choose from
    pub options: Vec<QuestionOption>,
    /// Whether multiple options can be selected
    #[serde(default, rename = "multiSelect")]
    pub multi_select: bool,
}

/// An option within a UserQuestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionOption {
    /// The display label for this option
    pub label: String,
    /// Description explaining what this option means
    #[serde(default)]
    pub description: String,
}

/// Answer to a user question
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuestionAnswer {
    /// Index of the question being answered
    pub question_index: usize,
    /// Selected option labels
    pub selected: Vec<String>,
}

/// Plugin information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Plugin name
    pub name: String,
    /// Plugin installation path
    pub path: String,
}

/// Session initialization data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionInitData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_servers: Vec<String>,
    #[serde(
        rename = "permissionMode",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub permission_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slash_commands: Vec<String>,
    #[serde(
        rename = "apiKeySource",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub api_key_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_code_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_style: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plugins: Vec<PluginInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

// ============================================================================
// Context Window State (for client-side tracking)
// ============================================================================

/// Warning levels for context window usage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextWarningLevel {
    /// Under 80% - normal operation
    #[default]
    Normal,
    /// 80-89% - approaching limit
    Medium,
    /// 90-94% - high usage, compaction likely soon
    High,
    /// 95%+ - critical, compaction imminent
    Critical,
}

/// Context window state for tracking usage against limits
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextWindowState {
    /// Current context usage (total tokens in context)
    pub current_tokens: i64,
    /// Maximum context window size for this model
    pub max_tokens: i64,
    /// Whether context has been compacted in this session
    pub has_compacted: bool,
    /// Number of compactions in this session
    pub compaction_count: u32,
}

impl ContextWindowState {
    /// Calculate usage percentage (0.0 to 1.0+)
    pub fn usage_percent(&self) -> f32 {
        if self.max_tokens <= 0 {
            return 0.0;
        }
        self.current_tokens as f32 / self.max_tokens as f32
    }

    /// Get warning level based on usage
    pub fn warning_level(&self) -> ContextWarningLevel {
        let pct = self.usage_percent();
        if pct >= 0.95 {
            ContextWarningLevel::Critical
        } else if pct >= 0.90 {
            ContextWarningLevel::High
        } else if pct >= 0.80 {
            ContextWarningLevel::Medium
        } else {
            ContextWarningLevel::Normal
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_event_serialization() {
        let event = AgentEvent::SessionInit {
            success: true,
            session_id: "session-123".to_string(),
            error: None,
            data: SessionInitData {
                cwd: Some("/test/path".to_string()),
                model: Some("claude-sonnet-4".to_string()),
                tools: vec!["Task".to_string(), "Bash".to_string()],
                mcp_servers: vec![],
                permission_mode: Some("default".to_string()),
                slash_commands: vec!["commit".to_string()],
                api_key_source: Some("none".to_string()),
                claude_code_version: Some("2.1.19".to_string()),
                output_style: Some("default".to_string()),
                agents: vec!["Bash".to_string()],
                skills: vec![],
                plugins: vec![PluginInfo {
                    name: "test-plugin".to_string(),
                    path: "/path/to/plugin".to_string(),
                }],
                uuid: Some("test-uuid".to_string()),
            },
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"session_init\""));
        assert!(json.contains("\"session_id\":\"session-123\""));
        // Verify flatten works - fields should be at top level, not nested under "data"
        assert!(json.contains("\"model\":\"claude-sonnet-4\""));
        assert!(json.contains("\"cwd\":\"/test/path\""));
        assert!(!json.contains("\"data\":{"));
    }

    #[test]
    fn test_token_usage_event() {
        let event = AgentEvent::TokenUsage {
            session_id: "session-123".to_string(),
            usage: TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
                cached_tokens: 20,
                total_tokens: 150,
            },
            context_window: Some(200000),
            usage_percent: Some(0.075),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"token_usage\""));
        assert!(json.contains("\"input_tokens\":100"));
    }

    #[test]
    fn test_client_message_serialization() {
        let msg = ClientMessage::UserMessage {
            session_id: "session-123".to_string(),
            content: "Hello".to_string(),
            parent_tool_use_id: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"user_message\""));
    }

    #[test]
    fn test_context_window_state() {
        let mut state = ContextWindowState {
            current_tokens: 180000,
            max_tokens: 200000,
            has_compacted: false,
            compaction_count: 0,
        };

        assert_eq!(state.usage_percent(), 0.9);
        assert_eq!(state.warning_level(), ContextWarningLevel::High);

        state.current_tokens = 195000;
        assert_eq!(state.warning_level(), ContextWarningLevel::Critical);
    }

    #[test]
    fn test_sidecar_message_serialization() {
        let msg = SidecarMessage {
            id: "session-123".to_string(),
            msg_type: "message".to_string(),
            agent_type: "claude".to_string(),
            data: serde_json::json!({"type": "assistant", "content": "Hello"}),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"id\":\"session-123\""));
        assert!(json.contains("\"type\":\"message\""));
        assert!(json.contains("\"agentType\":\"claude\""));
        assert!(json.contains("\"data\":{"));
    }

    #[test]
    fn test_sidecar_error_serialization() {
        let err = SidecarError {
            id: "session-123".to_string(),
            msg_type: "error".to_string(),
            agent_type: "claude".to_string(),
            error: "Something went wrong".to_string(),
            data: None,
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"id\":\"session-123\""));
        assert!(json.contains("\"type\":\"error\""));
        assert!(json.contains("\"agentType\":\"claude\""));
        assert!(json.contains("\"error\":\"Something went wrong\""));
        // data should not be present when None
        assert!(!json.contains("\"data\""));
    }

    #[test]
    fn test_sidecar_error_with_data() {
        let err = SidecarError {
            id: "session-123".to_string(),
            msg_type: "error".to_string(),
            agent_type: "claude".to_string(),
            error: "Error with context".to_string(),
            data: Some(serde_json::json!({"context": "additional info"})),
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"data\":{"));
        assert!(json.contains("\"context\":\"additional info\""));
    }
}
