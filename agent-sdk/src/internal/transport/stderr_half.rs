//! Stderr half for subprocess stderr.

use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::mpsc;

/// Stderr half for subprocess stderr.
///
/// Provides methods to read lines from stderr.
pub struct StderrHalf<R: AsyncRead + Unpin + Send> {
    reader: BufReader<R>,
}

impl<R: AsyncRead + Unpin + Send + 'static> StderrHalf<R> {
    /// Create a new stderr half from an AsyncRead.
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
        }
    }

    /// Consume self and return a channel that yields lines from stderr.
    ///
    /// This method spawns a background task that continuously reads lines
    /// from stderr and sends them through the channel.
    pub fn read_lines(self) -> mpsc::Receiver<String> {
        let (tx, rx) = mpsc::channel(100);
        let mut reader = self.reader;

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
