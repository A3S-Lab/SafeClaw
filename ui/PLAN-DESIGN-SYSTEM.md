# SafeClaw 设计系统审查与改进方案

基于 [Vercel Web Interface Guidelines](https://github.com/vercel-labs/web-interface-guidelines) 对现有 UI 进行全面审查，结合微信风格多会话需求，整理出设计系统改进方案。

---

## 一、现有问题审查

### 1. 可访问性（Accessibility）

| 问题 | 文件 | 严重度 |
|------|------|--------|
| ActivityBar 用 `<div>` + `onClick` 做导航，无键盘支持、无 ARIA role | `activity-bar.tsx:30-48` | 🔴 严重 |
| SessionItem 用 `<div onClick>` 做可选列表项，无 `role="listbox"` / `role="option"` | `agent-session-list.tsx:80-99` | 🔴 严重 |
| 聊天输入区的"发送"和"停止"按钮用原生 `<button>` 但无 `aria-label` | `agent-chat.tsx:347-370` | 🟡 中等 |
| PermissionBanner 的允许/拒绝按钮无 `aria-label`，屏幕阅读器只读到"允许"/"拒绝"缺少上下文 | `agent-chat.tsx:134-151` | 🟡 中等 |
| User 组件的 `<img>` alt 为空字符串，应提供有意义的 alt | `user.tsx:51-55` | 🟡 中等 |
| 设置页侧边栏用 `<button>` 但无 `aria-current="page"` 标记当前项 | `settings/index.tsx:35-48` | 🟡 中等 |
| Main 组件用 `<aside>` 语义错误，主内容区应为 `<main>` | `main.tsx:3-8` | 🟡 中等 |
| 无 skip-to-content 链接 | 全局 | 🟡 中等 |
| 无 `prefers-reduced-motion` 适配，页面切换有 blur 动画 | `chat/index.tsx:40-43` | 🟢 轻微 |

### 2. 焦点管理（Focus Management）

| 问题 | 文件 | 严重度 |
|------|------|--------|
| 无可见焦点环（focus ring）全局样式，依赖浏览器默认 | `index.css` | 🔴 严重 |
| 创建会话对话框打开后焦点未自动移到第一个输入框（Radix Dialog 默认处理，但需验证） | `create-session-dialog.tsx` | 🟡 中等 |
| 切换会话后焦点未移到聊天输入框 | `agent/index.tsx:24-33` | 🟡 中等 |
| ActivityBar 导航项无 `tabIndex`，无法 Tab 遍历 | `activity-bar.tsx` | 🔴 严重 |

### 3. 键盘支持（Keyboard）

| 问题 | 文件 | 严重度 |
|------|------|--------|
| ActivityBar 不支持 ↑↓ 箭头键导航 | `activity-bar.tsx` | 🔴 严重 |
| 会话列表不支持 ↑↓ 箭头键选择会话 | `agent-session-list.tsx` | 🟡 中等 |
| 聊天消息列表无键盘导航 | `agent-chat.tsx` | 🟢 轻微 |
| `window.prompt()` / `window.confirm()` 用于重命名/删除，应替换为自定义对话框 | `agent-session-list.tsx:50-56,72-73` | 🟡 中等 |

### 4. 语义化 HTML（Semantic HTML）

| 问题 | 文件 | 修复 |
|------|------|------|
| `Main` 用 `<aside>` 包裹主内容 | `main.tsx` | 改为 `<main>` |
| `ActivityBar` 的 `<aside>` 缺少 `role="navigation"` 或用 `<nav>` | `activity-bar.tsx:78` | 改为 `<nav aria-label="主导航">` |
| 会话列表无 `role="listbox"`，列表项无 `role="option"` | `agent-session-list.tsx` | 添加 ARIA roles |
| 聊天消息区无 `role="log"` 或 `aria-live="polite"` | `agent-chat.tsx` | 添加 live region |
| 设置页 `<main>` 缺少 `aria-label` | `settings/index.tsx:273` | 添加标签 |

### 5. 设计 Token 问题

| 问题 | 文件 | 说明 |
|------|------|------|
| `moment` 和 `market` 颜色用硬编码 hex 而非 HSL CSS 变量 | `tailwind.config.js:61-68` | 不一致，dark mode 下不会自动适配 |
| `body` 用 `bg-accent` 而非 `bg-background` | `index.css:106` | 语义不对，accent 是强调色不是背景色 |
| `--header-height` CSS 变量在 layout.tsx 中引用但未在 `:root` 定义 | `layout.tsx:64` | 缺少定义 |
| `w-[48]` 硬编码 ActivityBar 宽度，应为 CSS 变量 | `activity-bar.tsx:78` | 应统一为 `--activity-bar-width` |
| 无 `--spacing` 系统，间距靠 Tailwind 类名散落各处 | 全局 | 缺少统一间距规范 |
| `shadow-weak` / `shadow-strong` 用 rgba 硬编码，dark mode 下效果差 | `tailwind.config.js:76-78` | 应适配暗色模式 |

### 6. 动效与性能

| 问题 | 文件 | 说明 |
|------|------|------|
| 页面切换 blur 动画 500ms 偏长，且无 `prefers-reduced-motion` 适配 | `chat/index.tsx:40-43` | 建议 200-300ms，尊重用户偏好 |
| `no-scrollbar` 工具类隐藏滚动条，影响可访问性 | `index.css:121-131` | 滚动条是重要的视觉反馈 |
| `KeepAlive max={18}` 缓存 18 个页面，内存开销大 | `chat/index.tsx:34` | 当前只有 3 个路由，max=5 足够 |

### 7. 组件设计问题

| 问题 | 文件 | 说明 |
|------|------|------|
| `Layout.Footer` 的 displayName 错误设为 `"Footer"` 但赋值给 `Header` | `layout.tsx:121` | Bug: `Header.displayName = "Footer"` |
| ChatBubble 用 `React.cloneElement` 注入 props，不如用 Context 或 Compound Component | `chat-bubble.tsx:43-49` | 脆弱，子组件顺序敏感 |
| AgentChat 未复用 `components/custom/chat/` 下的通用组件 | `agent-chat.tsx` | 重复实现 |
| `useReactive` (ahooks) 在 AgentInput 中使用，与 Valtio 混用两套响应式方案 | `agent-chat.tsx:303` | 应统一用 `useState` 或 Valtio |

---

## 二、设计 Token 重构

### 2.1 新增 CSS 变量

在 `index.css` 的 `:root` 中补充缺失的 token：

```css
:root {
  /* Layout */
  --activity-bar-width: 48px;
  --session-list-width: 280px;
  --header-height: 48px;

  /* Chat-specific */
  --chat-bubble-max-width: 85%;
  --chat-avatar-size: 32px;
  --chat-bubble-radius: 12px;
  --chat-bubble-user: var(--primary);
  --chat-bubble-user-fg: var(--primary-foreground);
  --chat-bubble-assistant: var(--muted);
  --chat-bubble-assistant-fg: var(--foreground);

  /* Persona avatar sizes */
  --avatar-xs: 24px;
  --avatar-sm: 32px;
  --avatar-md: 40px;
  --avatar-lg: 48px;
  --avatar-xl: 64px;

  /* Transitions */
  --transition-fast: 150ms;
  --transition-normal: 200ms;
  --transition-slow: 300ms;

  /* Focus */
  --focus-ring-width: 2px;
  --focus-ring-offset: 2px;
  --focus-ring-color: var(--ring);
}
```

### 2.2 修复 body 背景色

```css
/* 修复前 */
body { @apply bg-accent text-accent-foreground font-sans; }

/* 修复后 */
body { @apply bg-background text-foreground font-sans; }
```

### 2.3 全局焦点环样式

```css
@layer base {
  /* 可见焦点环 - 仅键盘导航时显示 */
  :focus-visible {
    outline: var(--focus-ring-width) solid hsl(var(--focus-ring-color));
    outline-offset: var(--focus-ring-offset);
    border-radius: var(--radius);
  }

  /* 尊重用户动效偏好 */
  @media (prefers-reduced-motion: reduce) {
    *, *::before, *::after {
      animation-duration: 0.01ms !important;
      animation-iteration-count: 1 !important;
      transition-duration: 0.01ms !important;
    }
  }
}
```

### 2.4 统一 moment/market 颜色为 HSL 变量

```css
:root {
  --moment-primary: 220 30% 53%;    /* #576B95 */
  --market-primary: 33 93% 66%;     /* #F9A856 */
}
.dark {
  --moment-primary: 220 40% 65%;
  --market-primary: 33 90% 60%;
}
```

---

## 三、组件改进清单

### 3.1 ActivityBar → 可访问导航

```tsx
// 修复前: <div onClick> 无键盘支持
<div className={...} onClick={onClick}>

// 修复后: <button> + role="navigation" + aria-current
<nav aria-label="主导航">
  <button
    role="tab"
    aria-selected={isActive}
    aria-label={label}
    tabIndex={0}
    onClick={onClick}
    onKeyDown={handleArrowKeys}
    className={...}
  >
```

### 3.2 Main → 语义化

```tsx
// 修复前
<aside className="flex flex-col h-full w-full overflow-hidden">

// 修复后
<main className="flex flex-col h-full w-full overflow-hidden">
```

### 3.3 SessionItem → 可访问列表

```tsx
// 会话列表容器
<div role="listbox" aria-label="会话列表" aria-activedescendant={currentSessionId}>

// 每个会话项
<div
  role="option"
  aria-selected={isActive}
  tabIndex={isActive ? 0 : -1}
  onKeyDown={handleArrowKeys}
  onClick={onClick}
>
```

### 3.4 AgentChat → Live Region

```tsx
// 消息区域添加 aria-live，让屏幕阅读器播报新消息
<div role="log" aria-live="polite" aria-label="聊天消息">
  <Virtuoso ... />
</div>
```

### 3.5 Layout.Footer displayName Bug

```tsx
// 修复前 (line 121)
Header.displayName = "Footer";

// 修复后
Footer.displayName = "Footer";
```

### 3.6 window.prompt/confirm → 自定义对话框

用已有的 `AlertDialog` (shadcn/ui) 替换 `window.prompt()` 和 `window.confirm()`，保持视觉一致性。

---

## 四、微信风格 + 设计规范融合

将之前的 PLAN-WECHAT-UI.md 中的 Persona 功能与设计规范改进合并，确保新功能从一开始就符合规范。

### 4.1 Persona 卡片组件规范

```tsx
// 角色选择卡片 - 符合 Web Interface Guidelines
<button
  role="radio"
  aria-checked={isSelected}
  aria-label={`选择 ${persona.name}: ${persona.description}`}
  className={cn(
    "flex flex-col items-center gap-2 p-4 rounded-lg border-2 transition-colors",
    "focus-visible:outline focus-visible:outline-2 focus-visible:outline-ring",
    isSelected ? "border-primary bg-primary/5" : "border-transparent hover:border-border"
  )}
  onClick={() => onSelect(persona.id)}
>
  <NiceAvatar className="w-12 h-12" {...persona.avatar} />
  <span className="text-sm font-medium">{persona.name}</span>
</button>
```

### 4.2 微信风格会话列表规范

```tsx
// 会话列表项 - 可访问 + 微信风格
<div
  role="option"
  aria-selected={isActive}
  tabIndex={isActive ? 0 : -1}
  className={cn(
    "flex items-center gap-3 px-3 py-3 cursor-pointer",
    "transition-colors duration-[var(--transition-fast)]",
    "hover:bg-accent/50 focus-visible:bg-accent/50",
    isActive && "bg-accent"
  )}
>
  {/* 头像 + 在线状态 */}
  <div className="relative shrink-0">
    <NiceAvatar className="w-[var(--avatar-lg)] h-[var(--avatar-lg)]" {...avatar} />
    <StatusDot status={info.state} />
  </div>

  {/* 名称 + 预览 */}
  <div className="flex-1 min-w-0">
    <div className="flex justify-between items-baseline">
      <span className="text-sm font-medium truncate">{name}</span>
      <time className="text-xs text-muted-foreground shrink-0 ml-2"
            dateTime={isoTime}>
        {relativeTime}
      </time>
    </div>
    <p className="text-xs text-muted-foreground truncate mt-0.5">
      {lastMessage}
    </p>
  </div>

  {/* 未读徽章 */}
  {unreadCount > 0 && (
    <Badge aria-label={`${unreadCount} 条未读消息`}>
      {unreadCount}
    </Badge>
  )}
</div>
```

### 4.3 聊天气泡 + 头像规范

```tsx
// 助手消息 - 左侧头像
<div className="flex gap-2 px-4 py-2" role="article" aria-label={`${personaName} 说`}>
  <NiceAvatar
    className="w-[var(--chat-avatar-size)] h-[var(--chat-avatar-size)] shrink-0 mt-1"
    aria-hidden="true"
    {...personaAvatar}
  />
  <div className="max-w-[var(--chat-bubble-max-width)]">
    <div className="rounded-lg bg-[hsl(var(--chat-bubble-assistant))] px-3 py-2 text-sm">
      {content}
    </div>
    <time className="text-xs text-muted-foreground/50 mt-1 block">{time}</time>
  </div>
</div>

// 用户消息 - 右侧头像
<div className="flex gap-2 px-4 py-2 flex-row-reverse" role="article" aria-label="你说">
  <UserAvatar className="w-[var(--chat-avatar-size)] h-[var(--chat-avatar-size)] shrink-0 mt-1" />
  <div className="max-w-[var(--chat-bubble-max-width)]">
    <div className="rounded-lg bg-[hsl(var(--chat-bubble-user))] text-[hsl(var(--chat-bubble-user-fg))] px-3 py-2 text-sm">
      {content}
    </div>
  </div>
</div>
```

### 4.4 NiceAvatar 配置器规范

```tsx
// 交互式配置器 - 每个选项用 radio group
<fieldset>
  <legend className="text-sm font-medium mb-2">发型</legend>
  <div role="radiogroup" aria-label="选择发型" className="flex gap-2">
    {hairStyles.map(style => (
      <button
        key={style}
        role="radio"
        aria-checked={current === style}
        aria-label={style}
        onClick={() => onChange(style)}
        className={cn(
          "w-10 h-10 rounded-lg border-2 flex items-center justify-center",
          current === style ? "border-primary" : "border-border"
        )}
      >
        <NiceAvatar className="w-8 h-8" {...{...config, hairStyle: style}} />
      </button>
    ))}
  </div>
</fieldset>
```

---

## 五、实施优先级（合并后）

| 优先级 | 改动 | 工作量 | 说明 |
|--------|------|--------|------|
| P0 | 修复 `Footer.displayName` bug | 1 行 | 立即修复 |
| P0 | `Main` 改为 `<main>` 语义标签 | 1 行 | 立即修复 |
| P0 | `body` 背景色 `bg-accent` → `bg-background` | 1 行 | 立即修复 |
| P0 | 全局 `:focus-visible` 焦点环样式 | 小 | 可访问性基础 |
| P0 | `prefers-reduced-motion` 适配 | 小 | 可访问性基础 |
| P0 | 新增 CSS 变量（layout/chat/avatar/transition） | 小 | Token 基础 |
| P1 | ActivityBar → `<nav>` + `<button>` + 键盘导航 | 中 | 可访问性 |
| P1 | SessionList → ARIA roles + 键盘导航 | 中 | 可访问性 |
| P1 | 聊天区 `role="log"` + `aria-live` | 小 | 可访问性 |
| P1 | Persona 数据层（Phase 1） | 小 | 微信功能基础 |
| P1 | 微信风格会话列表（Phase 2） | 中 | 视觉改造 |
| P1 | 聊天头像（Phase 4） | 中 | 视觉改造 |
| P2 | `window.prompt/confirm` → AlertDialog | 中 | UX 一致性 |
| P2 | moment/market 颜色改为 HSL 变量 | 小 | Token 一致性 |
| P2 | 角色选择创建流程（Phase 3） | 中 | 微信功能 |
| P3 | 角色管理页面 + 配置器（Phase 5） | 大 | 微信功能 |
| P3 | 后端 system_prompt 注入（Phase 6） | 中 | 后端适配 |

---

## 六、文件改动总览

```
立即修复（P0）：
  src/index.css                          — 焦点环、reduced-motion、body 背景、新 CSS 变量
  src/components/custom/layout.tsx        — Footer.displayName bug
  src/layouts/chat/components/main.tsx    — <aside> → <main>
  tailwind.config.js                     — moment/market 颜色改为 HSL 引用

可访问性改进（P1）：
  src/layouts/chat/components/activity-bar.tsx  — <nav> + <button> + 键盘
  src/pages/agent/components/agent-session-list.tsx — ARIA roles + 键盘
  src/pages/agent/components/agent-chat.tsx     — role="log" + aria-live + 头像

微信功能（P1-P3）：
  （同 PLAN-WECHAT-UI.md 中的文件清单）
```

建议从 P0 的 6 个快速修复开始，10 分钟内就能完成，然后推进 P1 的可访问性 + 微信风格改造。
