//! SDK Control Protocol types for Claude Agent SDK.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::hooks::HookEvent;
use crate::types::permissions::PermissionUpdate;

/// SDK Control Interrupt Request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDKControlInterruptRequest {
    pub subtype: String, // "interrupt"
}

/// SDK Control Permission Request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDKControlPermissionRequest {
    pub subtype: String, // "can_use_tool"
    pub tool_name: String,
    pub input: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_suggestions: Option<Vec<PermissionUpdate>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_path: Option<String>,
}

/// SDK Control Initialize Request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDKControlInitializeRequest {
    pub subtype: String, // "initialize"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<HashMap<HookEvent, serde_json::Value>>,
}

/// SDK Control Set Permission Mode Request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDKControlSetPermissionModeRequest {
    pub subtype: String, // "set_permission_mode"
    pub mode: String,
}

/// SDK Hook Callback Request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDKHookCallbackRequest {
    pub subtype: String, // "hook_callback"
    pub callback_id: String,
    pub input: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
}

/// SDK Control MCP Message Request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDKControlMcpMessageRequest {
    pub subtype: String, // "mcp_message"
    pub server_name: String,
    pub message: serde_json::Value,
}

/// SDK Control Rewind Files Request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDKControlRewindFilesRequest {
    pub subtype: String, // "rewind_files"
    pub user_message_id: String,
}

/// SDK Control Request (union of all request types).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum SDKControlRequestType {
    Interrupt,
    #[serde(rename = "can_use_tool")]
    CanUseTool {
        tool_name: String,
        input: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        permission_suggestions: Option<Vec<PermissionUpdate>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        blocked_path: Option<String>,
    },
    Initialize {
        #[serde(skip_serializing_if = "Option::is_none")]
        hooks: Option<HashMap<HookEvent, serde_json::Value>>,
    },
    SetPermissionMode {
        mode: String,
    },
    HookCallback {
        callback_id: String,
        input: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_use_id: Option<String>,
    },
    McpMessage {
        server_name: String,
        message: serde_json::Value,
    },
    RewindFiles {
        user_message_id: String,
    },
}

/// SDK Control Request wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDKControlRequest {
    #[serde(rename = "type")]
    pub type_: String, // "control_request"
    pub request_id: String,
    #[serde(flatten)]
    pub request: SDKControlRequestType,
}

/// Control Response (success).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlResponse {
    pub subtype: String, // "success"
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<serde_json::Value>,
}

/// Control Error Response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlErrorResponse {
    pub subtype: String, // "error"
    pub request_id: String,
    pub error: String,
}

/// SDK Control Response (union of success and error).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "subtype", rename_all = "lowercase")]
pub enum SDKControlResponseType {
    Success {
        request_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        response: Option<serde_json::Value>,
    },
    Error {
        request_id: String,
        error: String,
    },
}

/// SDK Control Response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDKControlResponse {
    #[serde(rename = "type")]
    pub type_: String, // "control_response"
    #[serde(flatten)]
    pub response: SDKControlResponseType,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interrupt_request_serialization() {
        let request = SDKControlRequest {
            type_: "control_request".to_string(),
            request_id: "req-123".to_string(),
            request: SDKControlRequestType::Interrupt,
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["type"], "control_request");
        assert_eq!(json["request_id"], "req-123");
        assert_eq!(json["subtype"], "interrupt");
    }

    #[test]
    fn test_permission_request_serialization() {
        let request = SDKControlRequest {
            type_: "control_request".to_string(),
            request_id: "req-456".to_string(),
            request: SDKControlRequestType::CanUseTool {
                tool_name: "Bash".to_string(),
                input: serde_json::json!({"command": "ls"}),
                permission_suggestions: None,
                blocked_path: Some("/etc/passwd".to_string()),
            },
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["subtype"], "can_use_tool");
        assert_eq!(json["tool_name"], "Bash");
        assert_eq!(json["blocked_path"], "/etc/passwd");
    }

    #[test]
    fn test_initialize_request_serialization() {
        let mut hooks = HashMap::new();
        hooks.insert(HookEvent::PreToolUse, serde_json::json!({"test": "data"}));

        let request = SDKControlRequest {
            type_: "control_request".to_string(),
            request_id: "req-789".to_string(),
            request: SDKControlRequestType::Initialize { hooks: Some(hooks) },
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["subtype"], "initialize");
        assert!(json["hooks"].is_object());
    }

    #[test]
    fn test_set_permission_mode_request_serialization() {
        let request = SDKControlRequest {
            type_: "control_request".to_string(),
            request_id: "req-101".to_string(),
            request: SDKControlRequestType::SetPermissionMode {
                mode: "plan".to_string(),
            },
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["subtype"], "set_permission_mode");
        assert_eq!(json["mode"], "plan");
    }

    #[test]
    fn test_hook_callback_request_serialization() {
        let request = SDKControlRequest {
            type_: "control_request".to_string(),
            request_id: "req-202".to_string(),
            request: SDKControlRequestType::HookCallback {
                callback_id: "hook-123".to_string(),
                input: serde_json::json!({"data": "test"}),
                tool_use_id: Some("tool-456".to_string()),
            },
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["subtype"], "hook_callback");
        assert_eq!(json["callback_id"], "hook-123");
        assert_eq!(json["tool_use_id"], "tool-456");
    }

    #[test]
    fn test_mcp_message_request_serialization() {
        let request = SDKControlRequest {
            type_: "control_request".to_string(),
            request_id: "req-303".to_string(),
            request: SDKControlRequestType::McpMessage {
                server_name: "test-server".to_string(),
                message: serde_json::json!({"method": "test"}),
            },
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["subtype"], "mcp_message");
        assert_eq!(json["server_name"], "test-server");
    }

    #[test]
    fn test_rewind_files_request_serialization() {
        let request = SDKControlRequest {
            type_: "control_request".to_string(),
            request_id: "req-404".to_string(),
            request: SDKControlRequestType::RewindFiles {
                user_message_id: "msg-789".to_string(),
            },
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["subtype"], "rewind_files");
        assert_eq!(json["user_message_id"], "msg-789");
    }

    #[test]
    fn test_success_response_serialization() {
        let response = SDKControlResponse {
            type_: "control_response".to_string(),
            response: SDKControlResponseType::Success {
                request_id: "req-123".to_string(),
                response: Some(serde_json::json!({"result": "ok"})),
            },
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["type"], "control_response");
        assert_eq!(json["subtype"], "success");
        assert_eq!(json["request_id"], "req-123");
        assert_eq!(json["response"]["result"], "ok");
    }

    #[test]
    fn test_error_response_serialization() {
        let response = SDKControlResponse {
            type_: "control_response".to_string(),
            response: SDKControlResponseType::Error {
                request_id: "req-456".to_string(),
                error: "Something went wrong".to_string(),
            },
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["type"], "control_response");
        assert_eq!(json["subtype"], "error");
        assert_eq!(json["request_id"], "req-456");
        assert_eq!(json["error"], "Something went wrong");
    }
}
