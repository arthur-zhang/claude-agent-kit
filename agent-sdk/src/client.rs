//! Claude SDK Client for interacting with Claude Code.

use async_stream::stream;
use futures::Stream;
use std::pin::Pin;
use tokio::sync::mpsc;

use crate::internal::transport::{
    ProcessHandle, PromptInput as TransportPromptInput, SubprocessCLITransport,
};
use crate::internal::Query;
use crate::types::{ClaudeAgentOptions, Error, Message, Result};

/// Prompt input for client operations.
pub enum ClientPromptInput {
    /// String prompt.
    String(String),
    /// Streaming prompt (receiver of messages).
    Stream(mpsc::Receiver<serde_json::Value>),
    /// No initial prompt (for interactive mode).
    None,
}

/// Client for bidirectional, interactive conversations with Claude Code.
///
/// This client provides full control over the conversation flow with support
/// for streaming, interrupts, and dynamic message sending.
///
/// # Key Features
///
/// - **Bidirectional**: Send and receive messages at any time
/// - **Stateful**: Maintains conversation context across messages
/// - **Interactive**: Send follow-ups based on responses
/// - **Control flow**: Support for interrupts and session management
///
/// # When to Use ClaudeClient
///
/// - Building chat interfaces or conversational UIs
/// - Interactive debugging or exploration sessions
/// - Multi-turn conversations with context
/// - When you need to react to Claude's responses
/// - Real-time applications with user input
/// - When you need interrupt capabilities
///
/// # Example
///
/// ```rust,no_run
/// use claude_agent_sdk::{ClaudeClient, ClaudeAgentOptions};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let options = ClaudeAgentOptions::new();
///     let mut client = ClaudeClient::new(options);
///
///     // Connect to Claude
///     client.connect(None).await?;
///
///     // Send a query
///     client.query_string("What is the capital of France?", None).await?;
///
///     // Receive response (use StreamExt trait for .next())
///     // let mut response_stream = client.receive_response().await?;
///     // Process messages from the stream...
///
///     // Disconnect
///     client.disconnect().await?;
///     Ok(())
/// }
/// ```
pub struct ClaudeClient {
    options: ClaudeAgentOptions,
    query: Option<Query>,
    stderr_rx: Option<mpsc::Receiver<String>>,
    process_handle: Option<ProcessHandle>,
}

impl ClaudeClient {
    /// Create a new Claude SDK client.
    ///
    /// # Arguments
    /// * `options` - Configuration options for the client
    pub fn new(options: ClaudeAgentOptions) -> Self {
        Self {
            options,
            query: None,
            stderr_rx: None,
            process_handle: None,
        }
    }

    /// Connect to Claude with an optional prompt or message stream.
    ///
    /// # Arguments
    /// * `prompt` - Optional initial prompt (string, stream, or None for interactive mode)
    ///
    /// # Errors
    /// Returns an error if connection fails or configuration is invalid
    pub async fn connect(&mut self, prompt: Option<ClientPromptInput>) -> Result<()> {
        // Validate configuration
        // Note: We can't clone ClaudeAgentOptions because it contains trait objects
        // So we'll work with references and take ownership of callbacks

        if self.options.can_use_tool.is_some() {
            // canUseTool callback requires streaming mode
            if matches!(prompt, Some(ClientPromptInput::String(_))) {
                return Err(Error::InvalidConfig(
                    "can_use_tool callback requires streaming mode. \
                    Please provide prompt as a Stream instead of a String."
                        .to_string(),
                ));
            }

            // canUseTool and permission_prompt_tool_name are mutually exclusive
            if self.options.permission_prompt_tool_name.is_some() {
                return Err(Error::InvalidConfig(
                    "can_use_tool callback cannot be used with permission_prompt_tool_name. \
                    Please use one or the other."
                        .to_string(),
                ));
            }

            // Automatically set permission_prompt_tool_name to "stdio" for control protocol
            self.options.permission_prompt_tool_name = Some("stdio".to_string());
        }

        // Create empty stream for interactive mode if no prompt provided
        let (empty_tx, empty_rx) = mpsc::channel(1);
        drop(empty_tx); // Close immediately to create empty stream

        let actual_prompt = match prompt {
            Some(ClientPromptInput::String(s)) => TransportPromptInput::String(s),
            Some(ClientPromptInput::Stream(rx)) => TransportPromptInput::Stream(rx),
            Some(ClientPromptInput::None) | None => TransportPromptInput::Stream(empty_rx),
        };

        // Extract callbacks before creating transport
        let can_use_tool = self.options.can_use_tool.take();
        let hooks = self.options.hooks.take();

        // Create and connect transport
        let mut transport = SubprocessCLITransport::new(actual_prompt, self.options.clone())?;
        transport.connect().await?;

        // Split transport into independent halves
        let (read_half, write_half, stderr_half, process_handle) = transport.split()?;

        // Start reading messages and stderr
        let read_rx = read_half.read_messages();
        let stderr_rx = stderr_half.read_lines();

        // Create Query with write_half and read_rx
        let mut query = Query::new(
            write_half,
            read_rx,
            true, // ClaudeClient always uses streaming mode
            can_use_tool,
            hooks,
        );

        // Start reading messages and initialize
        query.start().await?;
        query.initialize().await?;

        self.query = Some(query);
        self.stderr_rx = Some(stderr_rx);
        self.process_handle = Some(process_handle);

        Ok(())
    }

    /// Send a new request with a string prompt.
    ///
    /// # Arguments
    /// * `prompt` - The message to send
    /// * `session_id` - Session identifier for the conversation
    ///
    /// # Errors
    /// Returns an error if not connected or write fails
    pub async fn query_string(&mut self, prompt: &str, session_id: Option<String>) -> Result<()> {
        let session_id = session_id.unwrap_or("default".into());
        let query = self.query.as_mut().ok_or_else(|| {
            Error::CLIConnection("Not connected. Call connect() first.".to_string())
        })?;

        let message = serde_json::json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": prompt
            },
            "parent_tool_use_id": null,
            "session_id": session_id
        });

        let message_str = serde_json::to_string(&message)? + "\n";
        query.write(&message_str).await?;

        Ok(())
    }

    /// Send a new request with a stream of messages.
    ///
    /// # Arguments
    /// * `messages` - Stream of message dictionaries
    /// * `session_id` - Session identifier for the conversation
    ///
    /// # Errors
    /// Returns an error if not connected or write fails
    pub async fn query_stream(
        &mut self,
        mut messages: mpsc::Receiver<serde_json::Value>,
        session_id: &str,
    ) -> Result<()> {
        let query = self.query.as_mut().ok_or_else(|| {
            Error::CLIConnection("Not connected. Call connect() first.".to_string())
        })?;

        while let Some(mut msg) = messages.recv().await {
            // Ensure session_id is set on each message
            if let Some(obj) = msg.as_object_mut() {
                if !obj.contains_key("session_id") {
                    obj.insert("session_id".to_string(), serde_json::json!(session_id));
                }
            }

            let message_str = serde_json::to_string(&msg)? + "\n";
            query.write(&message_str).await?;
        }

        Ok(())
    }

    /// Send interrupt signal.
    ///
    /// # Errors
    /// Returns an error if not connected
    pub async fn interrupt(&mut self) -> Result<()> {
        let query = self.query.as_mut().ok_or_else(|| {
            Error::CLIConnection("Not connected. Call connect() first.".to_string())
        })?;

        query.interrupt().await
    }

    /// Change permission mode during conversation.
    ///
    /// # Arguments
    /// * `mode` - The permission mode to set (e.g., "default", "acceptEdits", "bypassPermissions")
    ///
    /// # Errors
    /// Returns an error if not connected
    pub async fn set_permission_mode(&mut self, mode: &str) -> Result<()> {
        let query = self.query.as_mut().ok_or_else(|| {
            Error::CLIConnection("Not connected. Call connect() first.".to_string())
        })?;

        query.set_permission_mode(mode).await
    }

    /// Change the AI model during conversation.
    ///
    /// # Arguments
    /// * `model` - The model to use, or None to use default
    ///
    /// # Example
    /// ```rust,no_run
    /// # use claude_agent_sdk::{ClaudeClient, ClaudeAgentOptions};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let mut client = ClaudeClient::new(ClaudeAgentOptions::new());
    /// # client.connect(None).await?;
    /// // Switch to a different model
    /// client.set_model(Some("claude-sonnet-4-5")).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// Returns an error if not connected
    pub async fn set_model(&mut self, model: Option<&str>) -> Result<()> {
        let query = self.query.as_mut().ok_or_else(|| {
            Error::CLIConnection("Not connected. Call connect() first.".to_string())
        })?;

        query.set_model(model).await
    }

    /// Rewind tracked files to their state at a specific user message.
    ///
    /// Requires `enable_file_checkpointing=true` in options.
    ///
    /// # Arguments
    /// * `user_message_id` - UUID of the user message to rewind to
    ///
    /// # Errors
    /// Returns an error if not connected
    pub async fn rewind_files(&mut self, user_message_id: &str) -> Result<()> {
        let query = self.query.as_mut().ok_or_else(|| {
            Error::CLIConnection("Not connected. Call connect() first.".to_string())
        })?;

        query.rewind_files(user_message_id).await
    }

    /// Get server initialization info including available commands and output styles.
    ///
    /// Returns initialization information from the Claude Code server including:
    /// - Available commands (slash commands, system commands, etc.)
    /// - Current and available output styles
    /// - Server capabilities
    ///
    /// # Returns
    /// Dictionary with server info, or None if not available
    ///
    /// # Errors
    /// Returns an error if not connected
    pub async fn get_server_info(&self) -> Result<Option<serde_json::Value>> {
        let query = self.query.as_ref().ok_or_else(|| {
            Error::CLIConnection("Not connected. Call connect() first.".to_string())
        })?;

        Ok(query.get_initialization_result())
    }

    /// Receive all messages from Claude.
    ///
    /// Returns a stream of messages that continues until the connection is closed.
    ///
    /// # Errors
    /// Returns an error if not connected
    pub async fn receive_messages(
        &mut self,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Message>> + Send + '_>>> {
        let query = self.query.as_mut().ok_or_else(|| {
            Error::CLIConnection("Not connected. Call connect() first.".to_string())
        })?;

        let mut message_rx = query
            .receive_messages()
            .ok_or_else(|| Error::Unknown("Failed to get message receiver".to_string()))?;

        let message_stream = stream! {
            while let Some(data) = message_rx.recv().await {
                match crate::internal::parse_message(data) {
                    Ok(message) => yield Ok(message),
                    Err(e) => {
                        yield Err(e);
                        break;
                    }
                }
            }
        };

        Ok(Box::pin(message_stream))
    }

    /// Receive messages until and including a ResultMessage.
    ///
    /// This is a convenience method that automatically terminates after receiving
    /// a ResultMessage, which indicates the response is complete.
    ///
    /// # Errors
    /// Returns an error if not connected
    pub async fn receive_response(
        &mut self,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Message>> + Send + '_>>> {
        let mut messages = self.receive_messages().await?;

        let response_stream = stream! {
            use futures::StreamExt;

            while let Some(result) = messages.next().await {
                match result {
                    Ok(message) => {
                        let is_result = matches!(message, Message::Result(_));
                        yield Ok(message);
                        if is_result {
                            break;
                        }
                    }
                    Err(e) => {
                        yield Err(e);
                        break;
                    }
                }
            }
        };

        Ok(Box::pin(response_stream))
    }

    /// Disconnect from Claude.
    ///
    /// # Errors
    /// Returns an error if cleanup fails
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut query) = self.query.take() {
            query.close().await?;
        }
        Ok(())
    }

    /// Get the stderr receiver (can only be called once).
    ///
    /// Returns the receiver for stderr lines from the Claude CLI process.
    /// This can only be called once - subsequent calls will return None.
    ///
    /// # Returns
    /// The stderr receiver, or None if already taken or not connected
    pub fn stderr_receiver(&mut self) -> Option<mpsc::Receiver<String>> {
        self.stderr_rx.take()
    }

    /// Get the process handle (can only be called once).
    ///
    /// Returns the handle for managing the Claude CLI process lifecycle.
    /// This can only be called once - subsequent calls will return None.
    ///
    /// # Returns
    /// The process handle, or None if already taken or not connected
    pub fn process_handle(&mut self) -> Option<ProcessHandle> {
        self.process_handle.take()
    }
}

// Implement Drop to ensure cleanup
impl Drop for ClaudeClient {
    fn drop(&mut self) {
        // Note: We can't call async methods in Drop
        // Users should call disconnect() explicitly
        if self.query.is_some() {
            eprintln!("Warning: ClaudeClient dropped without calling disconnect()");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let options = ClaudeAgentOptions::new();
        let client = ClaudeClient::new(options);
        assert!(client.query.is_none());
    }

    #[tokio::test]
    async fn test_connect_without_prompt() {
        let options = ClaudeAgentOptions::new();
        let mut client = ClaudeClient::new(options);

        // This will fail without a real CLI, but tests the structure
        let result = client.connect(None).await;
        // Just verify it compiles and returns a Result
        let _ = result;
    }
}
