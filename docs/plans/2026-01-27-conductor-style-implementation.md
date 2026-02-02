# Conductor.build 风格迁移实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将现有 shadcn/ui 界面改造为 Conductor.build 风格，应用毛玻璃效果、轻量化边框和极简主义设计

**Architecture:** 分 5 个阶段逐步修改：CSS 基础设施 → 基础 UI 组件 → 布局组件 → 对话框组件 → 整体调整。每个组件修改后立即测试视觉效果。

**Tech Stack:** React 19, TypeScript 5.9, Tailwind CSS, shadcn/ui, Vite 7

---

## Phase 1: CSS 基础设施

### Task 1: 更新 CSS 变量配色方案

**Files:**
- Modify: `websocket/frontend/src/index.css`

**Step 1: 备份当前 CSS 变量**

Run: `cp websocket/frontend/src/index.css websocket/frontend/src/index.css.backup`

**Step 2: 更新深色主题 CSS 变量**

修改 `.dark` 部分的 CSS 变量：

```css
.dark {
  --background: 10 10 11;  /* #0a0a0b - 更深的背景 */
  --foreground: 250 250 250;  /* 更亮的前景色 */
  --card: 24 24 27;  /* #18181b - 稍亮的卡片背景 */
  --card-foreground: 250 250 250;
  --popover: 24 24 27;
  --popover-foreground: 250 250 250;
  --muted: 39 39 42;  /* #27272a */
  --muted-foreground: 161 161 170;  /* #a1a1aa */
  --accent: 39 39 42;
  --accent-foreground: 250 250 250;
  --destructive: 0 62.8% 30.6%;
  --destructive-foreground: 250 250 250;
  --border: 39 39 42;  /* #27272a - 更细腻的边框色 */
  --input: 39 39 42;
  --primary: 250 250 250;
  --primary-foreground: 10 10 11;
  --secondary: 39 39 42;
  --secondary-foreground: 250 250 250;
  --ring: 212.7 26.8% 83.9%;
}
```

**Step 3: 更新浅色主题 CSS 变量**

修改 `:root` 部分的 CSS 变量：

```css
:root {
  --background: 255 255 255;  /* 纯白背景 */
  --foreground: 10 10 11;  /* 深色文字 */
  --card: 250 250 250;  /* #fafafa - 浅灰卡片 */
  --card-foreground: 10 10 11;
  --popover: 255 255 255;
  --popover-foreground: 10 10 11;
  --muted: 245 245 245;  /* #f5f5f5 */
  --muted-foreground: 113 113 122;  /* #71717a */
  --accent: 245 245 245;
  --accent-foreground: 10 10 11;
  --destructive: 0 84.2% 60.2%;
  --destructive-foreground: 255 255 255;
  --border: 229 229 229;  /* #e5e5e5 - 浅色边框 */
  --input: 229 229 229;
  --primary: 10 10 11;
  --primary-foreground: 250 250 250;
  --secondary: 245 245 245;
  --secondary-foreground: 10 10 11;
  --ring: 10 10 11;
}
```

**Step 4: 移除不必要的动画定义**

删除或注释掉 `@keyframes fade-in` 和相关的 `.animate-fade-in` 类（如果存在）。

**Step 5: 验证 CSS 编译**

Run: `cd websocket/frontend && npm run dev`
Expected: Vite 启动成功，无 CSS 错误

**Step 6: 提交更改**

```bash
cd /Users/arthur/RustroverProjects/claude-agent-kit
git add websocket/frontend/src/index.css
git commit -m "style: update CSS variables for Conductor.build theme

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Phase 2: 基础 UI 组件

### Task 2: 更新 Button 组件

**Files:**
- Modify: `websocket/frontend/src/components/ui/button.tsx`

**Step 1: 更新 buttonVariants 基础类**

修改第 8 行的基础类，添加毛玻璃效果和移除位移动画：

```typescript
const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-lg text-sm font-medium ring-offset-background transition-colors duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:size-4 [&_svg]:shrink-0 backdrop-blur-md",
  // ...
)
```

**Step 2: 更新 default variant**

```typescript
default: "bg-primary/80 text-primary-foreground border border-primary/30 hover:bg-primary/90",
```

**Step 3: 更新 outline variant**

```typescript
outline: "border border-border/30 bg-background/60 backdrop-blur-md hover:bg-accent/40 hover:text-accent-foreground",
```

**Step 4: 更新 ghost variant**

```typescript
ghost: "hover:bg-accent/40 hover:text-accent-foreground backdrop-blur-sm",
```

**Step 5: 更新 secondary variant**

```typescript
secondary: "bg-secondary/60 text-secondary-foreground border border-border/30 backdrop-blur-sm hover:bg-secondary/80",
```

**Step 6: 验证编译**

Run: `cd websocket/frontend && npm run build`
Expected: 构建成功，无 TypeScript 错误

**Step 7: 提交更改**

```bash
git add websocket/frontend/src/components/ui/button.tsx
git commit -m "style: add glassmorphism effect to Button component

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

### Task 3: 更新 Card 组件

**Files:**
- Modify: `websocket/frontend/src/components/ui/card.tsx`

**Step 1: 更新 Card 基础样式**

修改第 11-13 行：

```typescript
className={cn(
  "rounded-lg border border-border/30 bg-card/60 text-card-foreground backdrop-blur-md",
  className
)}
```

**Step 2: 移除 shadow-sm**

确保移除了 `shadow-sm` 类。

**Step 3: 验证编译**

Run: `cd websocket/frontend && npm run build`
Expected: 构建成功

**Step 4: 提交更改**

```bash
git add websocket/frontend/src/components/ui/card.tsx
git commit -m "style: add glassmorphism effect to Card component

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

### Task 4: 更新 Badge 组件

**Files:**
- Modify: `websocket/frontend/src/components/ui/badge.tsx`

**Step 1: 读取当前 Badge 组件**

Run: `cat websocket/frontend/src/components/ui/badge.tsx`

**Step 2: 更新 badgeVariants 基础类**

添加 `backdrop-blur-sm` 到基础类。

**Step 3: 更新 default variant**

```typescript
default: "border-transparent bg-primary/80 text-primary-foreground backdrop-blur-sm hover:bg-primary/90",
```

**Step 4: 更新 secondary variant**

```typescript
secondary: "border-transparent bg-secondary/60 text-secondary-foreground backdrop-blur-sm hover:bg-secondary/80",
```

**Step 5: 更新 outline variant**

```typescript
outline: "text-foreground border-border/30 bg-background/40 backdrop-blur-sm",
```

**Step 6: 验证编译**

Run: `cd websocket/frontend && npm run build`
Expected: 构建成功

**Step 7: 提交更改**

```bash
git add websocket/frontend/src/components/ui/badge.tsx
git commit -m "style: add glassmorphism effect to Badge component

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

### Task 5: 更新 Input 和 Textarea 组件

**Files:**
- Modify: `websocket/frontend/src/components/ui/input.tsx`
- Modify: `websocket/frontend/src/components/ui/textarea.tsx`

**Step 1: 更新 Input 组件样式**

修改 className：

```typescript
className={cn(
  "flex h-10 w-full rounded-lg border border-border/30 bg-background/60 backdrop-blur-sm px-3 py-2 text-base ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium file:text-foreground placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 md:text-sm transition-colors duration-200",
  className
)
```

**Step 2: 更新 Textarea 组件样式**

修改 className：

```typescript
className={cn(
  "flex min-h-[80px] w-full rounded-lg border border-border/30 bg-background/60 backdrop-blur-sm px-3 py-2 text-base ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 md:text-sm transition-colors duration-200",
  className
)
```

**Step 3: 验证编译**

Run: `cd websocket/frontend && npm run build`
Expected: 构建成功

**Step 4: 提交更改**

```bash
git add websocket/frontend/src/components/ui/input.tsx websocket/frontend/src/components/ui/textarea.tsx
git commit -m "style: add glassmorphism effect to Input and Textarea

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Phase 3: 布局组件

### Task 6: 更新 Header 组件

**Files:**
- Modify: `websocket/frontend/src/components/layout/header.tsx`

**Step 1: 读取当前 Header 组件**

Run: `cat websocket/frontend/src/components/layout/header.tsx`

**Step 2: 更新 Header 容器样式**

修改主容器的 className，添加毛玻璃效果和细边框：

```typescript
<header className="sticky top-0 z-50 w-full border-b border-border/30 bg-background/80 backdrop-blur-xl">
  <div className="container flex h-16 items-center justify-between px-6">
    {/* ... */}
  </div>
</header>
```

**Step 3: 更新连接状态 Badge**

如果 Badge 使用了自定义样式，确保它使用 `variant="secondary"` 以获得毛玻璃效果。

**Step 4: 验证编译**

Run: `cd websocket/frontend && npm run build`
Expected: 构建成功

**Step 5: 提交更改**

```bash
git add websocket/frontend/src/components/layout/header.tsx
git commit -m "style: add glassmorphism effect to Header

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

### Task 7: 更新 MessageInput 组件

**Files:**
- Modify: `websocket/frontend/src/components/chat/message-input.tsx`

**Step 1: 更新容器样式**

修改第 35 行的容器 className：

```typescript
<div className="border-t border-border/30 bg-background/80 backdrop-blur-lg p-6">
```

**Step 2: 更新 Textarea 最小高度**

修改 Textarea 的 className，调整最小高度：

```typescript
className="min-h-[80px] max-h-[200px] resize-none"
```

**Step 3: 更新发送按钮尺寸**

修改按钮的 className：

```typescript
className="h-[80px] w-[80px] shrink-0"
```

**Step 4: 验证编译**

Run: `cd websocket/frontend && npm run build`
Expected: 构建成功

**Step 5: 提交更改**

```bash
git add websocket/frontend/src/components/chat/message-input.tsx
git commit -m "style: add glassmorphism effect to MessageInput

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

### Task 8: 更新 MessageBubble 组件

**Files:**
- Modify: `websocket/frontend/src/components/chat/message-bubble.tsx`

**Step 1: 更新系统消息 Badge 样式**

修改第 16-20 行：

```typescript
<div className="flex justify-center mb-6">
  <Badge variant="secondary" className="px-4 py-2 backdrop-blur-sm">
    <span className="text-xs">⚙️ {message.content}</span>
  </Badge>
</div>
```

**Step 2: 更新消息气泡容器间距**

修改第 26 行：

```typescript
className={`flex ${isUser ? 'justify-end' : 'justify-start'} mb-6`}
```

**Step 3: 更新消息气泡最大宽度和样式**

修改第 28-33 行：

```typescript
<Card
  className={`max-w-[75%] border ${
    isUser
      ? 'bg-primary/80 text-primary-foreground border-primary/30 backdrop-blur-md'
      : 'bg-card/60 border-border/30 backdrop-blur-md'
  }`}
>
```

**Step 4: 更新工具使用卡片样式**

修改第 56 行：

```typescript
<Card className="mt-3 p-3 bg-muted/40 backdrop-blur-md border-border/30">
```

**Step 5: 更新工具结果卡片样式**

修改第 70-76 行：

```typescript
<Card
  className={`mt-3 p-3 backdrop-blur-md ${
    message.toolResult.is_error
      ? 'bg-destructive/10 border-destructive/30'
      : 'bg-green-50/40 dark:bg-green-950/40 border-green-200/30 dark:border-green-800/30'
  }`}
>
```

**Step 6: 移除 animate-fade-in 类**

确保移除所有 `animate-fade-in` 类。

**Step 7: 验证编译**

Run: `cd websocket/frontend && npm run build`
Expected: 构建成功

**Step 8: 提交更改**

```bash
git add websocket/frontend/src/components/chat/message-bubble.tsx
git commit -m "style: add glassmorphism effect to MessageBubble

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

### Task 9: 更新 MessageList 组件

**Files:**
- Modify: `websocket/frontend/src/components/chat/message-list.tsx`

**Step 1: 读取当前 MessageList 组件**

Run: `cat websocket/frontend/src/components/chat/message-list.tsx`

**Step 2: 更新容器内边距**

如果有容器的 padding，从 `p-4` 改为 `p-6`。

**Step 3: 验证编译**

Run: `cd websocket/frontend && npm run build`
Expected: 构建成功

**Step 4: 提交更改**

```bash
git add websocket/frontend/src/components/chat/message-list.tsx
git commit -m "style: adjust spacing in MessageList

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Phase 4: 对话框组件

### Task 10: 更新 Dialog 基础组件

**Files:**
- Modify: `websocket/frontend/src/components/ui/dialog.tsx`

**Step 1: 读取当前 Dialog 组件**

Run: `cat websocket/frontend/src/components/ui/dialog.tsx`

**Step 2: 更新 DialogOverlay 样式**

添加 backdrop-blur 到遮罩层：

```typescript
<DialogPrimitive.Overlay
  className={cn(
    "fixed inset-0 z-50 bg-black/50 backdrop-blur-sm data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0",
    className
  )}
/>
```

**Step 3: 更新 DialogContent 样式**

修改对话框内容的样式：

```typescript
<DialogPrimitive.Content
  className={cn(
    "fixed left-[50%] top-[50%] z-50 grid w-full max-w-lg translate-x-[-50%] translate-y-[-50%] gap-4 border border-border/30 bg-background/90 backdrop-blur-xl p-6 shadow-lg duration-200 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[state=closed]:slide-out-to-left-1/2 data-[state=closed]:slide-out-to-top-[48%] data-[state=open]:slide-in-from-left-1/2 data-[state=open]:slide-in-from-top-[48%] sm:rounded-lg",
    className
  )}
/>
```

**Step 4: 验证编译**

Run: `cd websocket/frontend && npm run build`
Expected: 构建成功

**Step 5: 提交更改**

```bash
git add websocket/frontend/src/components/ui/dialog.tsx
git commit -m "style: add glassmorphism effect to Dialog

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

### Task 11: 更新 PermissionDialog 组件

**Files:**
- Modify: `websocket/frontend/src/components/dialogs/permission-dialog.tsx`

**Step 1: 读取当前 PermissionDialog 组件**

Run: `cat websocket/frontend/src/components/dialogs/permission-dialog.tsx`

**Step 2: 更新内部 Card 组件样式**

找到所有嵌套的 Card 组件，确保它们使用毛玻璃效果：

```typescript
<Card className="p-4 bg-muted/40 backdrop-blur-md border-border/30">
```

**Step 3: 更新 Badge 组件**

确保 Badge 使用适当的 variant 以获得毛玻璃效果。

**Step 4: 验证编译**

Run: `cd websocket/frontend && npm run build`
Expected: 构建成功

**Step 5: 提交更改**

```bash
git add websocket/frontend/src/components/dialogs/permission-dialog.tsx
git commit -m "style: enhance PermissionDialog with glassmorphism

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

### Task 12: 更新 UserQuestionDialog 组件

**Files:**
- Modify: `websocket/frontend/src/components/dialogs/user-question-dialog.tsx`

**Step 1: 读取当前 UserQuestionDialog 组件**

Run: `cat websocket/frontend/src/components/dialogs/user-question-dialog.tsx`

**Step 2: 更新内部 Card 组件样式**

找到所有嵌套的 Card 组件，添加毛玻璃效果：

```typescript
<Card className="p-4 bg-muted/40 backdrop-blur-md border-border/30">
```

**Step 3: 更新进度指示器样式**

如果有进度指示器，确保使用轻量化设计。

**Step 4: 验证编译**

Run: `cd websocket/frontend && npm run build`
Expected: 构建成功

**Step 5: 提交更改**

```bash
git add websocket/frontend/src/components/dialogs/user-question-dialog.tsx
git commit -m "style: enhance UserQuestionDialog with glassmorphism

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## Phase 5: 整体调整

### Task 13: 更新 ChatInterface 布局间距

**Files:**
- Modify: `websocket/frontend/src/components/chat/chat-interface.tsx`

**Step 1: 读取当前 ChatInterface 组件**

Run: `cat websocket/frontend/src/components/chat/chat-interface.tsx`

**Step 2: 更新设置面板间距**

修改设置面板的 padding 从 `p-4` 到 `p-6`：

```typescript
<Card className="m-4 p-6">
```

**Step 3: 更新系统信息面板间距**

修改系统信息面板的 padding：

```typescript
<Card className="m-4 p-6">
```

**Step 4: 更新控制按钮区域间距**

修改控制按钮区域的 padding：

```typescript
<div className="px-6 pb-4 flex gap-2">
```

**Step 5: 验证编译**

Run: `cd websocket/frontend && npm run build`
Expected: 构建成功

**Step 6: 提交更改**

```bash
git add websocket/frontend/src/components/chat/chat-interface.tsx
git commit -m "style: adjust spacing in ChatInterface

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

### Task 14: 移除主题切换图标动画

**Files:**
- Modify: `websocket/frontend/src/components/theme-toggle.tsx`

**Step 1: 读取当前 ThemeToggle 组件**

Run: `cat websocket/frontend/src/components/theme-toggle.tsx`

**Step 2: 移除图标的 transform 动画**

找到 Sun 和 Moon 图标，移除 `rotate` 和 `scale` 相关的类：

```typescript
<Sun className="h-4 w-4 transition-opacity dark:opacity-0" />
<Moon className="absolute h-4 w-4 opacity-0 transition-opacity dark:opacity-100" />
```

**Step 3: 验证编译**

Run: `cd websocket/frontend && npm run build`
Expected: 构建成功

**Step 4: 提交更改**

```bash
git add websocket/frontend/src/components/theme-toggle.tsx
git commit -m "style: remove transform animations from theme toggle

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

### Task 15: 最终测试和调整

**Files:**
- Test: 所有组件

**Step 1: 启动开发服务器**

Run: `cd websocket/frontend && npm run dev`
Expected: 服务器启动在 http://localhost:5174/

**Step 2: 测试浅色主题**

1. 打开浏览器访问 http://localhost:5174/
2. 切换到浅色主题
3. 检查所有组件的毛玻璃效果
4. 验证边框是否轻量化
5. 检查间距是否合适

**Step 3: 测试深色主题**

1. 切换到深色主题
2. 检查背景色是否更深（#0a0a0b）
3. 验证对比度是否足够
4. 检查所有组件的视觉效果

**Step 4: 测试交互**

1. 测试按钮悬停效果（仅颜色变化，无位移）
2. 测试输入框聚焦效果
3. 测试对话框打开/关闭
4. 验证主题切换过渡是否平滑

**Step 5: 性能检查**

1. 打开浏览器开发者工具
2. 检查 FPS 是否稳定
3. 验证 backdrop-blur 是否影响性能
4. 如有性能问题，考虑减少 blur 强度

**Step 6: 记录需要调整的地方**

如果发现任何视觉问题，记录下来准备微调。

**Step 7: 生产构建测试**

Run: `cd websocket/frontend && npm run build && npm run preview`
Expected: 构建成功，预览服务器启动

**Step 8: 最终提交**

```bash
git add -A
git commit -m "style: complete Conductor.build style migration

- Applied glassmorphism effects across all components
- Updated color scheme for better contrast
- Minimized animations to essential transitions only
- Increased spacing for better breathing room
- Achieved Conductor.build aesthetic

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 验证清单

完成所有任务后，验证以下内容：

- [ ] 所有组件都应用了毛玻璃效果（backdrop-blur）
- [ ] 边框都是轻量化的（border/30 或更低）
- [ ] 移除了所有 transform 动画
- [ ] 保留了颜色过渡动画
- [ ] 间距增大，营造呼吸感
- [ ] 消息气泡最大宽度为 75%
- [ ] 深色主题背景更深（#0a0a0b）
- [ ] 浅色主题使用纯白背景
- [ ] 所有功能正常工作（WebSocket、权限、用户问题）
- [ ] 浅色和深色主题都测试通过
- [ ] 生产构建成功

## 故障排除

**如果毛玻璃效果不显示：**
- 检查 Tailwind 配置是否支持 backdrop-blur
- 验证浏览器是否支持 backdrop-filter
- 尝试增加 blur 强度（从 md 到 lg）

**如果性能下降：**
- 减少 backdrop-blur 的使用范围
- 降低 blur 强度（从 lg 到 md 或 sm）
- 考虑在移动设备上禁用毛玻璃效果

**如果对比度不足：**
- 调整 CSS 变量中的颜色值
- 增加背景不透明度（从 /60 到 /80）
- 调整前景色的亮度

**如果边框太淡：**
- 增加边框不透明度（从 /30 到 /40）
- 调整 --border CSS 变量的颜色值
