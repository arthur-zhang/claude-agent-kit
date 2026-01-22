# Process I/O Split 实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**目标**: 重构 SubprocessCLITransport，将 stdin/stdout/stderr 分离为独立的 half，模仿 TCP 的 read/write half 模式

**架构**: 创建 ReadHalf、WriteHalf、StderrHalf、ProcessHandle 四个独立结构，移除 Transport trait，简化所有权管理

**技术栈**: Rust, tokio, async-trait, serde_json

---

## 阶段 1: 创建新的 Half 结构

### Task 1: 创建 ProcessHandle 结构

**文件**:
- 创建: `agent-sdk/src/internal/transport/process_handle.rs`

**Step 1: 创建 ProcessHandle 结构和基本实现**

```rust
//! Process handle for managing subprocess lifecycle.

use tokio::process::Child;
use crate::types::{Error, Result};

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
    pub async fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>> {
        self.child
            .try_wait()
            .map_err(|e| Error::Process(format!("Failed to check process status: {}", e)))
    }

    /// Get the process ID.
    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }
}
```

**Step 2: 添加到 mod.rs**

在 `agent-sdk/src/internal/transport/mod.rs` 中添加：

```rust
mod process_handle;
pub use process_handle::ProcessHandle;
```

**Step 3: 提交**

```bash
git add agent-sdk/src/internal/transport/process_handle.rs agent-sdk/src/internal/transport/mod.rs
git commit -m "feat: add ProcessHandle for subprocess lifecycle management"
```

---

### Task 2: 创建 WriteHalf 结构

**文件**:
- 创建: `agent-sdk/src/internal/transport/write_half.rs`

**Step 1: 创建 WriteHalf 结构和实现**

```rust
//! Write half for subprocess stdin.

use tokio::io::{AsyncWrite, AsyncWriteExt};
use crate::types::{Error, Result};

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
            .map_err(|e| Error::Io(e))?;
        self.writer.flush().await.map_err(|e| Error::Io(e))?;
        Ok(())
    }
}
```

**Step 2: 添加到 mod.rs**

在 `agent-sdk/src/internal/transport/mod.rs` 中添加：

```rust
mod write_half;
pub use write_half::WriteHalf;
```

**Step 3: 提交**

```bash
git add agent-sdk/src/internal/transport/write_half.rs agent-sdk/src/internal/transport/mod.rs
git commit -m "feat: add WriteHalf for subprocess stdin"
```

---

### Task 3: 创建 ReadHalf 结构

**文件**:
- 创建: `agent-sdk/src/internal/transport/read_half.rs`

**Step 1: 创建 ReadHalf 结构和实现**

```rust
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
        let mut reader = self.reader;

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
```

**Step 2: 添加到 mod.rs**

在 `agent-sdk/src/internal/transport/mod.rs` 中添加：

```rust
mod read_half;
pub use read_half::ReadHalf;
```

**Step 3: 提交**

```bash
git add agent-sdk/src/internal/transport/read_half.rs agent-sdk/src/internal/transport/mod.rs
git commit -m "feat: add ReadHalf for subprocess stdout"
```

---

### Task 4: 创建 StderrHalf 结构

**文件**:
- 创建: `agent-sdk/src/internal/transport/stderr_half.rs`

**Step 1: 创建 StderrHalf 结构和实现**

```rust
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
```

**Step 2: 添加到 mod.rs**

在 `agent-sdk/src/internal/transport/mod.rs` 中添加：

```rust
mod stderr_half;
pub use stderr_half::StderrHalf;
```

**Step 3: 提交**

```bash
git add agent-sdk/src/internal/transport/stderr_half.rs agent-sdk/src/internal/transport/mod.rs
git commit -m "feat: add StderrHalf for subprocess stderr"
```

---

## 阶段 2: 更新 SubprocessCLITransport

### Task 5: 添加 split() 方法到 SubprocessCLITransport

**文件**:
- 修改: `agent-sdk/src/internal/transport/subprocess.rs`

**Step 1: 添加必要的 imports**

在文件顶部添加：

```rust
use super::{ProcessHandle, ReadHalf, StderrHalf, WriteHalf};
use tokio::process::{ChildStderr, ChildStdin, ChildStdout};
```

**Step 2: 添加 split() 方法**

在 `impl SubprocessCLITransport` 块中添加（在 `build_command()` 方法之后）：

```rust
    /// Split the transport into independent read/write/stderr halves and process handle.
    ///
    /// This consumes the transport and returns four independent components:
    /// - ReadHalf: for reading stdout
    /// - WriteHalf: for writing to stdin
    /// - StderrHalf: for reading stderr
    /// - ProcessHandle: for managing the process lifecycle
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The process has not been started (call `connect()` first)
    /// - stdin, stdout, or stderr are not available
    pub fn split(
        mut self,
    ) -> Result<(
        ReadHalf<ChildStdout>,
        WriteHalf<ChildStdin>,
        StderrHalf<ChildStderr>,
        ProcessHandle,
    )> {
        // Ensure process is started
        let mut child = self
            .process
            .take()
            .ok_or_else(|| Error::Process("Process not started. Call connect() first.".to_string()))?;

        // Take stdin, stdout, stderr
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::Process("stdin not available".to_string()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Process("stdout not available".to_string()))?;

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| Error::Process("stderr not available".to_string()))?;

        // Create halves
        let read_half = ReadHalf::new(stdout);
        let write_half = WriteHalf::new(stdin);
        let stderr_half = StderrHalf::new(stderr);
        let process_handle = ProcessHandle::new(child);

        Ok((read_half, write_half, stderr_half, process_handle))
    }
```

**Step 3: 提交**

```bash
git add agent-sdk/src/internal/transport/subprocess.rs
git commit -m "feat: add split() method to SubprocessCLITransport"
```

---

## 阶段 3: 移除 Transport trait 并更新引用

### Task 6: 更新 Query 以使用 WriteHalf

**文件**:
- 修改: `agent-sdk/src/internal/query.rs`

**Step 1: 读取当前的 Query 实现**

先读取文件了解当前结构：

```bash
# 这一步由 agent 执行
```

**Step 2: 更新 Query 结构**

将 `transport: Arc<Mutex<Box<dyn Transport>>>` 替换为：

```rust
write_half: WriteHalf<tokio::process::ChildStdin>,
read_rx: Arc<Mutex<mpsc::Receiver<serde_json::Value>>>,
```

添加必要的 imports：

```rust
use crate::internal::transport::WriteHalf;
use tokio::process::ChildStdin;
```

**Step 3: 更新 Query::new() 构造函数**

修改签名为：

```rust
pub fn new(
    write_half: WriteHalf<ChildStdin>,
    read_rx: mpsc::Receiver<serde_json::Value>,
    can_use_tool: Option<CanUseToolCallback>,
    hooks: Option<Hooks>,
) -> Self
```

更新实现以使用新的字段。

**Step 4: 更新所有使用 transport 的方法**

将所有 `self.transport.lock().await.write(...)` 替换为 `self.write_half.write(...)`。

将所有 `self.transport.lock().await.read_messages()` 替换为使用 `self.read_rx`。

**Step 5: 提交**

```bash
git add agent-sdk/src/internal/query.rs
git commit -m "refactor: update Query to use WriteHalf instead of Transport trait"
```

---

### Task 7: 更新 ClaudeClient

**文件**:
- 修改: `agent-sdk/src/client.rs`

**Step 1: 读取当前的 ClaudeClient 实现**

先读取文件了解当前结构。

**Step 2: 更新 ClaudeClient 结构**

移除 `custom_transport: Option<Box<dyn Transport>>`。

添加：

```rust
stderr_rx: Option<mpsc::Receiver<String>>,
process_handle: Option<ProcessHandle>,
```

添加必要的 imports：

```rust
use crate::internal::transport::{ProcessHandle, SubprocessCLITransport};
```

**Step 3: 更新 ClaudeClient::new()**

移除 `transport` 参数，只保留 `options`：

```rust
pub fn new(options: ClaudeAgentOptions) -> Self {
    Self {
        options,
        query: None,
        stderr_rx: None,
        process_handle: None,
    }
}
```

**Step 4: 更新 connect() 方法**

修改 `connect()` 方法以使用 `split()`：

```rust
pub async fn connect(&mut self, prompt: Option<ClientPromptInput>) -> Result<()> {
    // ... 现有的配置验证代码 ...

    // 创建并连接 transport
    let mut transport = SubprocessCLITransport::new(actual_prompt, self.options.clone())?;
    transport.connect().await?;

    // 分离
    let (read_half, write_half, stderr_half, process_handle) = transport.split()?;

    // 启动读取
    let read_rx = read_half.read_messages();
    let stderr_rx = stderr_half.read_lines();

    // 创建 Query
    let query = Query::new(
        write_half,
        read_rx,
        self.options.can_use_tool.clone(),
        self.options.hooks.clone(),
    );
    query.start().await?;
    query.initialize().await?;

    self.query = Some(query);
    self.stderr_rx = Some(stderr_rx);
    self.process_handle = Some(process_handle);

    Ok(())
}
```

**Step 5: 添加新的访问方法**

```rust
/// Get the stderr receiver (can only be called once).
pub fn stderr_receiver(&mut self) -> Option<mpsc::Receiver<String>> {
    self.stderr_rx.take()
}

/// Get the process handle (can only be called once).
pub fn process_handle(&mut self) -> Option<ProcessHandle> {
    self.process_handle.take()
}
```

**Step 6: 提交**

```bash
git add agent-sdk/src/client.rs
git commit -m "refactor: update ClaudeClient to use split() and remove custom_transport"
```

---

### Task 8: 删除 Transport trait

**文件**:
- 删除: `agent-sdk/src/internal/transport/base.rs`
- 修改: `agent-sdk/src/internal/transport/mod.rs`
- 修改: `agent-sdk/src/internal/transport/subprocess.rs`

**Step 1: 从 subprocess.rs 移除 Transport trait 实现**

删除：
- `use super::Transport;`
- `#[async_trait]`
- `impl Transport for SubprocessCLITransport { ... }` 整个块

将 Transport trait 的方法移到 `impl SubprocessCLITransport` 块中（如果还需要的话）。

**Step 2: 删除 base.rs**

```bash
rm agent-sdk/src/internal/transport/base.rs
```

**Step 3: 更新 mod.rs**

移除：

```rust
mod base;
pub use base::Transport;
```

确保导出新的类型：

```rust
mod process_handle;
mod read_half;
mod stderr_half;
mod subprocess;
mod write_half;

pub use process_handle::ProcessHandle;
pub use read_half::ReadHalf;
pub use stderr_half::StderrHalf;
pub use subprocess::{PromptInput, SubprocessCLITransport};
pub use write_half::WriteHalf;
```

**Step 4: 提交**

```bash
git add -A
git commit -m "refactor: remove Transport trait and simplify architecture"
```

---

## 阶段 4: 更新 SubprocessCLITransport 结构

### Task 9: 移除 stdin/stdout 字段

**文件**:
- 修改: `agent-sdk/src/internal/transport/subprocess.rs`

**Step 1: 更新 SubprocessCLITransport 结构**

移除字段：

```rust
stdin: Option<Arc<Mutex<ChildStdin>>>,
stdout: Option<ChildStdout>,
```

**Step 2: 更新 new() 方法**

移除对这些字段的初始化：

```rust
Ok(Self {
    prompt,
    options,
    cli_path,
    process: None,
    ready: false,
})
```

**Step 3: 更新 connect() 方法**

移除对 stdin/stdout 的赋值：

```rust
// 删除这些行：
// self.stdin = child.stdin.take().map(|s| Arc::new(Mutex::new(s)));
// self.stdout = child.stdout.take();
```

只保留：

```rust
self.process = Some(child);
self.ready = true;
```

**Step 4: 移除不再需要的方法**

删除 `write()`, `read_messages()`, `end_input()` 方法（如果它们还在 impl 块中）。

保留 `connect()`, `close()`, `is_ready()` 方法。

**Step 5: 提交**

```bash
git add agent-sdk/src/internal/transport/subprocess.rs
git commit -m "refactor: remove stdin/stdout fields from SubprocessCLITransport"
```

---

## 阶段 5: 测试和验证

### Task 10: 更新现有测试

**文件**:
- 修改: `agent-sdk/src/internal/transport/subprocess.rs` (tests 模块)

**Step 1: 更新或移除过时的测试**

检查 `#[cfg(test)]` 模块中的测试，更新或移除不再适用的测试。

**Step 2: 运行测试**

```bash
cargo test --package agent-sdk
```

**Step 3: 修复任何失败的测试**

根据错误信息修复测试。

**Step 4: 提交**

```bash
git add agent-sdk/src/internal/transport/subprocess.rs
git commit -m "test: update tests for new split architecture"
```

---

### Task 11: 添加集成测试

**文件**:
- 创建: `agent-sdk/tests/split_integration_test.rs`

**Step 1: 创建集成测试**

```rust
//! Integration tests for process I/O split functionality.

use claude_agent_sdk::{ClaudeAgentOptions, ClaudeClient};

#[tokio::test]
async fn test_split_basic_usage() {
    let options = ClaudeAgentOptions::new();
    let mut client = ClaudeClient::new(options);

    // This test just verifies the API compiles and doesn't panic
    // Actual functionality testing requires a real Claude CLI
    let _ = client.stderr_receiver();
    let _ = client.process_handle();
}
```

**Step 2: 运行集成测试**

```bash
cargo test --package agent-sdk --test split_integration_test
```

**Step 3: 提交**

```bash
git add agent-sdk/tests/split_integration_test.rs
git commit -m "test: add integration tests for split functionality"
```

---

### Task 12: 运行完整测试套件

**Step 1: 运行所有测试**

```bash
cargo test --workspace
```

**Step 2: 修复任何失败的测试**

根据错误信息修复。

**Step 3: 运行 clippy**

```bash
cargo clippy --workspace -- -D warnings
```

**Step 4: 修复 clippy 警告**

根据 clippy 的建议修复代码。

**Step 5: 运行 fmt**

```bash
cargo fmt --all
```

**Step 6: 最终提交**

```bash
git add -A
git commit -m "chore: fix clippy warnings and format code"
```

---

## 阶段 6: 文档更新

### Task 13: 更新 API 文档

**文件**:
- 修改: `agent-sdk/src/internal/transport/subprocess.rs`
- 修改: `agent-sdk/src/client.rs`

**Step 1: 添加模块级文档**

在每个新文件顶部添加详细的文档注释。

**Step 2: 添加示例代码**

在 `SubprocessCLITransport::split()` 方法的文档中添加使用示例。

**Step 3: 生成文档**

```bash
cargo doc --package agent-sdk --no-deps --open
```

**Step 4: 检查文档**

确保所有公共 API 都有文档，示例代码可以编译。

**Step 5: 提交**

```bash
git add -A
git commit -m "docs: add comprehensive API documentation for split functionality"
```

---

### Task 14: 更新 CHANGELOG

**文件**:
- 修改: `CHANGELOG.md`

**Step 1: 添加新版本条目**

```markdown
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
- Replace `ClaudeClient::new(options, Some(transport))` with `ClaudeClient::new(options)`
- If you need custom transport behavior, please open an issue to discuss your use case
```

**Step 2: 提交**

```bash
git add CHANGELOG.md
git commit -m "docs: update CHANGELOG for process I/O split refactor"
```

---

## 完成检查清单

- [ ] 所有新结构已创建 (ProcessHandle, ReadHalf, WriteHalf, StderrHalf)
- [ ] SubprocessCLITransport 已添加 split() 方法
- [ ] Transport trait 已移除
- [ ] Query 已更新为使用 WriteHalf
- [ ] ClaudeClient 已更新为使用 split()
- [ ] SubprocessCLITransport 的 stdin/stdout 字段已移除
- [ ] 所有测试通过
- [ ] Clippy 无警告
- [ ] 代码已格式化
- [ ] API 文档已更新
- [ ] CHANGELOG 已更新

---

## 注意事项

1. **测试策略**: 由于需要真实的 Claude CLI 才能完整测试，集成测试主要验证 API 编译和基本调用
2. **错误处理**: 所有新方法都使用 `Result<T>` 返回类型，确保错误能够正确传播
3. **所有权**: `split()` 消费 self，确保 transport 不能在分离后继续使用
4. **并发**: ReadHalf 和 StderrHalf 的 `read_messages()`/`read_lines()` 方法启动后台任务，不会阻塞调用者
5. **向后兼容**: 这是一个破坏性变更，需要在 CHANGELOG 中明确标注

---

## 参考

- 设计文档: `docs/plans/2026-01-22-process-io-split-design.md`
- tokio::io::split: https://docs.rs/tokio/latest/tokio/io/fn.split.html
- tokio::net::TcpStream::into_split: https://docs.rs/tokio/latest/tokio/net/struct.TcpStream.html#method.into_split
