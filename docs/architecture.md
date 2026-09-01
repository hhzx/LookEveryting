# 技术架构

## 总览

LookEveryting 是基于 **Rust + eframe + egui + wgpu** 的本地桌面媒体查看器。所有文件在本地打开，不上传云端。

```
┌─────────────────────────────────────────────────┐
│  app (LookEveryting.exe)                        │
│  eframe 窗口 + egui UI 壳层                      │
├──────────────┬──────────────┬───────────────────┤
│  cap-image   │  cap-video   │  cap-model        │
│  图片解码     │  视频元信息   │  3D 元信息         │
├──────────────┴──────────────┴───────────────────┤
│  cap-ui (Design Token + 组件)                    │
│  cap-i18n (中英文)                               │
│  cap-core (文件路由、设置持久化)                  │
└─────────────────────────────────────────────────┘
```

## Crate 职责

| Crate | 路径 | 职责 |
|-------|------|------|
| `look-everyting` | `app/` | 主程序入口、应用状态、UI 编排 |
| `cap-core` | `crates/cap-core/` | 媒体类型分类、设置读写 |
| `cap-ui` | `crates/cap-ui/` | 设计 Token、主题、基础组件 |
| `cap-i18n` | `crates/cap-i18n/` | 运行时国际化 |
| `cap-image` | `crates/cap-image/` | 图片解码为 RGBA |
| `cap-video` | `crates/cap-video/` | 视频文件元信息 |
| `cap-model` | `crates/cap-model/` | 3D 模型元信息（GLTF 解析） |

## 数据流

### 打开文件

```
用户选择文件
    → cap-core::classify_extension()
    → 分支到 cap-image / cap-video / cap-model
    → 更新 LookApp 状态
    → egui 渲染对应视图
```

### 图片渲染

```
磁盘文件 → cap-image::DecodedImage
        → egui::ColorImage
        → TextureHandle (GPU)
        → egui::Image 显示
```

### 设置持久化

```
%APPDATA%/LookEveryting/settings.toml
```

## 技术选型理由

| 选择 | 原因 |
|------|------|
| Rust | 性能、内存安全、单二进制分发 |
| eframe + wgpu | 轻量（~7MB），GPU 加速，无 Chromium |
| egui | 即时模式 UI，适合工具栏/面板 |
| 自研 Token | 深色极简设计系统化，避免魔法数字 |

## 依赖概览

- `eframe` / `egui` — UI 框架
- `image` — 图片解码
- `gltf` — GLTF/GLB 解析
- `rfd` — 原生文件对话框
- `walkdir` — 文件夹遍历
- `open` — 调用系统默认程序
- `serde` + `toml` — 配置序列化

## 设计 Token 同步

```
design/tokens/*.json  →  docs/design/*.md  →  crates/cap-ui/src/*.rs
         ↑                      ↑                      ↑
    机器可读源            人类可读规范              运行时实现
```

## 性能目标

| 指标 | v0.1 现状 | 目标 |
|------|-----------|------|
| 安装包 | ~7.4 MB | < 25 MB（含视频/3D） |
| 冷启动 | < 1s | < 500ms |
| 测试 | 11 passed | 持续扩展 |
