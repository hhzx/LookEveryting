# LookEveryting — 竞品学习与优化路线

参考优秀开源看图 / 看视频 / 看模型软件，提炼可落地的改进点。

## 图片查看（参考 qView、ImageGlass、nomacs）

| 项目 | 优秀点 | LookEveryting 状态 |
|------|--------|-------------------|
| [qView](https://github.com/jurplel/qView) | 极速启动、无边框沉浸、拖拽即开 | 部分：拖拽已支持 |
| [ImageGlass](https://github.com/d2phap/ImageGlass) | 棋盘格透明底、EXIF 侧栏、快捷键丰富 | **v0.3 已加棋盘格** |
| [nomacs](https://github.com/nomacs/nomacs) | 同步浏览、批量重命名、RAW 支持 | 待规划 |

**近期可做：**
- 全屏模式（F11 / 双击标题栏）
- EXIF / 色彩空间信息面板
- 鼠标滚轮以指针为中心缩放（已实现）
- 幻灯片放映（文件夹自动播放）

## 视频播放（参考 mpv、VLC、SMPlayer）

| 项目 | 优秀点 | LookEveryting 状态 |
|------|--------|-------------------|
| [mpv](https://github.com/mpv-player/mpv) | 硬解、精确 seek、空格播放/暂停 | **v0.3 空格播放/暂停** |
| [VLC](https://github.com/videolan/vlc) | 格式全、音轨/字幕切换 | 待规划 |
| [Celluloid](https://github.com/celluloid-player/celluloid) | mpv 的简洁 GTK 壳 | UI 参考 |

**近期可做：**
- 进度条 + 拖动 seek
- 音量与静音
- 硬件解码（DXVA / D3D11VA）
- 音轨同步（当前仅视频轨）

## 3D 模型（参考 F3D、Open 3D Viewer、assimpView）

| 项目 | 优秀点 | LookEveryting 状态 |
|------|--------|-------------------|
| [F3D](https://github.com/f3d-app/f3d) | 基于 VTK，格式多、光照好 | 参考其 PBR 光照 |
| [Open3DViewer](https://github.com/open3dview/open3dview) | 轻量、轨道相机 | 已实现基础轨道相机 |
| [assimp](https://github.com/assimp/assimp) | 统一加载器 | 已用 tobj/stl_io/gltf/ufbx |

**近期可做：**
- wgpu 真 3D 管线（替代软件光栅）
- 线框 / 实体切换（已有 wireframe 字段）
- 环境光 + 多方向光
- 材质与贴图预览

## UI / 体验（跨类别）

1. **状态栏**：分辨率、缩放比、文件序号 — 参考 ImageGlass 底栏
2. **键盘**：方向键切换文件 — **v0.3 已支持上下左右**
3. **字体**：便携包内置 CJK 字体 — **v0.3 build 脚本已打包 fonts/**
4. **性能**：视频首帧 < 500ms、大图分块加载
5. **主题**：跟随系统浅/深色

## 版本规划

- **v0.3**（当前）：视频 MF 修复、字体打包、导航快捷键、棋盘格
- **v0.4**：视频进度条、全屏、EXIF 面板
- **v0.5**：wgpu 3D 渲染、材质贴图
- **v1.0**：文件关联稳定、安装包、自动更新
