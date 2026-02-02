//! Claude SDK Client for interacting with Claude Code.

use async_stream::stream;
use futures::Stream;
use std::pin::Pin;
use tokio::sync::mpsc;
use tracing::info;

use crate::internal::transport::{
    ProcessHandle, PromptInput as TransportPromptInput, SubprocessCLITransport,
};
use crate::types::{ClaudeAgentOptions, Error, InputMessage, Message, Result};

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
///     client.send_to_cc("What is the capital of France?", None).await?;
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
/// Client for bidirectional, interactive conversations with Claude Code.
///
/// This client provides full control over the conversation flow with support
/// for streaming, interrupts, and dynamic message sending.
///
/// Refactored to use Actor Pattern for robust full-duplex communication.
pub struct ClaudeClient {
    options: ClaudeAgentOptions,
    // Channel to send commands to the session actor
    command_tx: Option<mpsc::Sender<crate::internal::ClientCommand>>,
    // Channel to receive protocol events (shared with subscribers)
    event_rx: Option<tokio::sync::broadcast::Receiver<crate::types::ProtocolMessage>>,
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
            command_tx: None,
            event_rx: None,
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
        if self.options.can_use_tool.is_some() {
            info!("🔐 can_use_tool callback is set, configuring permission prompt tool");
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
            info!("🔐 Set permission_prompt_tool_name to 'stdio'");
        } else {
            info!("⚠️ can_use_tool callback is NOT set");
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
        // Hooks support will be added to AgentSession later if needed, currently unused in refactor
        let _hooks = self.options.hooks.take();

        // Create and connect transport
        let mut transport = SubprocessCLITransport::new(actual_prompt, self.options.clone())?;
        transport.connect().await?;

        // Split transport into independent halves
        let (read_half, write_half, stderr_half, process_handle) = transport.split()?;

        // Start reading messages
        let read_rx = read_half.read_messages();
        let stderr_rx = stderr_half.read_lines();

        // Create channels for Actor communication
        let (command_tx, command_rx) = mpsc::channel(100);

        // Create and spawn AgentSession actor
        let (session, event_rx) = crate::internal::AgentSession::new(
            command_rx,
            read_rx,
            write_half,
            can_use_tool,
        );

        tokio::spawn(session.run());


        self.command_tx = Some(command_tx);
        self.event_rx = Some(event_rx);
        self.stderr_rx = Some(stderr_rx);
        self.process_handle = Some(process_handle);

        Ok(())
    }

    /// Send a new request with a string prompt.
    pub async fn send_to_cc(&self, prompt: &str, session_id: Option<String>) -> Result<()> {
        let session_id = session_id.unwrap_or("default".into());
        self.send_command(crate::internal::ClientCommand::SendUserMessage {
            message: prompt.to_string(),
            session_id,
        }).await
    }

    /// Send a single input message (e.g., tool result).
    ///
    /// # Arguments
    /// * `message` - Input message to send
    pub async fn send_input_message(&self, message: InputMessage) -> Result<()> {
        self.send_command(crate::internal::ClientCommand::SendInputMessage(message)).await
    }

    /// Send a new request with a stream of messages.
    ///
    /// # Arguments
    /// * `messages` - Stream of message dictionaries
    /// * `session_id` - Session identifier
    pub async fn query_stream(
        &self,
        mut messages: mpsc::Receiver<InputMessage>,
        session_id: &str,
    ) -> Result<()> {
        while let Some(mut msg) = messages.recv().await {
            if msg.session_id != session_id {
                msg.session_id = session_id.to_string();
            }
             self.send_command(crate::internal::ClientCommand::SendInputMessage(msg)).await?;
        }
        Ok(())
    }

    /// Send interrupt signal and return the request_id for tracking the response.
    pub async fn interrupt(&self) -> Result<String> {
        let request_id = format!("interrupt_{}", uuid::Uuid::new_v4());
        self.send_command(crate::internal::ClientCommand::Interrupt(request_id.clone())).await?;
        Ok(request_id)
    }

    /// Get initialization data from the Claude connection.
    pub async fn get_server_info(&self) -> Result<Option<serde_json::Value>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        if let Some(cmd_tx) = &self.command_tx {
            cmd_tx.send(crate::internal::ClientCommand::GetInitData(tx)).await
                .map_err(|_| Error::CLIConnection("Actor closed".to_string()))?;
            rx.await.map_err(|_| Error::CLIConnection("Actor response failed".to_string()))
        } else {
            Err(Error::CLIConnection("Not connected".to_string()))
        }
    }

    /// Receive all messages from Claude (as broadcast stream).
    pub async fn receive_messages_from_cc_stdout(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Message>> + Send>>> {
        let mut rx = self.event_rx.as_ref()
            .ok_or_else(|| Error::CLIConnection("Not connected".to_string()))?
            .resubscribe();

        let message_stream = stream! {
            while let Ok(protocol_msg) = rx.recv().await {
                match crate::internal::protocol_message_to_message(protocol_msg) {
                    Ok(message) => yield Ok(message),
                    Err(e) => {
                         // Simplify error handling for stream
                         eprintln!("Error converting message: {:?}", e);
                    }
                }
            }
        };

        Ok(Box::pin(message_stream))
    }

    /// Receive all protocol messages from Claude (including ControlResponse).
    ///
    /// Unlike `receive_messages_from_cc_stdout`, this method returns raw ProtocolMessages
    /// including ControlResponse, which is useful for tracking request-response pairs
    /// (e.g., interrupt confirmation).
    pub async fn receive_protocol_messages(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<crate::types::ProtocolMessage>> + Send>>> {
        let mut rx = self.event_rx.as_ref()
            .ok_or_else(|| Error::CLIConnection("Not connected".to_string()))?
            .resubscribe();

        let protocol_stream = stream! {
            while let Ok(protocol_msg) = rx.recv().await {
                yield Ok(protocol_msg);
            }
        };

        Ok(Box::pin(protocol_stream))
    }
    
    // Legacy support method for receive_response (kept similar to before)
    pub async fn receive_response(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Message>> + Send>>> {
        let mut messages = self.receive_messages_from_cc_stdout().await?;

        let response_stream = stream! {
            use futures::StreamExt;
            while let Some(result) = messages.next().await {
                match result {
                    Ok(message) => {
                        let is_result = matches!(message, Message::Result(_));
                        yield Ok(message);
                        if is_result { break; }
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

    pub async fn set_permission_mode(&self, mode: &str) -> Result<()> {
        self.send_command(crate::internal::ClientCommand::SetPermissionMode(mode.to_string())).await
    }

    pub async fn set_model(&self, model: Option<&str>) -> Result<()> {
        self.send_command(crate::internal::ClientCommand::SetModel(model.map(String::from))).await
    }

    pub async fn rewind_files(&self, user_message_id: &str) -> Result<()> {
        self.send_command(crate::internal::ClientCommand::RewindFiles(user_message_id.to_string())).await
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(tx) = self.command_tx.take() {
            let _ = tx.send(crate::internal::ClientCommand::Disconnect).await;
        }
        Ok(())
    }

    pub fn stderr_receiver(&mut self) -> Option<mpsc::Receiver<String>> {
        self.stderr_rx.take()
    }

    pub fn process_handle(&mut self) -> Option<ProcessHandle> {
        self.process_handle.take()
    }

    // Helper to send commands
    async fn send_command(&self, cmd: crate::internal::ClientCommand) -> Result<()> {
        if let Some(tx) = &self.command_tx {
            tx.send(cmd).await.map_err(|_| Error::CLIConnection("Actor closed".to_string()))
        } else {
            Err(Error::CLIConnection("Not connected".to_string()))
        }
    }
}


// Implement Drop to ensure cleanup
impl Drop for ClaudeClient {
    fn drop(&mut self) {
        // Note: We can't call async methods in Drop
        // Users should call disconnect() explicitly
        if self.command_tx.is_some() {
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
        assert!(client.command_tx.is_none());
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
