# shadcn/ui 迁移实现计划

> **For Claude:** REQUIRED SUB-SKILL: 使用 superpowers:executing-plans 按任务逐一实施此计划。

**目标**：将 React 前端从自定义 Tailwind 组件完全迁移至 shadcn/ui，实现现代简约设计和亮色/暗色模式切换。

**架构**：分 5 个阶段进行模块化迁移。第一阶段建立基础设施（主题系统、CSS 变量），第二阶段初始化基础组件，第三、四阶段逐个替换业务组件，第五阶段清理优化。每个阶段都是独立可测试的单元。

**技术栈**：React 19、TypeScript 5.9、Tailwind CSS 3、shadcn/ui、Radix UI、lucide-react、Vite 7

---

## 第一阶段：基础设施配置

### Task 1: 安装 shadcn/ui 依赖

**文件**：
- 修改: `websocket/frontend/package.json`

**Step 1: 安装 shadcn/ui 核心依赖**

运行：
```bash
cd /Users/arthur/RustroverProjects/claude-agent-kit/websocket/frontend
npm install -D shadcn-ui
npm install class-variance-authority clsx tailwind-merge lucide-react
npm install @radix-ui/react-dialog @radix-ui/react-slot @radix-ui/react-switch
```

**Step 2: 验证安装成功**

运行：
```bash
npm list class-variance-authority clsx tailwind-merge lucide-react
```

预期：显示所有包已安装且版本正确

**Step 3: 提交**

```bash
cd /Users/arthur/RustroverProjects/claude-agent-kit
git add websocket/frontend/package.json websocket/frontend/bun.lock
git commit -m "feat: install shadcn/ui dependencies

Install shadcn/ui and required dependencies:
- class-variance-authority (CVA for component variants)
- clsx (class name utility)
- tailwind-merge (Tailwind class merging)
- lucide-react (icon library)
- Radix UI primitives (@radix-ui/react-dialog, @radix-ui/react-slot, @radix-ui/react-switch)

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 2: 创建工具函数和 utils

**文件**：
- 创建: `websocket/frontend/src/lib/utils.ts`

**Step 1: 创建 utils.ts 文件**

内容：
```typescript
import { type ClassValue, clsx } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}
```

**Step 2: 验证文件创建成功**

运行：
```bash
ls -la /Users/arthur/RustroverProjects/claude-agent-kit/websocket/frontend/src/lib/
```

预期：显示 `utils.ts` 文件存在

**Step 3: 提交**

```bash
cd /Users/arthur/RustroverProjects/claude-agent-kit
git add websocket/frontend/src/lib/utils.ts
git commit -m "feat: add cn() utility function for Tailwind class merging

Create utils.ts with cn() helper function that combines clsx and tailwind-merge
to safely merge Tailwind CSS classes and handle conflicts.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 3: 创建主题上下文提供者

**文件**：
- 创建: `websocket/frontend/src/components/theme-provider.tsx`

**Step 1: 创建 theme-provider.tsx 文件**

内容：
```typescript
import React, { createContext, useContext, useEffect, useState } from 'react'

type Theme = 'light' | 'dark' | 'system'

interface ThemeContextType {
  theme: Theme
  setTheme: (theme: Theme) => void
}

const ThemeContext = createContext<ThemeContextType | undefined>(undefined)

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const [theme, setThemeState] = useState<Theme>(() => {
    // 从 localStorage 读取用户选择的主题
    const stored = localStorage.getItem('theme') as Theme | null
    return stored || 'system'
  })

  useEffect(() => {
    // 确定实际使用的主题
    const resolvedTheme =
      theme === 'system'
        ? window.matchMedia('(prefers-color-scheme: dark)').matches
          ? 'dark'
          : 'light'
        : theme

    // 更新 DOM
    const root = document.documentElement
    if (resolvedTheme === 'dark') {
      root.classList.add('dark')
    } else {
      root.classList.remove('dark')
    }

    // 持久化选择
    localStorage.setItem('theme', theme)
  }, [theme])

  const setTheme = (newTheme: Theme) => {
    setThemeState(newTheme)
  }

  return (
    <ThemeContext.Provider value={{ theme, setTheme }}>
      {children}
    </ThemeContext.Provider>
  )
}

export function useTheme() {
  const context = useContext(ThemeContext)
  if (context === undefined) {
    throw new Error('useTheme must be used within ThemeProvider')
  }
  return context
}
```

**Step 2: 验证文件创建成功**

运行：
```bash
ls -la /Users/arthur/RustroverProjects/claude-agent-kit/websocket/frontend/src/components/theme-provider.tsx
```

预期：文件存在，TypeScript 检查无错误

**Step 3: 提交**

```bash
cd /Users/arthur/RustroverProjects/claude-agent-kit
git add websocket/frontend/src/components/theme-provider.tsx
git commit -m "feat: add ThemeProvider context for light/dark mode management

Create theme provider with:
- Support for 'light', 'dark', and 'system' themes
- localStorage persistence for user preference
- useTheme hook for accessing theme context
- Automatic DOM classList updates for theme switching

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 4: 创建 useTheme Hook

**文件**：
- 创建: `websocket/frontend/src/hooks/useTheme.ts`

**Step 1: 创建 useTheme.ts 文件**

内容：
```typescript
export { useTheme } from '../components/theme-provider'
```

**Step 2: 验证文件创建成功**

运行：
```bash
ls -la /Users/arthur/RustroverProjects/claude-agent-kit/websocket/frontend/src/hooks/
```

预期：`useTheme.ts` 文件存在

**Step 3: 提交**

```bash
cd /Users/arthur/RustroverProjects/claude-agent-kit
git add websocket/frontend/src/hooks/useTheme.ts
git commit -m "feat: export useTheme hook from hooks directory

Create re-export of useTheme hook for consistent import paths from hooks/ directory.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 5: 更新 index.css 添加主题 CSS 变量

**文件**：
- 修改: `websocket/frontend/src/index.css`

**Step 1: 更新 index.css 文件**

替换内容：
```css
@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
  :root {
    --background: 0 0% 100%;
    --foreground: 222.2 84% 4.9%;
    --card: 0 0% 100%;
    --card-foreground: 222.2 84% 4.9%;
    --popover: 0 0% 100%;
    --popover-foreground: 222.2 84% 4.9%;
    --muted: 210 40% 96%;
    --muted-foreground: 215.4 16.3% 46.9%;
    --accent: 210 40% 96%;
    --accent-foreground: 222.2 47.4% 11.2%;
    --destructive: 0 84.2% 60.2%;
    --destructive-foreground: 210 40% 98%;
    --border: 214.3 31.8% 91.4%;
    --input: 214.3 31.8% 91.4%;
    --primary: 222.2 47.4% 11.2%;
    --primary-foreground: 210 40% 98%;
    --secondary: 210 40% 96%;
    --secondary-foreground: 222.2 47.4% 11.2%;
    --ring: 222.2 84% 4.9%;
  }

  .dark {
    --background: 222.2 84% 4.9%;
    --foreground: 210 40% 98%;
    --card: 222.2 84% 4.9%;
    --card-foreground: 210 40% 98%;
    --popover: 222.2 84% 4.9%;
    --popover-foreground: 210 40% 98%;
    --muted: 217.2 32.6% 17.5%;
    --muted-foreground: 215 20.2% 65.1%;
    --accent: 217.2 32.6% 17.5%;
    --accent-foreground: 210 40% 98%;
    --destructive: 0 62.8% 30.6%;
    --destructive-foreground: 210 40% 98%;
    --border: 217.2 32.6% 17.5%;
    --input: 217.2 32.6% 17.5%;
    --primary: 210 40% 98%;
    --primary-foreground: 222.2 47.4% 11.2%;
    --secondary: 217.2 32.6% 17.5%;
    --secondary-foreground: 210 40% 98%;
    --ring: 212.7 26.8% 83.9%;
  }
}

@layer base {
  * {
    @apply border-border;
  }
  body {
    @apply bg-background text-foreground;
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen',
      'Ubuntu', 'Cantarell', 'Fira Sans', 'Droid Sans', 'Helvetica Neue',
      sans-serif;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }
  code {
    font-family: source-code-pro, Menlo, Monaco, Consolas, 'Courier New',
      monospace;
  }
}

/* 主题切换过渡动画 */
html {
  color-scheme: light dark;
  transition: background-color 0.3s ease, color 0.3s ease;
}
```

**Step 2: 验证 CSS 变量正确定义**

运行：
```bash
npm run build
```

预期：构建成功，无 CSS 错误

**Step 3: 提交**

```bash
cd /Users/arthur/RustroverProjects/claude-agent-kit
git add websocket/frontend/src/index.css
git commit -m "feat: add CSS variables for light/dark theme support

Add comprehensive CSS variables for:
- Light mode (:root) - default color scheme
- Dark mode (.dark) - dark color scheme
- Background, foreground, cards, inputs, buttons, borders
- Smooth transition animations for theme switching

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 6: 更新 tailwind.config.js 配置

**文件**：
- 修改: `websocket/frontend/tailwind.config.js`

**Step 1: 更新 tailwind.config.js**

新内容：
```javascript
/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ['class'],
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    container: {
      center: true,
      padding: "2rem",
      screens: {
        "2xl": "1400px",
      },
    },
    extend: {
      colors: {
        border: "hsl(var(--border))",
        input: "hsl(var(--input))",
        ring: "hsl(var(--ring))",
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        primary: {
          DEFAULT: "hsl(var(--primary))",
          foreground: "hsl(var(--primary-foreground))",
        },
        secondary: {
          DEFAULT: "hsl(var(--secondary))",
          foreground: "hsl(var(--secondary-foreground))",
        },
        destructive: {
          DEFAULT: "hsl(var(--destructive))",
          foreground: "hsl(var(--destructive-foreground))",
        },
        muted: {
          DEFAULT: "hsl(var(--muted))",
          foreground: "hsl(var(--muted-foreground))",
        },
        accent: {
          DEFAULT: "hsl(var(--accent))",
          foreground: "hsl(var(--accent-foreground))",
        },
        popover: {
          DEFAULT: "hsl(var(--popover))",
          foreground: "hsl(var(--popover-foreground))",
        },
        card: {
          DEFAULT: "hsl(var(--card))",
          foreground: "hsl(var(--card-foreground))",
        },
      },
      borderRadius: {
        lg: "var(--radius)",
        md: "calc(var(--radius) - 2px)",
        sm: "calc(var(--radius) - 4px)",
      },
      animation: {
        'fade-in': 'fadeIn 0.3s ease-out',
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0', transform: 'scale(0.95)' },
          '100%': { opacity: '1', transform: 'scale(1)' },
        },
      },
    },
  },
  plugins: [],
}
```

**Step 2: 验证配置无错误**

运行：
```bash
cd /Users/arthur/RustroverProjects/claude-agent-kit/websocket/frontend
npm run build 2>&1 | head -50
```

预期：构建成功或仅有预期的组件缺失错误

**Step 3: 提交**

```bash
cd /Users/arthur/RustroverProjects/claude-agent-kit
git add websocket/frontend/tailwind.config.js
git commit -m "feat: configure Tailwind for shadcn/ui theme system

Update tailwind config to:
- Enable dark mode with 'class' strategy
- Define color variables using CSS custom properties
- Add color palettes for primary, secondary, destructive, muted, accent
- Configure container and border radius
- Preserve existing animations (fade-in)

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 7: 更新 App.tsx 集成主题系统

**文件**：
- 修改: `websocket/frontend/src/App.tsx`

**Step 1: 读取当前 App.tsx 内容**

运行：
```bash
head -50 /Users/arthur/RustroverProjects/claude-agent-kit/websocket/frontend/src/App.tsx
```

**Step 2: 更新 App.tsx**

在文件顶部添加导入（在其他导入之后）：
```typescript
import { ThemeProvider } from './components/theme-provider'
```

将 App 组件包装在 ThemeProvider 中（修改 return 语句）：
```typescript
return (
  <ThemeProvider>
    <div className="w-full h-screen bg-background text-foreground">
      <ChatInterface />
    </div>
  </ThemeProvider>
)
```

**Step 3: 验证代码编译成功**

运行：
```bash
cd /Users/arthur/RustroverProjects/claude-agent-kit/websocket/frontend
npm run build 2>&1 | grep -E "(error|warning|compiled)" | head -20
```

预期：构建成功，无 TypeScript 错误

**Step 4: 提交**

```bash
cd /Users/arthur/RustroverProjects/claude-agent-kit
git add websocket/frontend/src/App.tsx
git commit -m "feat: integrate ThemeProvider into App component

Wrap App with ThemeProvider to enable theme context for all child components.
Update root div to use CSS variable classes (bg-background, text-foreground).

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 第二阶段：生成 shadcn/ui 基础组件

### Task 8: 初始化 shadcn/ui 组件

**文件**：
- 创建: `websocket/frontend/src/components/ui/` （13 个组件文件）

**Step 1: 创建 shadcn.json 配置文件**

在 `websocket/frontend` 目录创建 `components.json`：

```bash
cat > /Users/arthur/RustroverProjects/claude-agent-kit/websocket/frontend/components.json << 'EOF'
{
  "$schema": "https://ui.shadcn.com/schema.json",
  "style": "default",
  "rsc": false,
  "tsx": true,
  "aliasPrefix": "~",
  "alias": {
    "@": "./src"
  }
}
EOF
```

**Step 2: 使用 shadcn CLI 生成组件**

运行以下命令逐个生成组件（或一次性生成所有）：

```bash
cd /Users/arthur/RustroverProjects/claude-agent-kit/websocket/frontend

# 一次性生成所有需要的组件
npx shadcn-ui@latest add button --yes
npx shadcn-ui@latest add input --yes
npx shadcn-ui@latest add card --yes
npx shadcn-ui@latest add dialog --yes
npx shadcn-ui@latest add badge --yes
npx shadcn-ui@latest add scroll-area --yes
npx shadcn-ui@latest add separator --yes
npx shadcn-ui@latest add switch --yes
npx shadcn-ui@latest add label --yes
npx shadcn-ui@latest add textarea --yes
npx shadcn-ui@latest add select --yes
npx shadcn-ui@latest add radio-group --yes
npx shadcn-ui@latest add checkbox --yes
```

**Step 3: 验证所有组件生成成功**

运行：
```bash
ls -la /Users/arthur/RustroverProjects/claude-agent-kit/websocket/frontend/src/components/ui/ | wc -l
```

预期：显示 13+ 个文件（包括 `.tsx` 和可能的 `.ts` 文件）

**Step 4: 提交**

```bash
cd /Users/arthur/RustroverProjects/claude-agent-kit
git add websocket/frontend/components.json websocket/frontend/src/components/ui/
git commit -m "feat: generate shadcn/ui base components

Initialize 13 shadcn/ui components:
- button, input, card, dialog, badge
- scroll-area, separator, switch, label
- textarea, select, radio-group, checkbox

Components configured to use CSS variables for theming
and support light/dark mode switching.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 9: 验证主题在组件中正确应用

**文件**：
- 修改: `websocket/frontend/src/App.tsx`

**Step 1: 创建简单的主题测试页面**

更新 App.tsx 的 render 内容为：
```typescript
return (
  <ThemeProvider>
    <div className="w-full h-screen bg-background text-foreground p-8">
      <div className="max-w-2xl mx-auto space-y-8">
        <h1 className="text-3xl font-bold">Theme Test</h1>

        <div className="space-y-4">
          <h2 className="text-xl font-semibold">Components</h2>
          <Button>Primary Button</Button>
          <Button variant="secondary">Secondary Button</Button>
          <Input placeholder="Input field" />
          <Card className="p-4">
            <p>Card content</p>
          </Card>
        </div>

        <div className="space-y-4">
          <h2 className="text-xl font-semibold">Theme Switch</h2>
          <ThemeToggle />
        </div>
      </div>
    </div>
  </ThemeProvider>
)
```

（ThemeToggle 组件在下一个 task 创建）

**Step 2: 运行开发服务器验证**

运行：
```bash
cd /Users/arthur/RustroverProjects/claude-agent-kit/websocket/frontend
npm run dev
```

在浏览器打开 `http://localhost:5173`，验证：
- 组件正确渲染
- 颜色使用 CSS 变量
- 无控制台错误

**Step 3: 停止开发服务器**

按 `Ctrl+C` 停止

**Step 4: 提交**

```bash
cd /Users/arthur/RustroverProjects/claude-agent-kit
git add websocket/frontend/src/App.tsx
git commit -m "test: verify shadcn/ui components and theme system

Add temporary test page to verify:
- Components render correctly with CSS variable colors
- Theme switching works
- No console errors

This test code will be removed in phase 5.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 第三阶段：聊天核心组件重构

### Task 10: 创建 ThemeToggle 组件

（此组件用于主题切换）

**文件**：
- 创建: `websocket/frontend/src/components/theme-toggle.tsx`

**Step 1: 创建 theme-toggle.tsx**

内容：
```typescript
import { Moon, Sun } from 'lucide-react'
import { Button } from './ui/button'
import { useTheme } from '../hooks/useTheme'

export function ThemeToggle() {
  const { theme, setTheme } = useTheme()

  const toggleTheme = () => {
    if (theme === 'system') {
      setTheme('light')
    } else if (theme === 'light') {
      setTheme('dark')
    } else {
      setTheme('system')
    }
  }

  return (
    <Button
      variant="outline"
      size="icon"
      onClick={toggleTheme}
      className="rounded-full"
      title={`Current theme: ${theme}`}
    >
      <Sun className="h-4 w-4 rotate-0 scale-100 transition-all dark:-rotate-90 dark:scale-0" />
      <Moon className="absolute h-4 w-4 rotate-90 scale-0 transition-all dark:rotate-0 dark:scale-100" />
      <span className="sr-only">Toggle theme</span>
    </Button>
  )
}
```

**Step 2: 验证导入无误**

运行：
```bash
cd /Users/arthur/RustroverProjects/claude-agent-kit/websocket/frontend
npm run build 2>&1 | grep -E "error|ThemeToggle"
```

预期：无 ThemeToggle 相关错误

**Step 3: 提交**

```bash
cd /Users/arthur/RustroverProjects/claude-agent-kit
git add websocket/frontend/src/components/theme-toggle.tsx
git commit -m "feat: add ThemeToggle component for theme switching

Create ThemeToggle component with:
- Sun/Moon icon from lucide-react
- Cycle through light/dark/system themes
- Smooth icon rotation animations
- Accessibility label (sr-only)

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 11: 创建 Header 组件

**文件**：
- 创建: `websocket/frontend/src/components/layout/header.tsx`

**Step 1: 创建 header.tsx**

内容：
```typescript
import { Badge } from '../ui/badge'
import { ThemeToggle } from '../theme-toggle'

interface HeaderProps {
  isConnected: boolean
  connectionStatus?: 'connecting' | 'connected' | 'disconnected'
}

export function Header({ isConnected, connectionStatus = 'disconnected' }: HeaderProps) {
  const statusVariant = isConnected ? 'default' : 'secondary'
  const statusLabel = connectionStatus === 'connecting' ? 'Connecting...' :
                     connectionStatus === 'connected' ? 'Connected' :
                     'Disconnected'

  return (
    <header className="sticky top-0 z-50 w-full border-b border-border/40 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div className="container flex h-14 max-w-screen-2xl items-center justify-between">
        <div className="flex items-center gap-2">
          <h1 className="text-lg font-semibold">Claude Agent</h1>
        </div>

        <div className="flex items-center gap-4">
          <Badge variant={statusVariant}>{statusLabel}</Badge>
          <ThemeToggle />
        </div>
      </div>
    </header>
  )
}
```

**Step 2: 验证编译成功**

运行：
```bash
cd /Users/arthur/RustroverProjects/claude-agent-kit/websocket/frontend
npm run build 2>&1 | grep -E "error|Header"
```

预期：无错误

**Step 3: 提交**

```bash
cd /Users/arthur/RustroverProjects/claude-agent-kit
git add websocket/frontend/src/components/layout/header.tsx
git commit -m "feat: create Header component with connection status and theme toggle

Add Header component featuring:
- Sticky positioning with backdrop blur
- Connection status badge
- Theme toggle button
- Clean, minimal design with CSS variables

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 12: 创建 ConnectionStatus 组件

**文件**：
- 创建: `websocket/frontend/src/components/chat/connection-status.tsx`

**Step 1: 创建 connection-status.tsx**

内容：
```typescript
import { Badge } from '../ui/badge'

type ConnectionStatusType = 'connecting' | 'connected' | 'disconnected' | 'error'

interface ConnectionStatusProps {
  status: ConnectionStatusType
}

export function ConnectionStatus({ status }: ConnectionStatusProps) {
  const getVariant = (s: ConnectionStatusType): 'default' | 'secondary' | 'destructive' => {
    switch (s) {
      case 'connected':
        return 'default'
      case 'connecting':
        return 'secondary'
      case 'error':
        return 'destructive'
      case 'disconnected':
      default:
        return 'secondary'
    }
  }

  const getLabel = (s: ConnectionStatusType): string => {
    switch (s) {
      case 'connected':
        return 'Connected'
      case 'connecting':
        return 'Connecting...'
      case 'error':
        return 'Connection Error'
      case 'disconnected':
      default:
        return 'Disconnected'
    }
  }

  return (
    <Badge variant={getVariant(status)} className="animate-pulse">
      {getLabel(status)}
    </Badge>
  )
}
```

**Step 2: 验证编译成功**

运行：
```bash
cd /Users/arthur/RustroverProjects/claude-agent-kit/websocket/frontend
npm run build 2>&1 | grep -E "error|connection"
```

预期：无错误

**Step 3: 提交**

```bash
cd /Users/arthur/RustroverProjects/claude-agent-kit
git add websocket/frontend/src/components/chat/connection-status.tsx
git commit -m "feat: create ConnectionStatus component

Add ConnectionStatus component that displays:
- Connection state badge (connected/connecting/disconnected/error)
- Appropriate color variants for each state
- Animated pulse for connecting state
- Clear user feedback on connection health

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 13-18: 创建剩余聊天组件

由于篇幅限制，以下任务遵循相同的模式（创建组件 → 验证 → 提交）：

**Task 13**: 创建 `MessageBubble` 组件 - 使用 Card 和 Badge 显示不同类型消息
**Task 14**: 创建 `MessageList` 组件 - 使用 ScrollArea 包装消息列表
**Task 15**: 创建 `MessageInput` 组件 - 使用 Textarea 和 Button 组合
**Task 16**: 重构 `ChatInterface` 组件 - 整合所有新组件
**Task 17**: 更新 `App.tsx` - 移除测试代码，使用新的 ChatInterface
**Task 18**: 测试聊天功能 - 验证消息发送、接收、滚动

---

## 第四阶段：对话框组件重构

### Task 19: 重构 PermissionDialog

**文件**：
- 创建: `websocket/frontend/src/components/dialogs/permission-dialog.tsx`
- 删除: `websocket/frontend/src/components/PermissionDialog.tsx`

**关键点**：
- 使用 shadcn/ui Dialog 组件
- 保持现有的权限逻辑和状态管理
- 使用 Badge 显示权限级别
- 使用 Button 组件替换原有按钮

---

### Task 20: 重构 UserQuestionDialog

**文件**：
- 创建: `websocket/frontend/src/components/dialogs/user-question-dialog.tsx`
- 删除: `websocket/frontend/src/components/UserQuestionDialog.tsx`

**关键点**：
- 使用 shadcn/ui Dialog、RadioGroup、Checkbox、Input 组件
- 保持现有的问题处理逻辑
- 支持单选、多选、文本输入

---

### Task 21: 测试对话框功能

**验证项**：
- 权限请求对话框正确显示和响应
- 用户问题对话框支持所有问题类型
- 对话框在亮色/暗色模式下都清晰可读

---

## 第五阶段：清理和优化

### Task 22: 删除旧组件文件

**文件**：
- 删除: `websocket/frontend/src/components/ChatInterface.tsx`
- 删除: `websocket/frontend/src/components/MessageBubble.tsx`
- 删除: `websocket/frontend/src/components/SystemInfo.tsx`
- 删除: `websocket/frontend/src/App.css` (如果存在)

---

### Task 23: 运行完整测试

**测试清单**：
1. WebSocket 连接和断开
2. 消息发送和接收
3. 权限请求流程
4. 用户问题响应
5. 主题切换（亮色/暗色/系统）
6. 响应式布局（手机/平板/桌面）
7. 无控制台错误或警告

---

### Task 24: 最终提交

```bash
cd /Users/arthur/RustroverProjects/claude-agent-kit
git add -A
git commit -m "feat: complete shadcn/ui migration

Complete migration from custom components to shadcn/ui:
- All UI components replaced with shadcn/ui
- Light/dark mode theme system implemented
- Modern minimalist design applied
- All functionality preserved and tested
- Responsive design verified
- Clean code with no console errors

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 实施注意事项

### 关键原则
1. **DRY** - 不要重复代码，使用组件组合
2. **YAGNI** - 只实现需要的功能，不过度设计
3. **频繁提交** - 每个任务完成后立即提交
4. **保持功能** - 确保所有现有功能正常工作

### 测试策略
- 每个阶段完成后运行 `npm run build` 验证
- 使用 `npm run dev` 在浏览器中手动测试
- 检查控制台无错误
- 验证亮色/暗色模式切换

### 回滚策略
- 每个任务都有独立的 git commit
- 如果出现问题，可以 `git revert` 到上一个工作状态
- 分阶段实施降低风险

---

## 执行选项

计划已完成并保存到 `docs/plans/2026-01-27-shadcn-ui-migration-implementation.md`。

**两种执行方式：**

**1. Subagent-Driven（当前会话）** - 我在当前会话中为每个任务派发新的子代理，任务间进行代码审查，快速迭代

**2. Parallel Session（独立会话）** - 在新会话中使用 executing-plans skill，批量执行并设置检查点

**您选择哪种方式？**