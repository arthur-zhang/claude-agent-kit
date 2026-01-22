//! Stderr half for subprocess stderr.
//!
//! This module provides a wrapper for reading diagnostic logs from the subprocess stderr.
//! It spawns a background task that continuously reads lines from stderr.

use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::mpsc;

/// Stderr half for subprocess stderr.
///
/// Provides methods to read lines from stderr. This component is designed to be
/// moved to a separate task for concurrent reading of diagnostic logs while other
/// tasks handle stdout or stdin.
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
///     // Start reading stderr in a background task
///     let mut stderr_rx = stderr_half.read_lines();
///     tokio::spawn(async move {
///         while let Some(line) = stderr_rx.recv().await {
///             eprintln!("CLI stderr: {}", line);
///         }
///     });
///
///     Ok(())
/// }
/// ```
pub struct StderrHalf<R: AsyncRead + Unpin + Send> {
    reader: BufReader<R>,
}

impl<R: AsyncRead + Unpin + Send + 'static> StderrHalf<R> {
    /// Create a new stderr half from an AsyncRead.
    ///
    /// # Arguments
    ///
    /// * `reader` - An async reader (typically stderr from a subprocess)
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
        }
    }

    /// Consume self and return a channel that yields lines from stderr.
    ///
    /// This method spawns a background task that continuously reads lines
    /// from stderr and sends them through the channel. The task runs until
    /// EOF is reached or the receiver is dropped.
    ///
    /// # Returns
    ///
    /// A receiver that yields `String` lines from stderr. The channel has
    /// a buffer size of 100 lines.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use claude_agent_sdk::internal::transport::StderrHalf;
    /// # use tokio::process::ChildStderr;
    /// # async fn example(stderr: ChildStderr) {
    /// let stderr_half = StderrHalf::new(stderr);
    /// let mut stderr_rx = stderr_half.read_lines();
    ///
    /// // Log stderr lines as they arrive
    /// while let Some(line) = stderr_rx.recv().await {
    ///     eprintln!("CLI: {}", line);
    /// }
    /// # }
    /// ```
    pub fn read_lines(self) -> mpsc::Receiver<String> {
        let (tx, rx) = mpsc::channel(100);
        let reader = self.reader;

        tokio::spawn(async move {
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if tx.send(line).await.is_err() {
                    break;
                }
            }
        });

        rx
    }
}
