# 构建与发布

## 环境要求

- **Rust** stable（1.75+，推荐通过 [rustup](https://rustup.rs/) 安装）
- **Windows 10/11** x64（当前主要目标平台）
- GPU 支持 WebGPU / DX12（eframe wgpu 后端）

## 一键构建

```powershell
.\scripts\build.ps1
```

脚本依次执行：

1. `cargo test --workspace` — 运行全部单元测试
2. `cargo build --release -p look-everyting` — Release 编译
3. 打包到 `dist/LookEveryting/`

## 手动命令

```powershell
# 测试
cargo test --workspace

# 调试运行
cargo run -p look-everyting

# Release 编译
cargo build --release -p look-everyting

# 可执行文件位置
.\target\release\LookEveryting.exe
```

## 发布包内容

```
dist/LookEveryting/
├── LookEveryting.exe   # 主程序（约 7–8 MB）
├── locales/            # 翻译资源
├── design/             # Design Token
└── README.md
```

## 测试覆盖

当前 **11 项** 单元测试：

| Crate | 测试内容 |
|-------|----------|
| cap-core | 文件类型分类、设置序列化 |
| cap-i18n | 中英文翻译、key 回退 |
| cap-image | PNG 解码、格式校验 |
| cap-model | GLTF/OBJ 元信息 |
| cap-video | MP4 元信息 |
| cap-ui | 主题密度缩放 |
| app | 图片打开与文件夹索引 |

## 快捷键

| 按键 | 功能 |
|------|------|
| `Ctrl+O` | 打开文件 |
| `←` / `→` | 上一张 / 下一张 |
| `0` | 适应窗口 |
| `1` | 实际大小 (100%) |
| `I` | 信息面板 |
| `Ctrl+,` | 设置 |

## 支持格式

**图片**：jpg, jpeg, png, gif, webp, bmp, tif, tiff, ico, avif

**视频**：mp4, m4v, mov, mkv, webm, avi, wmv, flv, mpg, mpeg

**3D 模型**：glb, gltf, obj, stl, fbx, ply, dae, 3mf

## 已知限制（v0.1）

- 视频：显示元信息，播放需调用系统默认播放器
- 3D：GLB/GLTF 可解析统计信息，实时渲染待实现
- 大图：尚未集成瓦片渲染（计划接入 Bloom 引擎）
