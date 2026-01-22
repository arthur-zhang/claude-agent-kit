//! Write half for subprocess stdin.

use crate::types::{Error, Result};
use tokio::io::{AsyncWrite, AsyncWriteExt};

/// Write half for subprocess stdin.
///
/// Provides methods to write data to the subprocess stdin.
pub struct WriteHalf<W: AsyncWrite + Unpin + Send> {
    writer: W,
}

impl<W: AsyncWrite + Unpin + Send> WriteHalf<W> {
    /// Create a new write half from an AsyncWrite.
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Write data to stdin.
    ///
    /// This method automatically flushes after writing.
    pub async fn write(&mut self, data: &str) -> Result<()> {
        self.writer
            .write_all(data.as_bytes())
            .await
            .map_err(Error::Io)?;
        self.writer.flush().await.map_err(Error::Io)?;
        Ok(())
    }
}
