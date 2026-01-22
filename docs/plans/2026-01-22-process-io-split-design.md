# Process I/O Split 设计文档

**日期**: 2026-01-22
**作者**: Arthur Zhang
**状态**: 设计阶段

## 概述

本文档描述了如何重构 `SubprocessCLITransport`，模仿 TCP 的 read/write half 模式来分离进程的 stdin/stdout/stderr，解决并发访问和所有权问题。

## 问题陈述

### 当前问题

1. **所有权问题**: `Transport::read_messages()` 使用 `take()` 消费 stdout，导致只能调用一次
2. **并发访问问题**: stdin 使用 `Arc<Mutex<ChildStdin>>` 来支持并发写入，但这增加了复杂性和锁竞争
3. **职责不清**: Transport 同时管理进程生命周期、stdin、stdout、stderr，职责过于集中

### 目标

- 分离 stdin/stdout/stderr 为独立的 half，支持独立使用
- 使用泛型设计提高灵活性和可测试性
- 简化架构，移除不必要的抽象层（Transport trait）
- 提供清晰的进程生命周期管理

## 设计方案

### 核心架构

将 `SubprocessCLITransport` 分离为四个独立的结构：

```
SubprocessCLITransport::split()
    ↓
┌─────────────┬──────────────┬──────────────┬─────────────────┐
│  ReadHalf   │  WriteHalf   │ StderrHalf   │ ProcessHandle   │
│  (stdout)   │  (stdin)     │  (stderr)    │  (Child)        │
└─────────────┴──────────────┴──────────────┴─────────────────┘
```

### 1. 核心结构体

#### ReadHalf - 读取 stdout

```rust
pub struct ReadHalf<R: AsyncRead + Unpin + Send> {
    reader: BufReader<R>,
}

impl<R: AsyncRead + Unpin + Send + 'static> ReadHalf<R> {
    pub fn new(reader: R) -> Self;

    /// 消费 self，返回 JSON 消息流
    pub fn read_messages(self) -> mpsc::Receiver<serde_json::Value>;
}
```

**设计要点**:
- 使用泛型 `R: AsyncRead` 支持不同的输入源
- `read_messages()` 消费 self，明确表达所有权转移
- 内部启动异步任务持续读取并解析 JSON

#### WriteHalf - 写入 stdin

```rust
pub struct WriteHalf<W: AsyncWrite + Unpin + Send> {
    writer: W,
}

impl<W: AsyncWrite + Unpin + Send> WriteHalf<W> {
    pub fn new(writer: W) -> Self;

    /// 写入数据
    pub async fn write(&mut self, data: &str) -> Result<()>;
}
```

**设计要点**:
- 使用泛型 `W: AsyncWrite` 支持不同的输出目标
- 简单的写入接口，自动 flush
- 不需要 Arc<Mutex>，调用者负责同步

#### StderrHalf - 读取 stderr

```rust
pub struct StderrHalf<R: AsyncRead + Unpin + Send> {
    reader: BufReader<R>,
}

impl<R: AsyncRead + Unpin + Send + 'static> StderrHalf<R> {
    pub fn new(reader: R) -> Self;

    /// 消费 self，返回行流
    pub fn read_lines(self) -> mpsc::Receiver<String>;
}
```

**设计要点**:
- 与 ReadHalf 类似，但返回原始字符串而非 JSON
- 用于捕获错误日志和诊断信息

#### ProcessHandle - 进程管理

```rust
pub struct ProcessHandle {
    child: Child,
}

impl ProcessHandle {
    pub fn new(child: Child) -> Self;

    /// 终止进程
    pub async fn kill(&mut self) -> Result<()>;

    /// 等待进程结束
    pub async fn wait(&mut self) -> Result<std::process::ExitStatus>;

    /// 非阻塞检查进程状态
    pub async fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>>;

    /// 获取进程 ID
    pub fn id(&self) -> Option<u32>;
}
```

**设计要点**:
- 独立管理进程生命周期
- 提供完整的进程控制 API
- 不暴露内部 Child 对象，保持封装性

### 2. SubprocessCLITransport 的变更

#### 新的结构

```rust
pub struct SubprocessCLITransport {
    prompt: PromptInput,
    options: ClaudeAgentOptions,
    cli_path: PathBuf,
    process: Option<Child>,
    ready: bool,
}
```

**变更**:
- 移除 `stdin: Option<Arc<Mutex<ChildStdin>>>`
- 移除 `stdout: Option<ChildStdout>`
- 这些会在 `split()` 时转移到各个 half

#### split() 方法

```rust
impl SubprocessCLITransport {
    pub fn split(
        mut self,
    ) -> Result<(
        ReadHalf<ChildStdout>,
        WriteHalf<ChildStdin>,
        StderrHalf<ChildStderr>,
        ProcessHandle,
    )> {
        let mut child = self.process
            .take()
            .ok_or_else(|| Error::Process("Process not started".to_string()))?;

        let stdin = child.stdin.take().ok_or(...)?;
        let stdout = child.stdout.take().ok_or(...)?;
        let stderr = child.stderr.take().ok_or(...)?;

        Ok((
            ReadHalf::new(stdout),
            WriteHalf::new(stdin),
            StderrHalf::new(stderr),
            ProcessHandle::new(child),
        ))
    }
}
```

**设计要点**:
- 消费 self，确保 transport 不能在 split 后继续使用
- 返回四元组，调用者完全控制各个部分
- 必须在 `connect()` 之后调用

### 3. 移除 Transport trait

**理由**:
- 当前只有一个实现（SubprocessCLITransport）
- custom_transport 功能未被实际使用
- trait 增加了不必要的复杂性
- 简化设计，直接使用具体类型

**影响的文件**:
- 删除 `agent-sdk/src/internal/transport/base.rs`
- 更新 `agent-sdk/src/internal/transport/mod.rs`
- 更新 `agent-sdk/src/internal/query.rs`
- 更新 `agent-sdk/src/client.rs`

### 4. Query 的更新

**之前**:
```rust
pub struct Query {
    transport: Arc<Mutex<Box<dyn Transport>>>,
    // ...
}
```

**之后**:
```rust
pub struct Query {
    write_half: WriteHalf<ChildStdin>,
    // read_half 被消费，通过 channel 接收消息
    // ...
}

impl Query {
    pub fn new(
        write_half: WriteHalf<ChildStdin>,
        read_rx: mpsc::Receiver<serde_json::Value>,
        can_use_tool: Option<CanUseToolCallback>,
        hooks: Option<Hooks>,
    ) -> Self {
        // ...
    }
}
```

**变更**:
- 直接持有 `WriteHalf` 而非 trait object
- 接收已经启动的消息流 channel
- 简化所有权管理

### 5. ClaudeClient 的更新

**之前**:
```rust
pub struct ClaudeClient {
    options: ClaudeAgentOptions,
    custom_transport: Option<Box<dyn Transport>>,
    query: Option<Query>,
}
```

**之后**:
```rust
pub struct ClaudeClient {
    options: ClaudeAgentOptions,
    query: Option<Query>,
    stderr_rx: Option<mpsc::Receiver<String>>,
    process_handle: Option<ProcessHandle>,
}

impl ClaudeClient {
    pub fn new(options: ClaudeAgentOptions) -> Self {
        Self {
            options,
            query: None,
            stderr_rx: None,
            process_handle: None,
        }
    }

    pub async fn connect(&mut self, prompt: Option<ClientPromptInput>) -> Result<()> {
        // 创建并连接 transport
        let mut transport = SubprocessCLITransport::new(actual_prompt, self.options.clone())?;
        transport.connect().await?;

        // 分离
        let (read_half, write_half, stderr_half, process_handle) = transport.split()?;

        // 启动读取
        let read_rx = read_half.read_messages();
        let stderr_rx = stderr_half.read_lines();

        // 创建 Query
        let query = Query::new(write_half, read_rx, can_use_tool, hooks);
        query.start().await?;
        query.initialize().await?;

        self.query = Some(query);
        self.stderr_rx = Some(stderr_rx);
        self.process_handle = Some(process_handle);

        Ok(())
    }

    /// 获取 stderr 消息流（只能调用一次）
    pub fn stderr_receiver(&mut self) -> Option<mpsc::Receiver<String>> {
        self.stderr_rx.take()
    }

    /// 获取进程句柄（只能调用一次）
    pub fn process_handle(&mut self) -> Option<ProcessHandle> {
        self.process_handle.take()
    }
}
```

**新增功能**:
- 移除 `custom_transport` 参数
- 新增 `stderr_receiver()` 方法访问 stderr
- 新增 `process_handle()` 方法访问进程控制

## 使用示例

### 基本使用（通过 ClaudeClient）

```rust
let options = ClaudeAgentOptions::new();
let mut client = ClaudeClient::new(options);

// 连接
client.connect(None).await?;

// 可选：获取 stderr
if let Some(mut stderr_rx) = client.stderr_receiver() {
    tokio::spawn(async move {
        while let Some(line) = stderr_rx.recv().await {
            eprintln!("stderr: {}", line);
        }
    });
}

// 可选：获取进程句柄
if let Some(mut handle) = client.process_handle() {
    tokio::spawn(async move {
        let status = handle.wait().await;
        println!("Process exited: {:?}", status);
    });
}

// 正常使用 client
client.query_string("Hello", None).await?;
let mut response = client.receive_response().await?;
// ...
```

### 直接使用 split（高级用法）

```rust
// 创建并连接 transport
let mut transport = SubprocessCLITransport::new(prompt, options)?;
transport.connect().await?;

// 分离
let (read_half, mut write_half, stderr_half, mut handle) = transport.split()?;

// 在不同任务中使用
let read_task = tokio::spawn(async move {
    let mut rx = read_half.read_messages();
    while let Some(msg) = rx.recv().await {
        println!("Received: {:?}", msg);
    }
});

let stderr_task = tokio::spawn(async move {
    let mut rx = stderr_half.read_lines();
    while let Some(line) = rx.recv().await {
        eprintln!("stderr: {}", line);
    }
});

// 写入数据
write_half.write("{\"type\":\"user\",\"message\":\"hello\"}\n").await?;

// 等待进程
let status = handle.wait().await?;
```

## 实现计划

### 阶段 1: 创建新结构

1. 创建 `ReadHalf<R>` 结构和实现
2. 创建 `WriteHalf<W>` 结构和实现
3. 创建 `StderrHalf<R>` 结构和实现
4. 创建 `ProcessHandle` 结构和实现
5. 添加单元测试

### 阶段 2: 更新 SubprocessCLITransport

1. 移除 `stdin`、`stdout` 字段
2. 实现 `split()` 方法
3. 更新 `connect()` 方法
4. 更新测试

### 阶段 3: 移除 Transport trait

1. 删除 `base.rs`
2. 更新 `mod.rs` 导出
3. 更新所有引用 `Transport` trait 的代码

### 阶段 4: 更新 Query

1. 修改 `Query` 结构，使用 `WriteHalf`
2. 更新构造函数接受 `read_rx` channel
3. 移除对 `Transport` trait 的依赖
4. 更新所有相关方法

### 阶段 5: 更新 ClaudeClient

1. 移除 `custom_transport` 字段
2. 添加 `stderr_rx` 和 `process_handle` 字段
3. 更新 `connect()` 方法使用 `split()`
4. 添加 `stderr_receiver()` 和 `process_handle()` 方法
5. 更新文档和示例

### 阶段 6: 测试和文档

1. 更新所有单元测试
2. 添加集成测试
3. 更新 API 文档
4. 更新使用示例
5. 更新 CHANGELOG

## 破坏性变更

### 移除的功能

1. **Transport trait** - 不再存在，直接使用 `SubprocessCLITransport`
2. **custom_transport 参数** - `ClaudeClient::new()` 不再接受自定义 transport
3. **Transport::read_messages()** - 被 `ReadHalf::read_messages()` 替代

### API 变更

1. **ClaudeClient::new()** - 移除 `transport` 参数
   ```rust
   // 之前
   ClaudeClient::new(options, Some(custom_transport))

   // 之后
   ClaudeClient::new(options)
   ```

2. **新增方法**:
   - `ClaudeClient::stderr_receiver()` - 获取 stderr 流
   - `ClaudeClient::process_handle()` - 获取进程句柄

### 迁移指南

对于大多数用户，不需要修改代码，因为：
- `ClaudeClient` 的主要 API 保持不变
- `connect()`, `query_string()`, `receive_messages()` 等方法签名不变

对于使用 `custom_transport` 的用户：
- 需要直接使用 `SubprocessCLITransport`
- 或者提交 issue 说明使用场景，考虑重新引入扩展点

## 优势

1. **清晰的所有权**: 每个 half 独立拥有其 I/O 资源
2. **并发友好**: 可以在不同任务中独立使用读写
3. **灵活性**: 泛型设计支持测试和扩展
4. **简化**: 移除不必要的抽象层
5. **可控性**: 独立的 ProcessHandle 提供完整的进程控制
6. **可观测性**: 独立的 StderrHalf 便于日志收集

## 权衡

1. **破坏性变更**: 移除 Transport trait 和 custom_transport
   - **缓解**: 当前没有实际使用场景，未来可以重新引入

2. **API 复杂性**: split() 返回四元组
   - **缓解**: ClaudeClient 封装了复杂性，大多数用户不需要直接使用

3. **消费 self**: read_messages() 消费 ReadHalf
   - **缓解**: 这是明确的语义，避免了 Arc<Mutex> 的复杂性

## 未来扩展

1. **Reunite 支持**: 如果需要，可以添加 `reunite()` 方法重新组合
2. **更多 I/O 模式**: 可以支持其他类型的 transport（网络、IPC 等）
3. **流控制**: 可以添加背压和流控制机制
4. **监控**: 可以添加 metrics 和 tracing

## 参考

- [tokio::io::split](https://docs.rs/tokio/latest/tokio/io/fn.split.html)
- [tokio::net::TcpStream::into_split](https://docs.rs/tokio/latest/tokio/net/struct.TcpStream.html#method.into_split)
- Rust 所有权和借用模型
