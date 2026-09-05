# LookEveryting — 竞品学习与优化路线

> 完整学习整合计划见：**[LEARNING_PLAN.md](./LEARNING_PLAN.md)**（选型、差距、分阶段任务）。  
> **极致体验专项见：[EXPERIENCE_PLAN.md](./EXPERIENCE_PLAN.md)**（体感指标、交互宪法、14 日冲刺）。

参考优秀开源看图 / 看视频 / 看模型软件，提炼可落地的改进点。

## 主参考（结论）

| 品类 | 主学 | 辅学 |
|------|------|------|
| 图片 | **qView**（速度）+ **ImageGlass**（体验） | qimgv、nomacs |
| 视频 | **mpv**（解码/seek）+ **Celluloid**（简洁壳） | VLC（兼容兜底） |
| 3D | **F3D**（默认好看 + 交互） | glTF Sample Viewer、ModelViewer-Qt |

## 图片查看（参考 qView、ImageGlass、nomacs）

| 项目 | 优秀点 | LookEveryting 状态 |
|------|--------|-------------------|
| [qView](https://github.com/jurplel/qView) | 极速启动、无边框沉浸、拖拽即开、预加载 | 部分：拖拽、预览优先、邻居预取 |
| [ImageGlass](https://github.com/d2phap/ImageGlass) | 棋盘格、EXIF、智能缓存、幻灯片、Window Fit | 棋盘格 / 全屏 / 状态栏已有；EXIF、幻灯片待做 |
| [nomacs](https://github.com/nomacs/nomacs) | 同步浏览、批量重命名、RAW | 延后到 v0.7+ |
| [qimgv](https://github.com/easymodo/qimgv) | 键盘优先、EXIF 随选更新 | 快捷键已较强；EXIF 待做 |

**近期可做（Phase A）：**
- EXIF / 色彩空间信息面板
- 幻灯片放映（文件夹自动播放）
- GIF / 动画 WebP
- Window Fit（窗口跟随图片）
- 缩放插值：适应 Linear / 100% Nearest

## 视频播放（参考 mpv、VLC、Celluloid）

| 项目 | 优秀点 | LookEveryting 状态 |
|------|--------|-------------------|
| [mpv](https://github.com/mpv-player/mpv) | 硬解、精确 seek、空格播放/暂停 | 空格 / 进度条 seek / 逐帧已有；硬解与音频待做 |
| [Celluloid](https://github.com/celluloid-player/celluloid) | mpv 简洁 GUI 壳 | UI 密度参考 |
| [VLC](https://www.videolan.org/) | 格式全、音轨/字幕、失败兜底 | 外抛系统播放器兜底待做 |

**近期可做：**
- 音量与静音
- 硬件解码（DXVA / D3D11VA）探测与开关
- 精确帧 seek、±5s 跳转
- 打开失败 → 系统播放器
- （可选后端）libmpv 兼容模式

## 3D 模型（参考 F3D、Open 3D Viewer、assimpView）

| 项目 | 优秀点 | LookEveryting 状态 |
|------|--------|-------------------|
| [F3D](https://github.com/f3d-app/f3d) | 默认好看、PBR、快捷键、缩略图 | 轨道相机 / 线框 / 双灯已有；wgpu+PBR 待做 |
| ModelViewer-Qt / glTF Sample Viewer | PBR / IBL / 材质 | 学习管线，不引入 VTK |
| [assimp](https://github.com/assimp/assimp) | 统一加载器 | 现用 tobj/stl_io/gltf/ufbx（控体积） |

**近期可做：**
- 模型 HUD（顶点/三角面）
- wgpu 真 3D 管线（替代软件光栅）— **v0.6 核心**
- 基础 PBR + 贴图
- 重置视角、更好默认背景

## UI / 体验（跨类别）

1. **状态栏**：分辨率、缩放比、文件序号 — **已有**
2. **键盘**：方向键切换 — **已有**
3. **字体**：便携包内置 CJK — **已有**
4. **缩略图条滚轮横向滚动** — **已有**
5. **性能**：视频首帧 < 500ms、大图分块 — 进行中
6. **主题**：跟随系统浅/深色 — 待做
7. **快捷键帮助 `?`** — 待做

## 版本规划

| 版本 | 主题 | 对标 |
|------|------|------|
| **v0.4**（当前） | seek、全屏、原图按需、状态栏、缩略图滚轮 | ImageGlass / mpv 基础 |
| **v0.5** | EXIF、幻灯片、GIF、音量、3D HUD | qView + Celluloid + F3D |
| **v0.6** | wgpu 3D、硬解、音频、PBR | mpv + F3D |
| **v0.7** | 字幕、材质贴图、轻量批量 | VLC + nomacs |
| **v1.0** | 安装包、关联、文档、主题 | 成熟桌面产品 |

下一步优先 8 项与详细验收见 [LEARNING_PLAN.md](./LEARNING_PLAN.md)。
