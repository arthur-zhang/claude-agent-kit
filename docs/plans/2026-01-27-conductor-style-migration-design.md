# Conductor.build 风格迁移设计文档

> **创建日期**: 2026-01-27
> **目标**: 将当前 shadcn/ui 界面改造为 Conductor.build 风格

## 设计目标

将现有的 WebSocket 聊天界面改造为 Conductor.build 的视觉风格，专注于以下核心设计元素：
- 毛玻璃效果（backdrop-blur）
- 轻量化边框设计
- 平滑的悬停动画（最小化）
- 极简主义风格

## 核心设计原则

### 1. 视觉风格

**配色和主题系统：**
- 保留浅色/深色/系统主题切换功能
- 深色主题：使用更深的背景色（`#0a0a0a` 或 `#09090b`），增强对比度
- 浅色主题：采用柔和的灰白色背景，保持 Conductor 的极简美学
- 边框颜色：使用非常细的边框（`border-border/20` 或 `border-border/30`），轻量化设计
- 背景透明度：组件使用半透明背景（`bg-background/80` 或 `bg-card/60`）

**毛玻璃效果应用：**
- 全局应用 `backdrop-blur-md` 或 `backdrop-blur-lg`
- 主要组件：消息气泡、卡片、对话框、输入区域、按钮
- 头部导航：使用 `backdrop-blur-xl` + 半透明背景，实现浮动效果
- 系统信息面板：毛玻璃卡片设计

**极简主义原则：**
- 移除不必要的阴影效果
- 使用细边框代替厚重的卡片边框
- 增大组件间距，营造呼吸感
- 减少视觉噪音，专注内容

### 2. 组件设计

**消息气泡（MessageBubble）：**
- 用户消息：`bg-primary/80` + `backdrop-blur-md` + `border-primary/30`
- 助手消息：`bg-card/60` + `backdrop-blur-md` + `border-border/30`
- 系统消息：Badge 设计 + `backdrop-blur-sm`
- 工具使用/结果：嵌套的毛玻璃卡片 `bg-muted/40` + `backdrop-blur-md`
- 移除所有阴影效果
- 最大宽度从 80% 改为 75%，增加呼吸感

**头部导航（Header）：**
- 固定顶部：`bg-background/80` + `backdrop-blur-xl`
- 底部细边框分隔线：`border-b border-border/30`
- 连接状态 Badge：`bg-secondary/60` + `backdrop-blur-sm`
- 主题切换按钮：轻量化设计，细边框

**输入区域（MessageInput）：**
- 底部固定：`bg-background/80` + `backdrop-blur-lg`
- 顶部细边框：`border-t border-border/30`
- Textarea：`bg-background/60` + `backdrop-blur-sm` + `border-border/30`
- 发送按钮：`bg-primary/80` + `backdrop-blur-md`
- 增大内边距从 `p-4` 到 `p-6`

**卡片和面板：**
- 系统信息面板：`bg-card/60` + `backdrop-blur-md` + `border-border/30`
- 设置面板：`bg-card/60` + `backdrop-blur-md`
- 所有 Card 组件：统一使用半透明背景 + 毛玻璃效果
- 增大内边距和组件间距

**对话框组件（PermissionDialog & UserQuestionDialog）：**
- 背景遮罩：`backdrop-blur-sm` 替代纯黑遮罩
- 对话框内容：`bg-background/90` + `backdrop-blur-xl` + `border-border/30`
- 内部卡片：`bg-muted/40` + `backdrop-blur-md`
- Badge 组件：`bg-secondary/60` + `backdrop-blur-sm`

**按钮系统：**
- Primary 按钮：`bg-primary/80` + `backdrop-blur-md` + `border-primary/30`
- Outline 按钮：`bg-background/60` + `backdrop-blur-md` + `border-border/30`
- Ghost 按钮：悬停时 `bg-accent/40` + `backdrop-blur-sm`
- Icon 按钮：轻量化设计，细边框
- 移除所有位移动画

### 3. 交互设计（最小化动画）

**保留的过渡效果：**
- 主题切换：`transition-colors duration-300`
- 按钮悬停：`transition-colors duration-200`
- 对话框打开/关闭：保留默认的淡入淡出

**移除的动画：**
- 所有 transform 动画（translate、scale 等）
- 消息的 fade-in 动画
- 图标的位移动画

### 4. 布局调整

**间距优化：**
- 组件内边距：从 `p-4` 增加到 `p-6` 或 `p-8`
- 消息气泡间距：从 `mb-4` 增加到 `mb-6`
- 头部和输入区域：使用更宽松的内边距
- 系统信息面板：增大网格间距

**呼吸感：**
- 消息气泡最大宽度：从 80% 改为 75%
- 增加组件之间的垂直间距
- 使用更大的圆角（从 `rounded-md` 到 `rounded-lg`）

## CSS 变量更新

需要更新 `index.css` 中的 CSS 变量：

**深色主题：**
```css
.dark {
  --background: 10 10 11;  /* #0a0a0b - 更深的背景 */
  --foreground: 250 250 250;  /* 更亮的前景色 */
  --card: 24 24 27;  /* 稍亮的卡片背景 */
  --border: 39 39 42;  /* 更细腻的边框色 */
  /* 其他变量保持或微调 */
}
```

**浅色主题：**
```css
:root {
  --background: 255 255 255;  /* 纯白背景 */
  --foreground: 10 10 11;  /* 深色文字 */
  --card: 250 250 250;  /* 浅灰卡片 */
  --border: 229 229 229;  /* 浅色边框 */
  /* 其他变量保持或微调 */
}
```

## 实施计划

### Phase 1: CSS 基础设施
1. 更新 `index.css` 中的 CSS 变量
2. 添加全局毛玻璃效果支持类
3. 移除不必要的动画定义

### Phase 2: 基础 UI 组件
4. 更新 `button.tsx` - 添加毛玻璃效果和轻量边框
5. 更新 `card.tsx` - 半透明背景 + backdrop-blur
6. 更新 `badge.tsx` - 毛玻璃效果
7. 更新 `input.tsx` 和 `textarea.tsx` - 轻量化设计

### Phase 3: 布局组件
8. 更新 `Header` - 毛玻璃浮动效果
9. 更新 `MessageInput` - 毛玻璃底部栏
10. 更新 `MessageBubble` - 毛玻璃气泡设计
11. 更新 `MessageList` - 调整间距

### Phase 4: 对话框组件
12. 更新 `PermissionDialog` - 毛玻璃对话框
13. 更新 `UserQuestionDialog` - 毛玻璃对话框
14. 更新对话框遮罩效果

### Phase 5: 整体调整
15. 更新 `ChatInterface` - 调整布局间距
16. 移除所有不必要的动画
17. 测试浅色和深色主题
18. 最终视觉调整和优化

## 技术要点

**Tailwind 类名模式：**
- 半透明背景：`bg-{color}/{opacity}` (如 `bg-card/60`)
- 毛玻璃效果：`backdrop-blur-{size}` (sm/md/lg/xl)
- 细边框：`border border-{color}/{opacity}` (如 `border-border/30`)
- 过渡效果：`transition-colors duration-{time}`

**性能考虑：**
- backdrop-blur 可能影响性能，在移动设备上测试
- 使用 `will-change-[transform]` 优化动画性能（如果需要）
- 避免过度使用毛玻璃效果在嵌套元素上

**浏览器兼容性：**
- backdrop-filter 在现代浏览器中支持良好
- 为不支持的浏览器提供降级方案（实心背景）

## 预期效果

完成后，界面将呈现：
- 现代科技感的毛玻璃视觉效果
- 轻盈、通透的设计语言
- 极简主义的布局和交互
- 与 Conductor.build 一致的视觉风格
- 保留完整的功能性（WebSocket、权限、用户问题等）
