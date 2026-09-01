# LookEveryting 组件规范

> 配套 [DESIGN_SYSTEM.md](./DESIGN_SYSTEM.md) · 定义每个 UI 组件的尺寸、状态与交互

---

## 1. 组件总览

```
AppShell
├── TitleBar
├── SideRail
│   └── FileTree / IconNav
├── MediaViewport
│   ├── ImageViewer
│   ├── VideoPlayer
│   └── ModelViewer
├── FloatingToolbar
├── PlaybackBar          (video only)
├── ModelToolbar         (3d only)
├── InfoDrawer
├── ThumbnailGrid
├── SettingsPanel
├── Toast
└── EmptyState
```

---

## 2. TitleBar

自定义标题栏，替代系统原生（Windows 可选 Mica 背景）。

### 尺寸

| 属性 | 值 |
|------|-----|
| 高度 | 40px |
| 左右内边距 | 12px |
| 拖拽区域 | 除按钮外全部 |

### 结构

```
[App Icon 16px] [文件名 label/caption] ──── spacer ──── [─ □ ✕]
```

### 状态

| 状态 | 背景 | 文件名颜色 | 按钮 |
|------|------|-----------|------|
| default | transparent / mica | text-secondary | text-tertiary |
| focused | 同 default | text-primary | text-secondary |
| fullscreen | hidden | — | — |

### 窗口按钮

| 按钮 | 尺寸 | 悬停 | 按下 |
|------|------|------|------|
| 最小化 | 40×40 | surface-raised | surface-overlay |
| 最大化 | 40×40 | surface-raised | surface-overlay |
| 关闭 | 40×40 | `#E81123` 背景 + 白字 | `#BF0A1A` |

---

## 3. IconButton

圆形/圆角图标按钮，工具栏最常用控件。

### 尺寸变体

| 变体 | 尺寸 | 图标 | 圆角 |
|------|------|------|------|
| `sm` | 28×28 | 16px | radius-md |
| `md` | 32×32 | 18px | radius-md |
| `lg` | 40×40 | 20px | radius-lg |

### 状态表

| 状态 | 背景 | 图标色 | 边框 |
|------|------|--------|------|
| default | transparent | text-secondary | none |
| hover | surface-raised | text-primary | none |
| active/pressed | surface-overlay | text-primary | none |
| selected | accent-muted | accent | none |
| disabled | transparent | text-disabled | none |
| focus | 同 hover | 同 hover | focus-ring 2px |

### 带 Tooltip

- 延迟 400ms 显示
- 背景 `surface-overlay`，文字 `caption`
- 显示快捷键：`适应窗口  0`

---

## 4. Button（文字按钮）

| 变体 | 背景 | 文字 | 边框 |
|------|------|------|------|
| primary | accent | text-inverse | none |
| primary:hover | accent-hover | text-inverse | none |
| secondary | transparent | text-primary | 1px border-default |
| secondary:hover | surface-raised | text-primary | 1px border-strong |
| ghost | transparent | text-secondary | none |
| ghost:hover | surface-raised | text-primary | none |
| danger | danger-muted | danger | none |
| danger:hover | danger | text-inverse | none |

### 尺寸

| 变体 | 高度 | 水平内边距 | 字号 |
|------|------|-----------|------|
| sm | 28px | 10px | label |
| md | 36px | 14px | label |
| lg | 44px | 18px | body |

---

## 5. FloatingToolbar

图片/通用查看模式的底部浮层工具栏。

### 布局

```
┌──────────────────────────────────────────────────────────┐
│  [◀][▶]   3/48   filename.jpg   [Fit][100%][−][+][ℹ][⋮] │
└──────────────────────────────────────────────────────────┘
     ↑ nav      ↑ status          ↑ actions
```

### 样式

| 属性 | 值 |
|------|-----|
| 高度 | 44px |
| 距底边 | 12px |
| 水平边距 | 12px（不贴边） |
| 背景 | toolbar + blur(12px) |
| 圆角 | radius-xl |
| 阴影 | elevation-md |

### 行为

| 模式 | 可见性 |
|------|--------|
| 默认 | 始终显示 |
| 沉浸 | 3s 无操作隐藏；鼠标移入底部 80px 区域显示 |
| 全屏 | 同沉浸 |

### 状态

| 状态 | opacity | transform |
|------|---------|-----------|
| visible | 1 | translateY(0) |
| hidden | 0 | translateY(8px) |
| transitioning | 动画 200ms ease-out | — |

---

## 6. PlaybackBar（视频）

继承 FloatingToolbar 样式，增加进度条。

### 布局

```
[⏮][▶/⏸][⏭]  ────●────────────  03:24 / 12:08  [🔊━━] [CC] [⛶]
                ↑ progress track (4px height, radius-full)
```

### 进度条状态

| 部分 | 默认 | 悬停 |
|------|------|------|
| track | border-subtle | border-default |
| buffered | surface-overlay | — |
| played | accent | accent-hover |
| thumb | 12px 圆，accent | 14px 圆 |

### 交互

- 点击轨道：跳转
- 拖拽 thumb：scrub（显示预览帧，后续）
- 滚轮：±5s

---

## 7. ModelToolbar（3D）

顶部浮层，水平居中。

```
[实体][线框][点云]  │  [正视][侧视][顶视]  │  [ℹ][📷]
```

- 分段控件（SegmentedControl）：选中项 `accent-muted` 背景
- 分隔线：`1px border-subtle`，高 20px

---

## 8. SideRail

### 宽度

| 模式 | 宽度 |
|------|------|
| expanded | 240px |
| collapsed | 48px（仅图标） |
| hidden | 0（Compact 断点） |

### 文件树项

| 状态 | 背景 | 文字 | 图标 |
|------|------|------|------|
| default | transparent | text-secondary | text-tertiary |
| hover | surface-raised | text-primary | text-secondary |
| selected | accent-muted | accent | accent |
| active (当前文件) | accent-muted | text-primary | accent |

- 行高：32px
- 缩进：每级 +16px
- 展开箭头：16px chevron

---

## 9. InfoDrawer

右侧滑出信息面板。

### 尺寸

| 断点 | 宽度 | 行为 |
|------|------|------|
| Spacious | 280px | 固定侧栏 |
| Comfortable | 320px | 覆盖式抽屉 |
| Compact | 100% | 全屏抽屉 |

### 分区

```
┌─ InfoDrawer ─────────────┐
│  OVERLINE: 文件信息       │
│  filename.jpg            │
│  2.4 MB · JPEG · 4032×3024│
├──────────────────────────┤
│  OVERLINE: 相机          │
│  Canon EOS R5            │
│  50mm · f/2.8 · ISO 400  │
├──────────────────────────┤
│  OVERLINE: 位置          │
│  36.06°N, 120.38°E       │
└──────────────────────────┘
```

### 动画

- 进入：`translateX(100%) → 0`，300ms ease-out
- 退出：反向，200ms ease-in
- 遮罩：Compact 模式下 `scrim` 背景

---

## 10. ThumbnailGrid

### 网格规则

```css
grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
gap: 8px;
padding: 16px;
```

### 缩略图卡片

| 状态 | 边框 | 阴影 | 缩放 |
|------|------|------|------|
| default | 1px border-subtle | none | 1.0 |
| hover | 1px border-default | elevation-sm | 1.0 |
| selected | 2px accent | elevation-sm | 1.0 |
| loading | 1px border-subtle | none | skeleton 动画 |

### 角标

| 类型 | 位置 | 样式 |
|------|------|------|
| 视频时长 | 右下 | `caption` 白字 + 半透明黑底 |
| 3D 图标 | 左上 | 16px `box` 图标 |
| 多选序号 | 左上 | accent 圆形徽章 |

---

## 11. EmptyState

首次打开 / 无文件时居中显示。

```
        [ 48px image-icon ]
        
        拖放文件到此处
        或按 Ctrl+O 打开
        
        [  打开文件  ]  ghost 按钮
```

| 元素 | 样式 |
|------|------|
| 图标 | 48px, text-tertiary |
| 主文案 | body, text-secondary |
| 副文案 | caption, text-tertiary |
| 按钮 | ghost md |

### 拖拽悬停

- 整个窗口边框：`2px dashed accent`
- 背景：`accent-subtle @ 30%`

---

## 12. Toast

右下角轻提示。

| 属性 | 值 |
|------|-----|
| 最小宽度 | 280px |
| 最大宽度 | 400px |
| 内边距 | 12px 16px |
| 圆角 | radius-lg |
| 背景 | surface-overlay |
| 阴影 | elevation-lg |
| 自动消失 | 3s（错误 5s） |

### 变体

| 类型 | 左边框 | 图标色 |
|------|--------|--------|
| info | accent 3px | accent |
| success | success 3px | success |
| warning | warning 3px | warning |
| error | danger 3px | danger |

---

## 13. SettingsPanel

模态或侧滑设置页，分组卡片布局。

### 设置项行

```
语言          [ 简体中文  ▾ ]
主题          [ ● 深色  ○ 浅色  ○ 系统 ]
界面密度      [ 紧凑  ● 标准  ○ 大号 ]
工具栏自动隐藏  [ ────●── ]  开启
```

| 元素 | 高度 | 样式 |
|------|------|------|
| 分组标题 | 32px | overline |
| 设置行 | 44px | body + 右侧控件 |
| 分隔线 | 1px | border-subtle，上下 8px margin |

---

## 14. SegmentedControl

用于 3D 视图模式、列表/网格切换。

```
┌────────┬────────┬────────┐
│  实体  │  线框  │  点云  │   ← 容器: surface-raised, radius-md, p-2px
└────────┴────────┴────────┘
     ↑ 选中: surface-overlay + text-primary
     ↑ 未选: transparent + text-secondary
```

---

## 15. 输入框（Search / Path）

| 状态 | 背景 | 边框 | 文字 |
|------|------|------|------|
| default | surface-raised | border-subtle | text-primary |
| hover | surface-raised | border-default | text-primary |
| focus | surface-raised | accent | text-primary + focus-ring |
| disabled | surface | border-subtle | text-disabled |
| error | surface-raised | danger | text-primary |

- 高度：36px
- 内边距：8px 12px
- 圆角：radius-md
- 占位符：text-tertiary

---

## 16. 组件状态通用规则

1. **悬停** 必须有视觉反馈（背景或边框变化）
2. **禁用** 不响应点击，`cursor: not-allowed`
3. **加载** 用 skeleton 或 16px spinner（accent 色）
4. **选中** 用 `accent-muted` 背景 + `accent` 前景，不单靠颜色
5. **所有过渡** 120–200ms，除面板滑入 300ms

---

## 17. Z-Index 层级

| 层级 | 值 | 内容 |
|------|-----|------|
| viewport | 0 | 主画布 |
| toolbar | 10 | 浮层工具栏 |
| sidebar | 20 | 侧栏 |
| drawer | 30 | 信息抽屉 |
| dropdown | 40 | 下拉菜单 |
| modal | 50 | 设置弹窗 |
| toast | 60 | 提示 |
| tooltip | 70 | 工具提示 |
