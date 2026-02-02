# shadcn/ui 迁移设计文档

**日期**: 2026-01-27
**状态**: 已验证
**迁移范围**: 前端 React 组件库完全替换

## 项目概览

这是一个全栈 AI 应用框架，后端为 Rust WebSocket 服务器，前端为 React 应用。当前前端使用 Tailwind CSS 和自定义组件，需要迁移至 shadcn/ui 现代简约风格。

**主要目标**：
- 将所有自定义 UI 组件替换为 shadcn/ui 组件
- 实现亮色/暗色模式切换
- 保持现有功能和用户交互流程不变
- 提升代码可维护性和视觉一致性

## 设计决策

### 迁移策略
**选择**：模块化分批替换（方案3）

**理由**：
- 平衡风险与效率
- 每个模块独立测试和部署
- 保持代码整洁，不会长期混乱
- 及时发现和修复问题

### 设计风格
**选择**：现代简约风格

**特点**：
- 使用 shadcn/ui 的默认配色方案（zinc/slate）
- 类似 ChatGPT 或 Claude 官网的界面风格
- 强调信息清晰性和交互简洁性

### 主题系统
**选择**：默认主题 + 暗色模式切换

**特点**：
- 支持亮色/暗色模式自由切换
- 提供"跟随系统"选项
- 使用 localStorage 持久化用户选择
- 平滑过渡动画

### 对话框交互
**选择**：shadcn/ui Dialog 组件

**特点**：
- 标准模态对话框
- 居中显示，有遮罩层
- 符合现代 UI 规范

## 技术栈

### 新增依赖
```bash
# 核心工具
class-variance-authority clsx tailwind-merge

# 图标库
lucide-react

# Radix UI 原语（shadcn/ui 基于此构建）
@radix-ui/react-dialog
@radix-ui/react-slot
@radix-ui/react-switch
```

### 初始化 shadcn/ui 组件
- `button` - 按钮组件
- `input` - 输入框
- `card` - 卡片容器
- `dialog` - 对话框
- `badge` - 状态标签
- `scroll-area` - 滚动区域
- `separator` - 分隔线
- `switch` - 开关（主题切换）
- `label` - 表单标签
- `textarea` - 多行输入
- `select` - 下拉选择
- `radio-group` - 单选组
- `checkbox` - 复选框

## 架构设计

### 目录结构
```
frontend/src/
├── components/
│   ├── ui/                      # shadcn/ui 基础组件
│   │   ├── button.tsx
│   │   ├── input.tsx
│   │   ├── card.tsx
│   │   ├── dialog.tsx
│   │   ├── badge.tsx
│   │   ├── scroll-area.tsx
│   │   ├── separator.tsx
│   │   ├── switch.tsx
│   │   ├── label.tsx
│   │   ├── textarea.tsx
│   │   ├── select.tsx
│   │   ├── radio-group.tsx
│   │   └── checkbox.tsx
│   │
│   ├── chat/                    # 聊天相关组件
│   │   ├── chat-interface.tsx   # 主界面
│   │   ├── message-list.tsx     # 消息列表
│   │   ├── message-bubble.tsx   # 消息气泡
│   │   ├── message-input.tsx    # 输入区域
│   │   └── connection-status.tsx # 连接状态
│   │
│   ├── dialogs/                 # 对话框组件
│   │   ├── permission-dialog.tsx    # 权限请求
│   │   └── user-question-dialog.tsx # 用户问题
│   │
│   ├── layout/                  # 布局组件
│   │   ├── header.tsx           # 顶部栏
│   │   └── sidebar.tsx          # 侧边栏（可选）
│   │
│   └── theme-provider.tsx       # 主题上下文提供者
│
├── lib/
│   └── utils.ts                 # 工具函数（cn()）
│
└── hooks/
    ├── useWebSocket.ts          # 保持不变
    └── useTheme.ts              # 主题管理
```

### 组件设计

#### ChatInterface
- 整体布局和状态管理容器
- 使用 `Card` 作为主容器
- 顶部 `Header`：标题、连接状态 Badge、主题切换 Switch
- 中间 `ScrollArea`：消息列表
- 底部固定：消息输入区域
- 响应式：移动端全屏，桌面端居中卡片

#### MessageBubble
- **用户消息**：右对齐，primary 色调 Card
- **助手消息**：左对齐，secondary 色调 Card
- **系统消息**：居中，muted 色调 Badge
- **工具消息**：带图标 Card，显示工具名称和状态
- 支持 Markdown 渲染

#### MessageList
- 新增子组件，负责消息列表的渲染和滚动控制
- 使用 `ScrollArea` 包装
- 自动滚动到最新消息

#### MessageInput
- 新增子组件，负责输入和发送
- `Textarea` + `Button` 组合
- 支持 Enter 快捷键发送

#### ConnectionStatus
- 新增子组件，显示 WebSocket 连接状态
- 使用 `Badge` 显示状态：连接中、已连接、已断开

#### PermissionDialog
- 使用 `Dialog` 组件（模态对话框）
- 标题：工具名称
- 内容：工具参数（JSON 格式化展示）
- 权限级别 Badge（危险/安全）
- 底部按钮：
  - `Allow Once` - 允许一次
  - `Allow Always` - 总是允许
  - `Deny` - 拒绝

#### UserQuestionDialog
- 使用 `Dialog` 组件
- 支持多种问题类型：
  - 文本输入（`Input` 或 `Textarea`）
  - 单选（`RadioGroup`）
  - 多选（`Checkbox`）
- 问题描述和选项说明清晰显示
- 底部提交按钮

### 主题系统

#### ThemeProvider
- React Context 管理主题状态
- 支持三种模式：`light`、`dark`、`system`
- 使用 `localStorage` 持久化用户选择
- 通过 `document.documentElement.classList` 切换主题类名

#### CSS 变量
在 `src/index.css` 中定义 CSS 变量：

**亮色模式（:root）**：
```css
--background: 0 0% 100%;
--foreground: 222.2 84% 4.9%;
--card: 0 0% 100%;
--primary: 222.2 47.4% 11.2%;
--primary-foreground: 210 40% 98%;
--secondary: 210 40% 96%;
--muted: 210 10% 40%;
/* ... 更多 */
```

**暗色模式（.dark）**：
```css
--background: 222.2 84% 4.9%;
--foreground: 210 40% 98%;
--card: 222.2 84% 4.9%;
--primary: 210 40% 98%;
--primary-foreground: 222.2 47.4% 11.2%;
--secondary: 217 32.6% 17.5%;
--muted: 210 11% 60%;
/* ... 更多 */
```

#### 主题切换 UI
- Header 中添加 `Switch` 组件
- 图标：太阳（亮色）/ 月亮（暗色）使用 lucide-react
- 平滑过渡动画

## 实现计划

### 第一阶段：基础设施（安装和配置）
**任务**：
1. 安装 shadcn/ui 依赖和工具
2. 初始化 shadcn/ui 配置
3. 创建 `src/lib/utils.ts` - cn() 工具函数
4. 设置 Tailwind CSS 主题变量
5. 创建 `src/components/theme-provider.tsx` - 主题上下文
6. 创建 `src/hooks/useTheme.ts` - 主题 Hook
7. 修改 `src/index.css` - 添加 CSS 变量定义
8. 修改 `App.tsx` - 集成主题系统
9. 修改 `tailwind.config.js` - 添加主题配置

### 第二阶段：UI 基础组件
**任务**：
1. 使用 shadcn/ui CLI 生成 13 个基础组件
2. 验证组件在亮色/暗色模式下显示正常
3. 检查组件响应式表现

### 第三阶段：聊天核心组件
**任务**：
1. 创建 `src/components/layout/header.tsx` - 顶部栏（包含主题切换）
2. 重构 `src/components/chat/chat-interface.tsx` - 主界面
3. 创建 `src/components/chat/message-list.tsx` - 消息列表
4. 重构 `src/components/chat/message-bubble.tsx` - 消息气泡
5. 创建 `src/components/chat/message-input.tsx` - 输入区域
6. 创建 `src/components/chat/connection-status.tsx` - 连接状态
7. 删除旧的 `ChatInterface.tsx`、`MessageBubble.tsx`
8. 更新 `App.tsx` 导入新组件

### 第四阶段：对话框组件
**任务**：
1. 重构 `src/components/dialogs/permission-dialog.tsx` - 权限对话框
2. 重构 `src/components/dialogs/user-question-dialog.tsx` - 用户问题对话框
3. 测试权限请求流程
4. 测试用户问题响应流程
5. 删除旧的 `PermissionDialog.tsx`、`UserQuestionDialog.tsx`
6. 删除旧的 `SystemInfo.tsx`（功能整合到侧边栏或 Header）

### 第五阶段：清理和优化
**任务**：
1. 删除所有未使用的旧样式文件
2. 删除 `App.css`（旧动画）- 迁移到新组件
3. 更新类型定义（如需要）
4. 运行完整功能测试
5. 测试亮色/暗色模式切换
6. 验证响应式表现
7. 提交最终代码

## 实现细节

### 保持不变
- WebSocket 连接和事件处理逻辑（`useWebSocket.ts`）
- 消息类型判断和处理逻辑
- 权限对话框状态管理
- 用户问题对话框状态管理
- 后端 Rust 代码

### 需要调整
- 所有颜色值改为使用 CSS 变量
- 动画和过渡效果迁移到新组件
- 响应式断点可能需要微调
- 日志和错误显示的样式

### 关键考虑
1. **过渡动画**：保留消息进入/退出的动画效果
2. **滚动行为**：确保新消息自动滚动到底部
3. **主题持久化**：用户选择的主题在刷新后保持
4. **无障碍**：保持现有的 ARIA 属性
5. **性能**：主题切换应该平滑无闪烁

## 预期效果

### 功能维度
- ✅ 完全替换现有组件
- ✅ 保持所有交互功能
- ✅ 支持亮色/暗色模式
- ✅ 现代简约设计风格
- ✅ 更好的代码可维护性

### 用户体验维度
- ✅ 更专业的界面外观
- ✅ 更好的视觉一致性
- ✅ 更清晰的信息层级
- ✅ 流畅的主题切换
- ✅ 更好的响应式支持

## 风险和缓解

| 风险 | 缓解措施 |
|------|----------|
| 迁移过程中功能丧失 | 分阶段迁移，每阶段完成后测试 |
| 主题切换性能问题 | 使用 CSS 变量优化，避免 JavaScript 性能瓶颈 |
| 浏览器兼容性 | 验证 CSS 变量支持范围，必要时提供 fallback |
| 自定义样式丧失 | 使用 Tailwind 的 `@apply` 和 CSS 变量保持灵活性 |

## 成功标准

1. 所有旧组件已替换为 shadcn/ui 组件
2. 所有功能测试通过（聊天、权限、问题）
3. 亮色/暗色模式切换正常，无视觉问题
4. 代码通过 ESLint 检查
5. 响应式在手机、平板、桌面设备上正常显示
6. 没有 console 错误或警告
