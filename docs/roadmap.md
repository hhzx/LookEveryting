# 路线图

> 详细计划：[`LEARNING_PLAN.md`](./LEARNING_PLAN.md) · 极致体验：[`EXPERIENCE_PLAN.md`](./EXPERIENCE_PLAN.md) · 格式表：[`FORMATS.md`](./FORMATS.md)

## v0.1–v0.5 — 已完成

- [x] 深色极简设计系统、中英切换、文件关联
- [x] 图片：预览优先、原图按需、缩放/平移、棋盘格、filmstrip 滚轮
- [x] 视频：MF 内嵌、进度条 seek、逐帧、±5s、seek 气泡
- [x] 3D：轨道相机、线框/实体、渐变背景、HUD、`R` 重置
- [x] 全屏 F11、状态栏、加载动画、CJK 字体打包
- [x] EXIF、幻灯片、零黑场、工具栏热区、快捷键 `?`、错误可行动
- [x] GIF 动画、Window Fit、Fit/100% 补间、100% Nearest
- [x] 主题 Dark / Light / System
- [x] 批量重命名（F2）、便携 zip 打包脚本

## v0.6 — 画质与性能

- [x] wgpu mesh PaintCallback（实体着色 + 深度；线框仍走 CPU）
- [x] MF DXVA/D3D11 硬解（`prefer_hw_decode` + DXGI device manager）
- [x] 音频轨 WASAPI/cpal 播放（与视频时钟软同步；音量/静音生效）
- [x] 邻居预取 ±2
- [x] 侧车 SRT 字幕叠加（设置 / `V` 开关）
- [x] 播放速度 0.5x–2.0x（`[` `]` / 工具栏按钮）

## v0.7 — 专业能力

- [x] 视频/3D 缩略图真实首帧
- [x] A-B 循环
- [x] ASS/SSA 侧车字幕 + 多音轨切换（`T`）
- [x] glTF PBR-lite（base color + albedo 贴图 + metallic/roughness）
- [x] 轻量批量重命名
- [x] RAW 预览（crude demosaic）
- [x] 图片旋转 / 翻转（Ctrl+R / H / Shift+H）

## v1.0 — 产品化

- [x] 便携 zip（`scripts/build.ps1`）
- [x] 每用户安装脚本（`scripts/install.ps1`）
- [x] WiX MSI 脚本（`scripts/build-msi.ps1` + `packaging/LookEveryting.wxs`，需本机 WiX）
- [x] 主题跟随系统
- [x] 格式支持表 `docs/FORMATS.md`
- [x] 更新检查/下载（`scripts/check-update.ps1 -Download`）
- [ ] MF 内嵌字幕轨（可选）
- [ ] 完整 IBL / normal map（可选进阶）
