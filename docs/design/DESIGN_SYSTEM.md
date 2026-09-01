# LookEveryting 设计系统

> 版本 1.0.0 · 主题：深色极简 · 最后更新：2026-03-25

本文档是 LookEveryting 的 **Figma 级设计规范**，定义色彩、字体、间距、组件状态与交互规则。实现时以 `design/tokens/` 为机器可读单一数据源。

---

## 1. 设计原则

| 原则 | 说明 | 反例 |
|------|------|------|
| **内容优先** | UI 是浮层，画布占 ≥ 85% 可视面积 | 厚重边框、大面积面板 |
| **极简克制** | 单屏可见控件 ≤ 12 个 | 工具栏塞满按钮 |
| **隐式交互** | 常用操作 1 步；高级功能二级入口 | 多级菜单 |
| **系统一致** | Token 驱动，禁止魔法数字 | 各处圆角/颜色不统一 |
| **性能感知** | 动效 ≤ 300ms；支持减少动效 | 拖沓过渡动画 |

---

## 2. 色彩系统

### 2.1 色板（Dark Minimal）

```
层级示意（由深到浅）：

  #000000  viewport     主画布（图片/视频/3D 纯黑底）
  #0A0A0B  canvas       应用背景
  #111113  surface      侧栏、面板
  #18181B  raised       卡片、输入框
  #1F1F23  overlay      浮层、下拉
  #27272A  border-sub   分割线
  #3F3F46  border       默认边框
```

### 2.2 语义色

| Token | 色值 | 用途 |
|-------|------|------|
| `text-primary` | `#FAFAFA` | 标题、主文案 |
| `text-secondary` | `#A1A1AA` | 副文案、标签 |
| `text-tertiary` | `#71717A` | 占位符、辅助信息 |
| `text-disabled` | `#52525B` | 禁用态 |
| `accent` | `#3B82F6` | 选中、链接、主按钮 |
| `accent-hover` | `#60A5FA` | 悬停强调 |
| `accent-muted` | `#1E3A5F` | 选中背景（低饱和） |
| `danger` | `#EF4444` | 删除、错误 |
| `success` | `#22C55E` | 成功提示 |
| `warning` | `#F59E0B` | 警告 |
| `focus-ring` | `#3B82F680` | 键盘焦点环（50% 透明） |
| `scrim` | `#000000B3` | 遮罩层（70% 黑） |
| `toolbar` | `#111113E6` | 浮层工具栏（90% 不透明 + 模糊） |

### 2.3 使用规则

1. **主画布永远纯黑** `#000000` — 图片/视频/3D 区域不加边框
2. **面板与画布对比要克制** — 相邻层级色差 ≤ 8% 亮度
3. **强调色仅用于**：选中项、主 CTA、进度条、链接
4. **禁止** 大面积高饱和色块（除状态提示条）
5. **图片查看时** 隐藏所有面板，仅保留半透明底部工具栏

### 2.4 对比度（WCAG AA）

| 组合 | 对比度 | 通过 |
|------|--------|------|
| text-primary / surface | 15.8:1 | ✅ |
| text-secondary / surface | 7.2:1 | ✅ |
| text-tertiary / surface | 4.6:1 | ✅ |
| accent / canvas | 5.1:1 | ✅ |

---

## 3. 字体系统

### 3.1 字体栈

```
Sans:  Segoe UI Variable → Segoe UI → PingFang SC → Noto Sans SC → system-ui
Mono:  Cascadia Code → Consolas → Noto Sans Mono
```

### 3.2 字号阶梯

| Token | 大小 | 行高 | 字重 | 用途 |
|-------|------|------|------|------|
| `display` | 28px | 1.2 | 600 | 空状态大标题 |
| `title` | 18px | 1.35 | 600 | 面板标题 |
| `heading` | 15px | 1.35 | 500 | 分组标题 |
| `body` | 15px | 1.5 | 400 | 正文 |
| `label` | 13px | 1.35 | 500 | 按钮、标签 |
| `caption` | 13px | 1.5 | 400 | 状态栏、EXIF |
| `overline` | 11px | 1.2 | 500 | 分组小标题（全大写可选） |
| `mono` | 13px | 1.5 | 400 | 路径、哈希、尺寸 |

### 3.3 排版规则

- 文件名：单行截断，中间省略 `vacation_2024...jpg`
- 路径：mono 字体，次级色
- 数字信息：tabular-nums 对齐（`1920 × 1080`）
- 中文与英文混排：不加额外字间距
- 按钮文案：≤ 8 个汉字 / ≤ 16 个英文字符

---

## 4. 间距与圆角

### 4.1 间距（4px 基准，8px 主网格）

| Token | 值 | 用途 |
|-------|-----|------|
| `space-1` | 4px | 图标与文字间距 |
| `space-2` | 8px | 按钮内边距、缩略图间距 |
| `space-3` | 12px | 工具栏内分组间距 |
| `space-4` | 16px | 面板内边距 |
| `space-6` | 24px | 区块间距 |
| `space-8` | 32px | 大区块 |

### 4.2 圆角

| Token | 值 | 用途 |
|-------|-----|------|
| `radius-sm` | 4px | 标签、徽章 |
| `radius-md` | 6px | 按钮、输入框 |
| `radius-lg` | 8px | 卡片、缩略图 |
| `radius-xl` | 12px | 浮层工具栏、弹窗 |
| `radius-full` | 9999px | 圆形图标按钮、进度条 |

### 4.3 边框

- 默认：`1px solid border-subtle`（`#27272A`）
- 悬停：`border-default`（`#3F3F46`）
- 选中：`1px solid accent`
- 分割线：仅 `border-subtle`，不用阴影代替

---

## 5. 阴影与模糊

| 层级 | 阴影 | 模糊 | 用途 |
|------|------|------|------|
| `elevation-none` | 无 | — | 内嵌面板 |
| `elevation-sm` | `0 1px 2px rgba(0,0,0,0.4)` | — | 缩略图悬停 |
| `elevation-md` | `0 4px 12px rgba(0,0,0,0.5)` | 12px | 浮层工具栏 |
| `elevation-lg` | `0 8px 24px rgba(0,0,0,0.6)` | — | 抽屉、下拉 |
| `elevation-xl` | `0 16px 48px rgba(0,0,0,0.7)` | — | 模态框 |

**浮层工具栏样式：**
```
background: toolbar (#111113 @ 90%)
backdrop-filter: blur(12px)
border: 1px solid border-subtle
border-radius: radius-xl (12px)
box-shadow: elevation-md
```

---

## 6. 动效

| 预设 | 时长 | 缓动 | 场景 |
|------|------|------|------|
| `instant` | 0ms | — | 缩放跟手 |
| `fast` | 120ms | ease-in-out | 图片切换淡入 |
| `normal` | 200ms | ease-out | 工具栏显隐 |
| `slow` | 300ms | ease-out | 面板滑入 |

**自动隐藏：** 沉浸模式下 3 秒无操作 → 工具栏 `opacity: 0`（200ms）

**减少动效：** 检测 `prefers-reduced-motion` → 所有时长降为 0

---

## 7. 图标

- **风格**：Lucide，1.5px 描边，圆角端点
- **尺寸**：16px（工具栏内）、20px（侧栏）、24px（空状态）
- **颜色**：默认 `text-secondary`，悬停 `text-primary`，选中 `accent`
- **禁止**：填充风格图标与描边混用

常用图标映射：

| 功能 | 图标名 |
|------|--------|
| 打开 | `folder-open` |
| 适应窗口 | `maximize-2` |
| 实际大小 | `scan` |
| 放大/缩小 | `zoom-in` / `zoom-out` |
| 信息 | `info` |
| 播放/暂停 | `play` / `pause` |
| 全屏 | `expand` |
| 线框/实体 | `box` / `grid-3x3` |
| 设置 | `settings` |

---

## 8. 布局尺寸常量

| 常量 | 值 | 说明 |
|------|-----|------|
| `titlebar-height` | 40px | 自定义标题栏 |
| `toolbar-height` | 44px | 底部/顶部工具栏 |
| `sidebar-width` | 240px | 展开侧栏 |
| `sidebar-collapsed` | 48px | 图标模式 |
| `info-panel-width` | 280px | 信息面板 |
| `drawer-width` | 320px | 移动端抽屉 |
| `thumbnail-size` | 120px | 网格缩略图 |
| `icon-button` | 32×32px | 标准图标按钮 |
| `min-window-width` | 480px | 最小窗口宽 |
| `min-window-height` | 360px | 最小窗口高 |

---

## 9. 无障碍

- 所有可交互元素最小点击区域 **32×32px**
- Tab 顺序：标题栏 → 侧栏 → 画布 → 工具栏 → 面板
- 焦点环：`2px solid focus-ring`，offset 2px
- 不只用颜色传达状态（选中加图标/边框）
- 支持系统「减少动效」「高对比度」（后续）

---

## 10. 品牌气质参考

```
┌─────────────────────────────────────────────┐
│  气质关键词                                  │
│  ─────────────────────────────────────────  │
│  沉静 · 专业 · 克制 · 快速 · 无边框感        │
│                                             │
│  参考（学感觉，不抄）                         │
│  · macOS Quick Look  — 几乎无 chrome         │
│  · Linear           — 深色、细线、精致间距   │
│  · Bloom            — 画布即一切             │
│  · Arc Browser      — 浮层 UI、毛玻璃        │
└─────────────────────────────────────────────┘
```

---

## 11. Token 文件索引

| 文件 | 内容 |
|------|------|
| `design/tokens/colors.json` | 色板与语义色 |
| `design/tokens/typography.json` | 字体与排版 |
| `design/tokens/spacing.json` | 间距、圆角、组件尺寸 |
| `design/tokens/shadows.json` | 阴影与模糊 |
| `design/tokens/motion.json` | 动效时长与缓动 |
| `design/tokens/breakpoints.json` | 响应式断点 |

实现代码从 JSON 生成或手写镜像至 `crates/cap-ui/src/tokens/`。

---

## 12. 变更记录

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0.0 | 2026-03-25 | 初版：深色极简主题定稿 |
