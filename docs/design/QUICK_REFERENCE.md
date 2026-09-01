# LookEveryting 设计规范速查

> 开发时快速查阅，完整规范见 `docs/` 目录。

## 色彩（深色极简）

| 用途 | Token | 色值 |
|------|-------|------|
| 主画布 | viewport | `#000000` |
| 应用背景 | canvas | `#0A0A0B` |
| 面板 | surface | `#111113` |
| 主文字 | text-primary | `#FAFAFA` |
| 次文字 | text-secondary | `#A1A1AA` |
| 强调 | accent | `#3B82F6` |
| 浮层工具栏 | toolbar | `#111113E6` + blur 12px |

## 间距

- 基准网格：**8px**
- 面板内边距：**16px**
- 工具栏边距：**12px**
- 圆角：按钮 6px · 浮层 12px · 缩略图 8px

## 断点

| 模式 | 宽度 | 侧栏 | 信息面板 |
|------|------|------|----------|
| Compact | < 720 | 隐藏 | 全屏抽屉 |
| Comfortable | 720–1279 | 48px 图标 | 覆盖抽屉 |
| Spacious | ≥ 1280 | 240px | 280px 侧栏 |

## 组件高度

- TitleBar: 40px
- Toolbar: 44px
- IconButton: 32×32px
- Input: 36px

## 动效

- 工具栏显隐: 200ms
- 面板滑入: 300ms
- 自动隐藏: 3s 无操作

## 文件索引

```
design/tokens/     ← JSON 单一数据源
docs/DESIGN_SYSTEM.md
docs/COMPONENTS.md
docs/LAYOUT.md
crates/cap-ui/     ← Rust Token 实现
locales/           ← FTL 翻译
```
