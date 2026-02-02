# AGENTS.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

This is a Rust-based Claude Agent SDK with a WebSocket server implementation. The project consists of two main workspace members:
- **agent-sdk**: Core Rust SDK for interacting with Claude Code CLI (port of Python SDK)
- **websocket**: WebSocket server with React/TypeScript frontend for browser-based Claude interactions

## Development Commands

### Building
```bash
# Build entire workspace
cargo build

# Build specific package
cargo build -p claude-agent-sdk
cargo build -p websocket

# Release build
cargo build --release
```

### Testing
```bash
# Run all tests in workspace
cargo test

# Run tests for specific package
cargo test -p claude-agent-sdk
cargo test -p websocket

# Run specific test
cargo test --test integration_test
cargo test --test split_integration_test

# Run with debug output
RUST_LOG=debug cargo test
```

### Running the WebSocket Server
```bash
cd websocket
cargo run

# With custom logging
RUST_LOG=debug cargo run

# Server will start at http://127.0.0.1:3000
```

### Frontend Development
```bash
cd websocket/frontend

# Install dependencies (using bun or npm)
bun install
# or
npm install

# Start dev server (http://localhost:5173)
bun dev
# or
npm run dev

# Build frontend
bun run build
# or
npm run build

# Lint
bun run lint
# or
npm run lint
```

## Architecture

### Three-Layer Architecture

**Layer 1: agent-sdk (Core SDK)**
- `types/`: Complete type system mirroring Python SDK
  - `agent.rs`: ClaudeAgentOptions, model configs
  - `permissions.rs`: Permission modes and handling
  - `messages.rs`: Message types (Assistant, User, System, etc.)
  - `control.rs`: SDK control protocol messages
  - `hooks.rs`: Hook system for pre/post tool execution
  - `mcp.rs`: MCP server configuration
  - `sandbox.rs`: Sandbox settings
- `internal/`: Core implementation
  - `transport/subprocess.rs`: Manages Claude CLI subprocess I/O (stdin/stdout/stderr)
  - `session.rs`: AgentSession actor for full-duplex communication
  - `message_parser.rs`: Parses SDK protocol messages
- `client.rs`: High-level ClaudeClient API

**Layer 2: websocket (Server)**
- `server.rs`: Axum router, WebSocket handler, session initialization
- `protocol/`: Message conversion between SDK and WebSocket
  - `events.rs`: Unified event system (AgentEvent, ClientMessage)
  - `sdk_converter.rs`: Converts SDK ProtocolMessages → AgentEvents
  - `types.rs`: WebSocket message types
- `session/`: Session state and permission management
  - `state.rs`: SessionState with pending permissions
  - `approval.rs`: ApprovalService for async permission requests
  - `permission.rs`: PermissionHandler wrapping CanUseTool callback
  - `event_handler.rs`: Main session event loop
- `connection.rs`: Connection manager (DashMap-based)

**Layer 3: websocket/frontend (React UI)**
- React + TypeScript + Vite
- Components: PermissionDialog, Chat UI
- WebSocket client for bidirectional communication

### Key Architectural Patterns

**Actor Pattern**: `AgentSession` runs as tokio actor receiving commands via mpsc channel, handling bidirectional SDK communication.

**Transport Split**: `SubprocessCLITransport` splits into `ReadHalf`, `WriteHalf`, `StderrHalf`, and `ProcessHandle` for independent I/O.

**Approval System**: Permission requests flow through:
1. SDK → PermissionHandler.can_use_tool()
2. ApprovalService creates oneshot channel, stores in pending map
3. Send permission_request to WebSocket client
4. Client responds → respond_to_approval() → oneshot sender notifies
5. PermissionHandler returns PermissionResult to SDK

**Permission Modes**:
- `Auto`: Automatically approve tools
- `Manual`: Prompt for each tool
- `Bypass`: Skip all permission checks
- Special: `ExitPlanMode` approval auto-switches to Bypass

### Protocol Flow

The system uses a two-protocol architecture:

1. **SDK Protocol** (stdin/stdout with Claude CLI):
   - JSON-based control protocol
   - Message types: `AgentMessage`, `ControlRequest`, `ControlResponse`
   - Handles: initialization, user messages, tool execution, interrupts

2. **WebSocket Protocol** (between server and frontend):
   - Event-driven architecture
   - Unified events: `SessionInit`, `TurnStarted`, `ToolStarted`, `AssistantMessage`, etc.
   - Permission flow: `ControlRequest` → `permission_request` → `permission_response` → `ControlResponse`

See `docs/websocket-protocol.md` and `docs/unified-protocol.md` for complete protocol specs.

## Important Implementation Notes

### Working with the Agent SDK

**Creating a Client**:
```rust
use claude_agent_sdk::{ClaudeAgentOptions, ClaudeClient};

let options = ClaudeAgentOptions::new()
    .with_model("claude-sonnet-4")
    .with_max_turns(10);
    
let mut client = ClaudeClient::new(options);
client.connect(None).await?;
```

**Permission Handling**: The SDK requires either:
- Setting a `can_use_tool` callback (requires streaming mode)
- Setting `permission_prompt_tool_name` to "stdio" for control protocol
- These two options are mutually exclusive

**Subprocess Management**: The client manages a Claude CLI subprocess. Access via:
- `client.stderr_receiver()`: Get stderr stream
- `client.process_handle()`: Control subprocess lifecycle

### WebSocket Session Initialization

Sessions require a `user_session_init` message before accepting user messages:
```json
{
  "type": "user_session_init",
  "cwd": "/path/to/workspace",
  "model": "claude-sonnet-4",
  "permission_mode": "plan",
  "max_turns": 10
}
```

The server waits for this message before creating the ClaudeClient.

### Testing Approval System

See `websocket/APPROVAL_TESTING.md` for detailed testing instructions. Key test scenarios:
1. Basic approval flow (allow/deny)
2. Risk level display (high/medium/low)
3. ExitPlanMode auto-bypass
4. Approval timeout (5 minutes)

## File Conventions

- **Rust**: Standard Cargo workspace structure
- **Frontend**: Standard Vite + React structure in `websocket/frontend/`
- **Tests**: Integration tests in `{package}/tests/`, unit tests co-located with code
- **Docs**: Detailed protocol specs and design docs in `docs/`

## Common Pitfalls

1. **Permission callback confusion**: Don't set both `can_use_tool` and `permission_prompt_tool_name`
2. **Session initialization**: Always send `user_session_init` before `user_message`
3. **Async approval**: ApprovalService uses oneshot channels; don't drop the receiver before responding
4. **Transport lifecycle**: After calling `.split()`, the original transport cannot be used
5. **Frontend build**: The frontend must be built before running the server for embedded assets to work

## Dependencies

### Rust Core
- tokio (async runtime)
- serde/serde_json (serialization)
- axum (web framework)
- futures (async streams)

### Frontend
- React 19
- Vite (build tool)
- Radix UI components
- TailwindCSS
