//! Query class for handling bidirectional control protocol.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};
use tracing::error;

use crate::types::{
    CanUseTool, Error, HookCallback, HookContext, HookEvent, HookInput,
    HookMatcher, PermissionResult, Result,
    ToolPermissionContext,
};
use crate::internal::transport::WriteHalf;
use tokio::process::ChildStdin;

/// Query handles bidirectional control protocol on top of Transport.
///
/// This class manages:
/// - Control request/response routing
/// - Hook callbacks
/// - Tool permission callbacks
/// - Message streaming
/// - Initialization handshake
pub struct Query {
    write_half: Arc<Mutex<WriteHalf<ChildStdin>>>,
    read_rx: Arc<Mutex<mpsc::Receiver<serde_json::Value>>>,
    is_streaming: bool,

    // Control protocol state
    pending_requests: Arc<Mutex<HashMap<String, oneshot::Sender<serde_json::Value>>>>,
    hook_callbacks: Arc<Mutex<HashMap<String, Box<dyn HookCallback>>>>,
    request_counter: Arc<Mutex<usize>>,

    // Message stream
    message_tx: mpsc::Sender<serde_json::Value>,
    message_rx: Option<mpsc::Receiver<serde_json::Value>>,

    // Callbacks
    can_use_tool: Option<Arc<Box<dyn CanUseTool>>>,

    // State
    initialized: bool,
    closed: bool,
}

impl Query {
    /// Create a new Query instance.
    pub fn new(
        write_half: WriteHalf<ChildStdin>,
        read_rx: mpsc::Receiver<serde_json::Value>,
        is_streaming: bool,
        can_use_tool: Option<Box<dyn CanUseTool>>,
        _hooks: Option<HashMap<HookEvent, Vec<HookMatcher>>>,
    ) -> Self {
        let (message_tx, message_rx) = mpsc::channel(100);

        Self {
            write_half: Arc::new(Mutex::new(write_half)),
            read_rx: Arc::new(Mutex::new(read_rx)),
            is_streaming,
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            hook_callbacks: Arc::new(Mutex::new(HashMap::new())),
            request_counter: Arc::new(Mutex::new(0)),
            message_tx,
            message_rx: Some(message_rx),
            can_use_tool: can_use_tool.map(Arc::new),
            initialized: false,
            closed: false,
        }
    }

    /// Initialize control protocol if in streaming mode.
    pub async fn initialize(&mut self) -> Result<Option<serde_json::Value>> {
        if !self.is_streaming {
            return Ok(None);
        }

        // Build hooks configuration
        let hooks_config = serde_json::json!({});

        // Send initialize request
        let request = serde_json::json!({
            "subtype": "initialize",
            "hooks": hooks_config
        });

        let response = self.send_control_request(request, 60.0).await?;
        self.initialized = true;

        Ok(Some(response))
    }

    /// Start reading messages from transport.
    pub async fn start(&mut self) -> Result<()> {
        let read_rx = Arc::clone(&self.read_rx);
        let write_half = Arc::clone(&self.write_half);
        let message_tx = self.message_tx.clone();
        let pending_requests = Arc::clone(&self.pending_requests);
        let hook_callbacks = Arc::clone(&self.hook_callbacks);
        let can_use_tool = self.can_use_tool.clone();

        tokio::spawn(async move {
            let mut rx = read_rx.lock().await;

            while let Some(message) = rx.recv().await {
                let msg_type = message.get("type").and_then(|v| v.as_str());

                match msg_type {
                    Some("control_response") => {
                        // Handle control response
                        if let Some(response) = message.get("response") {
                            if let Some(request_id) = response.get("request_id").and_then(|v| v.as_str()) {
                                let mut pending = pending_requests.lock().await;
                                if let Some(tx) = pending.remove(request_id) {
                                    let _ = tx.send(response.clone());
                                }
                            }
                        }
                    }
                    Some("control_request") => {
                        // Handle incoming control request
                        let write_half_clone = Arc::clone(&write_half);
                        let hook_callbacks_clone = Arc::clone(&hook_callbacks);
                        let can_use_tool_clone = can_use_tool.clone();

                        tokio::spawn(async move {
                            if let Err(e) = Self::handle_control_request(
                                message,
                                write_half_clone,
                                hook_callbacks_clone,
                                can_use_tool_clone,
                            ).await {
                                error!("Failed to handle control request: {}", e);
                            }
                        });
                    }
                    _ => {
                        // Regular message - send to stream
                        if message_tx.send(message).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Handle incoming control request from CLI.
    async fn handle_control_request(
        request: serde_json::Value,
        write_half: Arc<Mutex<WriteHalf<ChildStdin>>>,
        hook_callbacks: Arc<Mutex<HashMap<String, Box<dyn HookCallback>>>>,
        can_use_tool: Option<Arc<Box<dyn CanUseTool>>>,
    ) -> Result<()> {
        let request_obj = request.as_object()
            .ok_or_else(|| Error::ControlProtocol("Invalid control request".to_string()))?;

        let request_id = request_obj.get("request_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::ControlProtocol("Missing request_id".to_string()))?
            .to_string();

        let request_data = request_obj.get("request")
            .ok_or_else(|| Error::ControlProtocol("Missing request data".to_string()))?;

        let subtype = request_data.get("subtype")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::ControlProtocol("Missing subtype".to_string()))?;

        let response_data = match subtype {
            "can_use_tool" => {
                Self::handle_permission_request(request_data, can_use_tool).await?
            }
            "hook_callback" => {
                Self::handle_hook_callback(request_data, hook_callbacks).await?
            }
            _ => {
                return Err(Error::ControlProtocol(format!("Unsupported subtype: {}", subtype)));
            }
        };

        // Send success response
        let response = serde_json::json!({
            "type": "control_response",
            "response": {
                "subtype": "success",
                "request_id": request_id,
                "response": response_data
            }
        });

        let response_str = serde_json::to_string(&response)? + "\n";
        let mut write_guard = write_half.lock().await;
        write_guard.write(&response_str).await?;

        Ok(())
    }

    /// Handle permission request.
    async fn handle_permission_request(
        request_data: &serde_json::Value,
        can_use_tool: Option<Arc<Box<dyn CanUseTool>>>,
    ) -> Result<serde_json::Value> {
        let tool_name = request_data.get("tool_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::ControlProtocol("Missing tool_name".to_string()))?;

        let input = request_data.get("input")
            .ok_or_else(|| Error::ControlProtocol("Missing input".to_string()))?;

        let can_use_tool = can_use_tool
            .ok_or_else(|| Error::ControlProtocol("canUseTool callback not provided".to_string()))?;

        let context = ToolPermissionContext {
            signal: None,
            suggestions: vec![],
        };

        let result = can_use_tool.can_use(tool_name, input, &context).await?;

        let response = match result {
            PermissionResult::Allow(allow) => {
                serde_json::json!({
                    "behavior": "allow",
                    "updatedInput": allow.updated_input.unwrap_or_else(|| input.clone()),
                    "updatedPermissions": allow.updated_permissions
                })
            }
            PermissionResult::Deny(deny) => {
                serde_json::json!({
                    "behavior": "deny",
                    "message": deny.message,
                    "interrupt": deny.interrupt
                })
            }
        };

        Ok(response)
    }

    /// Handle hook callback.
    async fn handle_hook_callback(
        request_data: &serde_json::Value,
        hook_callbacks: Arc<Mutex<HashMap<String, Box<dyn HookCallback>>>>,
    ) -> Result<serde_json::Value> {
        let callback_id = request_data.get("callback_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::ControlProtocol("Missing callback_id".to_string()))?;

        let input = request_data.get("input")
            .ok_or_else(|| Error::ControlProtocol("Missing input".to_string()))?;

        let tool_use_id = request_data.get("tool_use_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let callbacks = hook_callbacks.lock().await;
        let callback = callbacks.get(callback_id)
            .ok_or_else(|| Error::ControlProtocol(format!("Hook callback not found: {}", callback_id)))?;

        // Parse hook input
        let hook_input: HookInput = serde_json::from_value(input.clone())?;

        let context = HookContext {
            signal: None,
        };

        let output = callback.call(hook_input, tool_use_id, context).await?;

        // Convert to JSON
        let output_json = serde_json::to_value(&output)?;

        Ok(output_json)
    }

    /// Send control request to CLI and wait for response.
    async fn send_control_request(
        &mut self,
        request: serde_json::Value,
        timeout_secs: f64,
    ) -> Result<serde_json::Value> {
        if !self.is_streaming {
            return Err(Error::ControlProtocol("Control requests require streaming mode".to_string()));
        }

        // Generate unique request ID
        let mut counter = self.request_counter.lock().await;
        *counter += 1;
        let request_id = format!("req_{}_{}", *counter, uuid::Uuid::new_v4());
        drop(counter);

        // Create oneshot channel for response
        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending_requests.lock().await;
            pending.insert(request_id.clone(), tx);
        }

        // Build and send request
        let control_request = serde_json::json!({
            "type": "control_request",
            "request_id": request_id,
            "request": request
        });

        let request_str = serde_json::to_string(&control_request)? + "\n";

        let mut write_guard = self.write_half.lock().await;
        write_guard.write(&request_str).await?;
        drop(write_guard); // Release lock before waiting for response

        // Wait for response with timeout
        let response = tokio::time::timeout(
            std::time::Duration::from_secs_f64(timeout_secs),
            rx
        ).await
            .map_err(|_| Error::Timeout(format!("Control request timeout: {:?}", request.get("subtype"))))?
            .map_err(|_| Error::ControlProtocol("Response channel closed".to_string()))?;

        // Check for error response
        if let Some(subtype) = response.get("subtype").and_then(|v| v.as_str()) {
            if subtype == "error" {
                let error_msg = response.get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error");
                return Err(Error::ControlProtocol(error_msg.to_string()));
            }
        }

        Ok(response.get("response").cloned().unwrap_or(serde_json::json!({})))
    }

    /// Send interrupt control request.
    pub async fn interrupt(&mut self) -> Result<()> {
        let request = serde_json::json!({
            "subtype": "interrupt"
        });
        self.send_control_request(request, 60.0).await?;
        Ok(())
    }

    /// Change permission mode.
    pub async fn set_permission_mode(&mut self, mode: &str) -> Result<()> {
        let request = serde_json::json!({
            "subtype": "set_permission_mode",
            "mode": mode
        });
        self.send_control_request(request, 60.0).await?;
        Ok(())
    }

    /// Change the AI model.
    pub async fn set_model(&mut self, model: Option<&str>) -> Result<()> {
        let request = serde_json::json!({
            "subtype": "set_model",
            "model": model
        });
        self.send_control_request(request, 60.0).await?;
        Ok(())
    }

    /// Rewind tracked files to their state at a specific user message.
    pub async fn rewind_files(&mut self, user_message_id: &str) -> Result<()> {
        let request = serde_json::json!({
            "subtype": "rewind_files",
            "user_message_id": user_message_id
        });
        self.send_control_request(request, 60.0).await?;
        Ok(())
    }

    /// Stream input messages to transport.
    pub async fn stream_input(&mut self, mut input_rx: mpsc::Receiver<serde_json::Value>) -> Result<()> {
        while let Some(message) = input_rx.recv().await {
            if self.closed {
                break;
            }

            let message_str = serde_json::to_string(&message)? + "\n";
            let mut write_guard = self.write_half.lock().await;
            write_guard.write(&message_str).await?;
        }

        Ok(())
    }

    /// Receive SDK messages (not control messages).
    pub fn receive_messages(&mut self) -> Option<mpsc::Receiver<serde_json::Value>> {
        self.message_rx.take()
    }

    /// Write raw data to transport.
    pub async fn write(&mut self, data: &str) -> Result<()> {
        let mut write_guard = self.write_half.lock().await;
        write_guard.write(data).await
    }

    /// Get initialization result.
    pub fn get_initialization_result(&self) -> Option<serde_json::Value> {
        // Store initialization result when we implement it
        None
    }

    /// Close the query and transport.
    pub async fn close(&mut self) -> Result<()> {
        self.closed = true;
        // WriteHalf will be dropped automatically, closing the write side
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full integration tests would require a mock transport
    // These are basic structural tests

    #[test]
    fn test_query_creation() {
        // This test just verifies the structure compiles
        // Real tests would need mock transport
    }
}
