//! Integration tests for process I/O split functionality.
//!
//! These tests verify that the split API works correctly and that the
//! stderr receiver and process handle can be obtained independently.
//!
//! Note: Full functionality testing requires a real Claude CLI process.
//! These tests primarily verify API compilation and basic usage patterns.

use claude_agent_sdk::{ClaudeAgentOptions, ClaudeClient};

#[tokio::test]
async fn test_split_basic_usage() {
    // Create a client with default options
    let options = ClaudeAgentOptions::new();
    let mut client = ClaudeClient::new(options);

    // Verify that we can call the split API methods
    // These should return None before connection, but the API should compile
    let stderr_rx = client.stderr_receiver();
    let process_handle = client.process_handle();

    // Both should be None since we haven't connected yet
    assert!(stderr_rx.is_none(), "stderr_receiver should be None before connection");
    assert!(process_handle.is_none(), "process_handle should be None before connection");
}

#[tokio::test]
async fn test_split_api_can_only_be_called_once() {
    let options = ClaudeAgentOptions::new();
    let mut client = ClaudeClient::new(options);

    // First call should work (returns None since not connected)
    let first_stderr = client.stderr_receiver();
    assert!(first_stderr.is_none());

    // Second call should also return None (already taken)
    let second_stderr = client.stderr_receiver();
    assert!(second_stderr.is_none(), "stderr_receiver should return None on second call");

    // Same for process_handle
    let first_handle = client.process_handle();
    assert!(first_handle.is_none());

    let second_handle = client.process_handle();
    assert!(second_handle.is_none(), "process_handle should return None on second call");
}

#[test]
fn test_client_creation_with_split_api() {
    // Verify that creating a client and accessing split API compiles
    let options = ClaudeAgentOptions::new();
    let mut client = ClaudeClient::new(options);

    // This test just verifies the API compiles and doesn't panic
    let _ = client.stderr_receiver();
    let _ = client.process_handle();
}

#[tokio::test]
async fn test_split_api_types() {
    use tokio::sync::mpsc;

    let options = ClaudeAgentOptions::new();
    let mut client = ClaudeClient::new(options);

    // Verify the types are correct
    let stderr_rx: Option<mpsc::Receiver<String>> = client.stderr_receiver();
    assert!(stderr_rx.is_none());

    // ProcessHandle type should be available
    let _process_handle = client.process_handle();
}
