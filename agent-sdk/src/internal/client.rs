//! Internal client implementation.

use async_stream::stream;
use futures::Stream;
use tokio::sync::mpsc;

use crate::types::{ClaudeAgentOptions, Message, Result};
use super::{message_parser::parse_message, Query, Transport};
use super::transport::SubprocessCLITransport;

/// Prompt input type.
pub enum PromptInput {
    /// String prompt.
    String(String),
    /// Streaming prompt (receiver of messages).
    Stream(mpsc::Receiver<serde_json::Value>),
}

/// Internal client implementation.
pub struct InternalClient;

impl InternalClient {
    /// Create a new internal client.
    pub fn new() -> Self {
        Self
    }

    /// Process a query through transport and Query.
    ///
    /// # Arguments
    /// * `prompt` - Prompt input (string or stream)
    /// * `options` - Claude agent options
    /// * `transport` - Optional custom transport (defaults to SubprocessCLITransport)
    ///
    /// # Returns
    /// Stream of parsed messages
    pub async fn process_query(
        &self,
        prompt: PromptInput,
        options: ClaudeAgentOptions,
        transport: Option<Box<dyn Transport>>,
    ) -> Result<impl Stream<Item = Result<Message>>> {
        // Validate configuration
        let mut configured_options = options;

        // Determine if streaming mode before consuming prompt
        let is_streaming = matches!(prompt, PromptInput::Stream(_));

        if configured_options.can_use_tool.is_some() {
            // canUseTool callback requires streaming mode
            if !is_streaming {
                return Err(crate::types::Error::InvalidConfig(
                    "can_use_tool callback requires streaming mode. \
                    Please provide prompt as a Stream instead of a String.".to_string()
                ));
            }

            // canUseTool and permission_prompt_tool_name are mutually exclusive
            if configured_options.permission_prompt_tool_name.is_some() {
                return Err(crate::types::Error::InvalidConfig(
                    "can_use_tool callback cannot be used with permission_prompt_tool_name. \
                    Please use one or the other.".to_string()
                ));
            }

            // Automatically set permission_prompt_tool_name to "stdio" for control protocol
            configured_options.permission_prompt_tool_name = Some("stdio".to_string());
        }

        // Extract callbacks before creating transport
        let can_use_tool = configured_options.can_use_tool.take();
        let hooks = configured_options.hooks.take();

        // Create or use provided transport
        let chosen_transport: Box<dyn Transport> = if let Some(t) = transport {
            t
        } else {
            // Create subprocess transport
            let prompt_input = match prompt {
                PromptInput::String(s) => {
                    super::transport::PromptInput::String(s)
                }
                PromptInput::Stream(rx) => {
                    super::transport::PromptInput::Stream(rx)
                }
            };

            Box::new(SubprocessCLITransport::new(prompt_input, configured_options)?)
        };

        // Connect transport
        let mut transport_mut = chosen_transport;
        transport_mut.connect().await?;

        // Create Query to handle control protocol
        let mut query = Query::new(
            transport_mut,
            is_streaming,
            can_use_tool,
            hooks,
        );

        // Start reading messages
        query.start().await?;

        // Initialize if streaming
        if is_streaming {
            query.initialize().await?;
        }

        // Get message receiver
        let mut message_rx = query.receive_messages()
            .ok_or_else(|| crate::types::Error::Unknown("Failed to get message receiver".to_string()))?;

        // Create stream of parsed messages
        let message_stream = stream! {
            while let Some(data) = message_rx.recv().await {
                match parse_message(data) {
                    Ok(message) => yield Ok(message),
                    Err(e) => {
                        yield Err(e);
                        break;
                    }
                }
            }
        };

        Ok(message_stream)
    }
}

impl Default for InternalClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = InternalClient::new();
        // Just verify it compiles and creates
        let _ = client;
    }

    #[tokio::test]
    async fn test_validation_can_use_tool_with_string() {
        let client = InternalClient::new();
        let mut options = ClaudeAgentOptions::new();

        // This should fail because can_use_tool requires streaming mode
        // Note: We can't actually test this without a mock CanUseTool implementation
        // This is just a structural test
    }
}
