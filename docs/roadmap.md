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

## v0.6 — 画质与性能（进行中）

- [ ] wgpu mesh PaintCallback（UI 已跑在 wgpu；网格仍为高质量软件光栅）
- [x] MF 硬件变换偏好开关（`prefer_hw_decode`；完整 DXVA/D3D11 设备管理仍待）
- [ ] 音频轨 WASAPI 播放（音量 UI 已就绪）
- [x] 邻居预取 ±2
- [x] 侧车 SRT 字幕叠加（设置 / `V` 开关）

## v0.7 — 专业能力

- [ ] ASS / 多音轨 / 内嵌字幕轨
- [ ] 材质贴图预览
- [x] 轻量批量重命名
- [ ] RAW 预览（可选）
- [ ] 视频/3D 缩略图真实首帧

## v1.0 — 产品化

- [x] 便携 zip（`scripts/build.ps1`）
- [ ] MSI 安装包
- [x] 主题跟随系统
- [x] 格式支持表 `docs/FORMATS.md`
- [ ] 自动更新（可选）
