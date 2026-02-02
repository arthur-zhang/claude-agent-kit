# Claude Agent Chat - WebSocket Frontend

基于 React + TypeScript + Tailwind CSS + Bun 构建的 Claude Agent SDK WebSocket 聊天界面。

## 功能特性

- 🚀 **实时通信**: 基于 WebSocket 协议的实时双向通信
- 💬 **聊天界面**: 美观的聊天气泡界面，支持用户和 Agent 消息
- 🔄 **流式响应**: 支持 Agent 的流式文本输出，实时显示打字效果
- 🔐 **权限管理**: 可视化的工具权限请求对话框
- 🛠️ **工具调用**: 显示工具使用和执行结果
- ⚡ **快速开发**: 使用 Bun 作为包管理器和运行时
- 🎨 **现代 UI**: 使用 Tailwind CSS 构建响应式界面

## 技术栈

- **React 19** - UI 框架
- **TypeScript** - 类型安全
- **Tailwind CSS** - 样式框架
- **Vite** - 构建工具
- **Bun** - 包管理器和运行时

## 快速开始

### 前置要求

- Bun >= 1.0.0
- Node.js >= 18.0.0 (可选)

### 安装依赖

```bash
cd websocket/frontend
bun install
```

### 开发模式

```bash
bun dev
```

应用将在 http://localhost:5173 启动。

### 构建生产版本

```bash
bun run build
```

### 预览生产构建

```bash
bun run preview
```

## 项目结构

```
frontend/
├── src/
│   ├── components/          # React 组件
│   │   ├── ChatInterface.tsx    # 主聊天界面
│   │   ├── MessageBubble.tsx    # 消息气泡组件
│   │   └── PermissionDialog.tsx # 权限请求对话框
│   ├── hooks/               # 自定义 Hooks
│   │   └── useWebSocket.ts      # WebSocket 连接管理
│   ├── types.ts             # TypeScript 类型定义
│   ├── App.tsx              # 应用入口
│   ├── App.css              # 应用样式
│   ├── index.css            # 全局样式（Tailwind）
│   └── main.tsx             # React 入口
├── index.html               # HTML 模板
├── tailwind.config.js       # Tailwind 配置
├── postcss.config.js        # PostCSS 配置
├── tsconfig.json            # TypeScript 配置
├── vite.config.ts           # Vite 配置
└── package.json             # 项目配置
```

## WebSocket 协议

前端实现了完整的 WebSocket 协议规范，详见 `/docs/websocket-protocol.md`。

### 支持的消息类型

**客户端 → 服务器**:
- `user_message` - 发送用户消息
- `permission_response` - 响应权限请求
- `interrupt` - 中断当前操作

**服务器 → 客户端**:
- `assistant_message_start` - Agent 消息开始
- `assistant_message_delta` - 流式内容更新
- `assistant_message_complete` - Agent 消息完成
- `tool_use` - 工具调用通知
- `tool_result` - 工具执行结果
- `permission_request` - 权限请求
- `result` - 操作结果
- `error` - 错误消息
- `warning` - 警告消息
- `session_info` - 会话信息

## 使用说明

### 连接到服务器

1. 在顶部输入框中输入 Session ID（默认为 "default"）
2. 点击 "Connect" 按钮连接到 WebSocket 服务器
3. 连接成功后，状态指示灯变为绿色

### 发送消息

1. 在底部输入框中输入消息
2. 按 Enter 发送（Shift+Enter 换行）
3. 或点击 "📤 Send" 按钮

### 处理权限请求

当 Agent 需要执行工具时，会弹出权限对话框：
- 查看工具名称、描述、风险级别和输入参数
- 点击 "✅ Allow" 允许执行
- 点击 "❌ Deny" 拒绝执行

### 中断操作

点击 "⏸️ Interrupt" 按钮可以中断 Agent 当前正在执行的操作。

### 清空消息

点击 "🗑️ Clear" 按钮清空聊天历史。

## 配置

### WebSocket 服务器地址

在 `src/components/ChatInterface.tsx` 中修改：

```typescript
const WS_URL = 'ws://localhost:3000/ws';
```

### Tailwind 配置

在 `tailwind.config.js` 中自定义主题、颜色等。

## 开发指南

### 添加新的消息类型

1. 在 `src/types.ts` 中定义类型
2. 在 `src/hooks/useWebSocket.ts` 中处理消息
3. 在 `src/components/MessageBubble.tsx` 中渲染

### 自定义样式

使用 Tailwind CSS 的 utility classes，或在 `src/App.css` 中添加自定义样式。

## 故障排除

### WebSocket 连接失败

- 确保后端 WebSocket 服务器正在运行（默认端口 3000）
- 检查浏览器控制台的错误信息
- 确认 WebSocket URL 配置正确

### 样式不生效

- 确保 Tailwind CSS 已正确配置
- 运行 `bun install` 重新安装依赖
- 清除浏览器缓存

### TypeScript 错误

- 运行 `bun run build` 检查类型错误
- 确保所有依赖都已安装

## 许可证

MIT

## 相关链接

- [WebSocket 协议文档](../../docs/websocket-protocol.md)
- [Claude Agent SDK](https://github.com/anthropics/anthropic-sdk-rust)
- [React 文档](https://react.dev)
- [Tailwind CSS 文档](https://tailwindcss.com)
- [Bun 文档](https://bun.sh)
