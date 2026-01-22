//! Process handle for managing subprocess lifecycle.

use crate::types::{Error, Result};
use tokio::process::Child;

/// Handle for managing a subprocess.
///
/// Provides methods to control the subprocess lifecycle (kill, wait, etc.)
/// without exposing the underlying Child object.
pub struct ProcessHandle {
    child: Child,
}

impl ProcessHandle {
    /// Create a new process handle from a Child process.
    pub fn new(child: Child) -> Self {
        Self { child }
    }

    /// Terminate the process.
    pub async fn kill(&mut self) -> Result<()> {
        self.child
            .kill()
            .await
            .map_err(|e| Error::Process(format!("Failed to kill process: {}", e)))
    }

    /// Wait for the process to exit and return its status.
    pub async fn wait(&mut self) -> Result<std::process::ExitStatus> {
        self.child
            .wait()
            .await
            .map_err(|e| Error::Process(format!("Failed to wait for process: {}", e)))
    }

    /// Check if the process has exited without blocking.
    pub fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>> {
        self.child
            .try_wait()
            .map_err(|e| Error::Process(format!("Failed to check process status: {}", e)))
    }

    /// Get the process ID.
    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }
}
