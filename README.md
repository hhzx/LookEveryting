# LookEveryting

本地全能媒体查看器 — 看图、看视频、看 3D 模型。深色极简、零上传、启动快。

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## 功能

| 类型 | 支持 |
|------|------|
| 图片 | PNG / JPEG / GIF / WebP / BMP / TIFF / AVIF — 缩放、适应窗口、文件夹浏览 |
| 视频 | MP4 / MKV / MOV 等 — 元信息展示，一键系统播放器打开 |
| 3D 模型 | GLB / GLTF / OBJ / STL 等 — 面数/顶点统计，系统查看器打开 |

- 深色极简 UI，中英文切换
- 响应式三栏布局（Compact / Comfortable / Spacious）
- 单文件 ~7.4 MB，无需安装运行时

## 快速开始

```powershell
# 构建 + 测试 + 打包
.\scripts\build.ps1

# 运行
.\dist\LookEveryting\LookEveryting.exe
```

需要 [Rust stable](https://rustup.rs/) 和 Windows 10/11 x64。

## 快捷键

| 按键 | 功能 |
|------|------|
| `Ctrl+O` | 打开文件 |
| `←` `→` | 上一张 / 下一张 |
| `0` | 适应窗口 |
| `1` | 实际大小 |
| `I` | 信息面板 |

## 项目结构

```
LookEveryting/
├── app/                    # 主程序 (eframe)
├── crates/
│   ├── cap-core/           # 文件路由、配置
│   ├── cap-ui/             # 设计 Token、组件
│   ├── cap-i18n/           # 国际化
│   ├── cap-image/          # 图片解码
│   ├── cap-model/          # 3D 元信息
│   └── cap-video/          # 视频元信息
├── design/tokens/          # JSON Design Token
├── docs/                   # 文档（见 docs/README.md）
├── locales/                # 翻译文件
└── scripts/build.ps1       # 构建脚本
```

## 文档

完整文档索引：[docs/README.md](docs/README.md)

| 文档 | 说明 |
|------|------|
| [构建与发布](docs/build.md) | 编译、测试、打包 |
| [技术架构](docs/architecture.md) | 模块设计与数据流 |
| [设计系统](docs/design/DESIGN_SYSTEM.md) | 色彩、字体、间距规范 |
| [路线图](docs/roadmap.md) | 版本规划 |

## 技术栈

Rust · eframe · egui · wgpu · image · gltf

## 测试

```powershell
cargo test --workspace   # 11 tests
```

## 许可证

[MIT](LICENSE)
