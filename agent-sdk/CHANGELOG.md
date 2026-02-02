# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `ProcessHandle` for managing subprocess lifecycle
- `ReadHalf`, `WriteHalf`, `StderrHalf` for independent I/O operations
- `SubprocessCLITransport::split()` method for splitting transport into halves
- `ClaudeClient::stderr_receiver()` for accessing stderr stream
- `ClaudeClient::process_handle()` for accessing process control

### Changed
- **BREAKING**: Removed `Transport` trait - use `SubprocessCLITransport` directly
- **BREAKING**: Removed `custom_transport` parameter from `ClaudeClient`
- Refactored `Query` to use `WriteHalf` instead of trait object
- Simplified ownership model for stdin/stdout/stderr

### Removed
- **BREAKING**: `Transport` trait and `base.rs`
- **BREAKING**: `custom_transport` support

### Migration Guide

#### Removing Custom Transport Support

**Before:**
```rust
let transport = SubprocessCLITransport::new(options.clone())?;
let client = ClaudeClient::new(options, Some(transport))?;
```

**After:**
```rust
let client = ClaudeClient::new(options)?;
```

The `ClaudeClient` now creates and manages the `SubprocessCLITransport` internally. If you need custom transport behavior, please open an issue to discuss your use case.

#### Accessing Process Control

**New capability:**
```rust
let client = ClaudeClient::new(options)?;

// Access stderr stream
let stderr_receiver = client.stderr_receiver();
tokio::spawn(async move {
    while let Some(line) = stderr_receiver.recv().await {
        eprintln!("stderr: {}", line);
    }
});

// Access process handle for lifecycle management
let process_handle = client.process_handle();
// Can now wait for process, kill it, etc.
```

#### Architecture Changes

The refactor introduces a cleaner separation of concerns:
- **ProcessHandle**: Manages subprocess lifecycle (wait, kill, status)
- **ReadHalf**: Handles stdout reading independently
- **WriteHalf**: Handles stdin writing independently
- **StderrHalf**: Handles stderr reading independently

This allows for better concurrency and clearer ownership semantics.
