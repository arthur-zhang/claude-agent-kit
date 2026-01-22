//! Read half for subprocess stdout.

use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::mpsc;

/// Read half for subprocess stdout.
///
/// Provides methods to read and parse JSON messages from stdout.
pub struct ReadHalf<R: AsyncRead + Unpin + Send> {
    reader: BufReader<R>,
}

impl<R: AsyncRead + Unpin + Send + 'static> ReadHalf<R> {
    /// Create a new read half from an AsyncRead.
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
        }
    }

    /// Consume self and return a channel that yields parsed JSON messages.
    ///
    /// This method spawns a background task that continuously reads lines
    /// from stdout, parses them as JSON, and sends them through the channel.
    pub fn read_messages(self) -> mpsc::Receiver<serde_json::Value> {
        let (tx, rx) = mpsc::channel(100);
        let reader = self.reader;

        tokio::spawn(async move {
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                    if tx.send(json).await.is_err() {
                        break;
                    }
                }
            }
        });

        rx
    }
}
