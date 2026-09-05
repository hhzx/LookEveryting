# 文档中心

LookEveryting 项目文档索引。按用途分类如下。

## 快速上手

| 文档 | 说明 |
|------|------|
| [../README.md](../README.md) | 项目概览、功能、快速开始 |
| [build.md](./build.md) | 构建、测试、打包发布 |
| [architecture.md](./architecture.md) | 技术架构与模块说明 |

## 设计规范（深色极简 v1.0）

| 文档 | 说明 |
|------|------|
| [design/DESIGN_SYSTEM.md](./design/DESIGN_SYSTEM.md) | 色彩、字体、间距、阴影、动效 |
| [design/COMPONENTS.md](./design/COMPONENTS.md) | 组件状态与交互规范 |
| [design/LAYOUT.md](./design/LAYOUT.md) | 响应式布局、断点、DPI |
| [design/QUICK_REFERENCE.md](./design/QUICK_REFERENCE.md) | 开发速查表 |

## 资源文件

| 路径 | 说明 |
|------|------|
| [../design/tokens/](../design/tokens/) | JSON Design Token（机器可读） |
| [../locales/](../locales/) | Fluent 翻译文件（en-US / zh-Hans） |
| [../crates/cap-ui/src/](../crates/cap-ui/src/) | Rust Token 实现 |

## 路线图

| 文档 | 说明 |
|------|------|
| [roadmap.md](./roadmap.md) | 版本规划与待办 |
| [LEARNING_PLAN.md](./LEARNING_PLAN.md) | 开源竞品学习整合计划 |
| [EXPERIENCE_PLAN.md](./EXPERIENCE_PLAN.md) | 极致体验专项 |
| [FORMATS.md](./FORMATS.md) | 支持的文件格式 |
| [INSPIRATION.md](./INSPIRATION.md) | 竞品摘要 |

## 文档维护约定

1. **设计变更** → 先改 `design/tokens/*.json`，再同步 `crates/cap-ui` 与 `design/*.md`
2. **文案变更** → 改 `locales/*.ftl` 与 `crates/cap-i18n`
3. **功能变更** → 更新 `architecture.md` 与根 `README.md`
