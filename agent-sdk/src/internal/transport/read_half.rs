//! Read half for subprocess stdout.
//!
//! This module provides a wrapper for reading JSON messages from the subprocess stdout.
//! It spawns a background task that continuously reads and parses JSON lines.

use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::mpsc;

use crate::types::ProtocolMessage;

/// Read half for subprocess stdout.
///
/// Provides methods to read and parse JSON messages from stdout. This component
/// is designed to be moved to a separate task for concurrent reading while other
/// tasks handle writing or process management.
///
/// # Example
///
/// ```rust,no_run
/// use claude_agent_sdk::internal::transport::{SubprocessCLITransport, PromptInput};
/// use claude_agent_sdk::ClaudeAgentOptions;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let options = ClaudeAgentOptions::new();
///     let prompt = PromptInput::String("Hello!".to_string());
///
///     let mut transport = SubprocessCLITransport::new(prompt, options)?;
///     transport.connect().await?;
///
///     let (read_half, write_half, stderr_half, process_handle) = transport.split()?;
///
///     // Start reading messages
///     let mut message_rx = read_half.read_messages();
///
///     // Process messages in a loop
///     while let Some(msg) = message_rx.recv().await {
///         println!("Received message: {:?}", msg);
///     }
///
///     Ok(())
/// }
/// ```
pub struct ReadHalf<R: AsyncRead + Unpin + Send> {
    reader: BufReader<R>,
}

impl<R: AsyncRead + Unpin + Send + 'static> ReadHalf<R> {
    /// Create a new read half from an AsyncRead.
    ///
    /// # Arguments
    ///
    /// * `reader` - An async reader (typically stdout from a subprocess)
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
        }
    }

    /// Consume self and return a channel that yields parsed protocol messages.
    ///
    /// This method spawns a background task that continuously reads lines
    /// from stdout, parses them as JSON, and sends them through the channel.
    /// The task runs until EOF is reached or the receiver is dropped.
    ///
    /// # Returns
    ///
    /// A receiver that yields `ProtocolMessage` messages. The channel has
    /// a buffer size of 100 messages.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use claude_agent_sdk::internal::transport::ReadHalf;
    /// # use tokio::process::ChildStdout;
    /// # async fn example(stdout: ChildStdout) {
    /// let read_half = ReadHalf::new(stdout);
    /// let mut message_rx = read_half.read_messages();
    ///
    /// // Process messages as they arrive
    /// while let Some(msg) = message_rx.recv().await {
    ///     println!("Message: {:?}", msg);
    /// }
    /// # }
    /// ```
    pub fn read_messages(self) -> mpsc::Receiver<ProtocolMessage> {
        let (tx, rx) = mpsc::channel(100);
        let reader = self.reader;

        tokio::spawn(async move {
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::info!("📥 [STDOUT] Received from Claude Code:\n{}", line);
                match serde_json::from_str::<ProtocolMessage>(&line) {
                    Ok(msg) => {
                        if tx.send(msg).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("⚠️  [STDOUT] Failed to parse as ProtocolMessage: {} - line: {}", e, line);
                    }
                }
            }
            tracing::info!("📥 [STDOUT] Stream ended");
        });

        rx
    }
}

