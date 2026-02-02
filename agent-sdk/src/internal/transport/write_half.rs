//! Write half for subprocess stdin.
//!
//! This module provides a wrapper for writing data to the subprocess stdin.
//! It handles buffering and automatic flushing to ensure messages are sent immediately.

use crate::types::{Error, Result};
use tokio::io::{AsyncWrite, AsyncWriteExt};

/// Write half for subprocess stdin.
///
/// Provides methods to write data to the subprocess stdin. This component
/// is designed to be moved to a separate task for concurrent writing while
/// other tasks handle reading or process management.
///
/// All writes are automatically flushed to ensure immediate delivery.
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
///     let (read_half, mut write_half, stderr_half, process_handle) = transport.split()?;
///
///     // Write a message
///     let message = r#"{"type":"user","message":{"role":"user","content":"Hello!"}}"#;
///     write_half.write(&format!("{}\n", message)).await?;
///
///     Ok(())
/// }
/// ```
pub struct WriteHalf<W: AsyncWrite + Unpin + Send> {
    writer: W,
}

impl<W: AsyncWrite + Unpin + Send> WriteHalf<W> {
    /// Create a new write half from an AsyncWrite.
    ///
    /// # Arguments
    ///
    /// * `writer` - An async writer (typically stdin from a subprocess)
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Write data to stdin.
    ///
    /// This method writes the data and automatically flushes to ensure
    /// immediate delivery. Each write operation is atomic and guaranteed
    /// to be sent before the method returns.
    ///
    /// # Arguments
    ///
    /// * `data` - The string data to write (typically a JSON message)
    ///
    /// # Errors
    ///
    /// Returns an error if the write or flush operation fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use claude_agent_sdk::internal::transport::WriteHalf;
    /// # use tokio::process::ChildStdin;
    /// # async fn example(stdin: ChildStdin) -> Result<(), Box<dyn std::error::Error>> {
    /// let mut write_half = WriteHalf::new(stdin);
    ///
    /// // Write a JSON message (must include newline)
    /// write_half.write("{\"type\":\"ping\"}\n").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn write(&mut self, data: &str) -> Result<()> {
        tracing::info!("📤 [STDIN] Writing to Claude Code:\n{}", data.trim_end());
        self.writer
            .write_all(data.as_bytes())
            .await
            .map_err(Error::Io)?;
        self.writer.flush().await.map_err(Error::Io)?;
        Ok(())
    }

    pub async fn write_with_newline(&mut self, data: &str) -> Result<()> {
        tracing::info!("📤 [STDIN] Writing to Claude Code:\n{}", data);
        self.writer
            .write_all(data.as_bytes())
            .await
            .map_err(Error::Io)?;
        self.writer.write_all(b"\n").await.map_err(Error::Io)?;
        self.writer.flush().await.map_err(Error::Io)?;
        Ok(())
    }
    pub async fn write_json<T: serde::Serialize>(&mut self, message: &T) -> Result<()> {
        let json = serde_json::to_string(message)?;
        tracing::info!("📤 [STDIN] Writing JSON to Claude Code:\n{}", json);
        self.writer
            .write_all(json.as_bytes())
            .await
            .map_err(Error::Io)?;
        self.writer.write_all(b"\n").await.map_err(Error::Io)?;
        self.writer.flush().await.map_err(Error::Io)?;
        Ok(())
    }
}
