# LookEveryting — 开源竞品学习与整合优化计划

> 目标：从「最好的开源看图 / 看视频 / 看模型软件」中抽取可复用体验与架构，落地到 LookEveryting，而不是做成功能堆叠的大杂烩。  
> 原则：**极速打开、统一浏览、一种交互语言、本地优先、体积可控**。  
> **极致体验专项（体感/手感/沉浸/验收指标）见：[EXPERIENCE_PLAN.md](./EXPERIENCE_PLAN.md)** — 与功能计划解耦，体验回归不可带进 Release。

---

## 1. 选型结论（学谁、不学谁）

### 1.1 图片（核心体验）

| 排序 | 项目 | 许可证 | 为什么学 | 不原样抄什么 |
|------|------|--------|----------|--------------|
| ★★★ | [qView](https://github.com/jurplel/qView) | GPL-3.0 | 启动极快、无边框沉浸、预加载邻居、动画 GIF 可控 | 不做成“只有图”的单用途工具 |
| ★★★ | [ImageGlass](https://github.com/d2phap/ImageGlass) | MIT | 现代 UI、棋盘格/全屏/Window Fit、智能缓存、EXIF、幻灯片、触控 | 不引入插件生态与重型编辑工具链 |
| ★★☆ | [qimgv](https://github.com/easymodo/qimgv) | GPL-3.0 | 键盘优先、EXIF 随选更新、视频缩略图条思路可参考 | 不做成 Qt 大而全媒体库 |
| ★★☆ | [nomacs](https://github.com/nomacs/nomacs) | GPL | 多窗同步对比、批量重命名、RAW | 暂不做多窗同步（成本高） |
| ★☆☆ | JPEGView / Honeyview | — | 极致速度、归档内浏览 | 仅作性能对照 |

**主参考：qView（速度）+ ImageGlass（体验与信息架构）**

### 1.2 视频（播放质量）

| 排序 | 项目 | 为什么学 | 不原样抄什么 |
|------|------|----------|--------------|
| ★★★ | [mpv](https://github.com/mpv-player/mpv) | 硬解、精确 seek、低开销、键盘驱动、OSC 悬停控件 | 不做成命令行配置地狱 |
| ★★★ | [Celluloid](https://github.com/celluloid-player/celluloid) | mpv 的简洁壳：GUI + 极简控件 | UI 布局可借鉴 |
| ★★☆ | [VLC](https://www.videolan.org/) | 格式兼容、音轨/字幕/失败兜底 | 不引入完整 VLC 体积与 UI |
| ★☆☆ | MPC-HC / MPC-BE | Windows 硬解与低延迟 | 仅作 Windows 行为参考 |

**主参考：mpv（解码与 seek 语义）+ Celluloid（控件密度）**  
**落地策略：Windows 继续 MF；行为对齐 mpv（硬解优先、精确 seek、空格/方向键），失败时再考虑 libmpv 可选后端。**

### 1.3 3D 模型（预览质量）

| 排序 | 项目 | 为什么学 | 不原样抄什么 |
|------|------|----------|--------------|
| ★★★ | [F3D](https://github.com/f3d-app/f3d) | 极简默认好看、PBR、轨道相机、缩略图、快捷键、拖放 | 不引入 VTK 全家桶 |
| ★★☆ | [ModelViewer-Qt](https://github.com/sharjith/ModelViewer-Qt) | PBR / IBL / 材质贴图管线参考 | 不照搬 CAD(STEP) 全栈 |
| ★★☆ | Assimp / glTF Sample Viewer | 格式与材质标准 | Assimp 体积大时优先保留现有 tobj/ufbx/gltf |
| ★☆☆ | Open3D Viewer | 点云/科学可视化 | 非当前产品定位 |

**主参考：F3D（默认观感与交互）+ glTF PBR（材质正确性）**  
**落地策略：用 wgpu 自研轻量管线，行为对齐 F3D，不依赖 VTK。**

---

## 2. 跨产品共性（必须对齐的“产品宪法”）

从 qView / ImageGlass / mpv / F3D 共同提炼：

1. **打开 < 200ms 感知**：先预览再精修（我们已经在做）。
2. **文件夹即相册**：←→ 切文件、底栏 filmstrip、当前项自动滚入视野。
3. **一种快捷键语言**：空格=播放/暂停或幻灯；`1`=100%；`F`/`0`=适应；`F11`=全屏。
4. **信息可查但不抢戏**：EXIF / 时长 / 面数放在侧栏或状态栏。
5. **失败可解释**：视频缺编解码器、模型无网格，要给出可操作提示。
6. **默认好看**：棋盘格、PBR 默认灯、视频黑底、工具栏可隐藏。

---

## 3. LookEveryting 现状差距（相对竞品）

| 领域 | 已有 | 明显差距 |
|------|------|----------|
| 图片 | 预览优先、4096 上限、100% 按需、缩放/平移、棋盘格、缩略图滚轮 | EXIF、GIF/WebP 动画、幻灯片、RAW、Window Fit、更锐利插值 |
| 视频 | MF 内嵌、进度条 seek、逐帧、全局解码线程 | 音频、硬解开关、字幕、音量、精确帧 seek、更多格式兜底 |
| 3D | 软件光栅、轨道相机、线框、包围盒适配 | wgpu、PBR、贴图、IBL、网格统计 HUD、大模型 LOD |
| 体验 | 全屏、状态栏、加载动画、文件关联 | 无边框沉浸、触控手势、主题、安装包、自动更新 |

---

## 4. 整合原则（避免做成“四不像”）

1. **统一入口**：同一窗口切图/视频/模型，不拆三个 App。
2. **学行为，不抄 UI 皮肤**：快捷键与信息密度学竞品，视觉保持 LookEveryting 自身设计语言。
3. **能力分层**：核心路径自研（egui + wgpu + MF）；可选能力外挂（系统播放器 / 外部 3D 工具）。
4. **体积预算**：Release 目标继续压在约 **10MB 级**（不含字体）；重库（VTK/Assimp 全量）默认不进主包。
5. **Windows 优先做深**，再谈跨平台。

---

## 5. 分阶段实施计划

### Phase A — 体验对齐（约 1–2 周）· 对标 qView + ImageGlass + Celluloid

**图片**
- [x] EXIF / 基础元数据面板（尺寸、色彩空间、拍摄时间、方向）
- [x] 幻灯片放映（可调间隔、空格暂停）
- [x] GIF 动画播放与逐帧
- [ ] Window Fit：窗口跟随图片比例（ImageGlass）
- [ ] 缩放插值可选：适应时 Linear，100% 时 Nearest（像素画友好）

**视频**
- [x] 音量滑条 + 静音（`M`）— UI 已落地，音频轨解码待 WASAPI
- [ ] 左右键短跳（±5s），Shift+左右 逐帧（已有）对齐 mpv
- [ ] 进度条拖动时显示时间预览气泡
- [x] 打开失败时一键「用系统播放器打开」

**3D**
- [x] 模型 HUD：顶点数 / 三角面 / 包围盒尺寸
- [ ] 重置视角快捷键（`R`）
- [ ] 背景可选：纯色 / 渐变（F3D 默认观感）

**通用**
- [x] 无边框 / 沉浸模式加强（标题栏可隐藏）— 全屏 + 工具栏热区
- [x] 快捷键帮助面板（`?` / `F1`）
- [ ] 缩略图：视频首帧、3D 简易预览图（非仅占位符）

**验收标准**
- 文件夹内连续翻图体感接近 qView
- 视频控件密度接近 Celluloid，不遮挡画面中心
- 新用户 30 秒内能发现全屏 / 100% / 幻灯片

---

### Phase B — 画质与性能（约 2–3 周）· 对标 ImageGlass 缓存 + mpv 硬解 + F3D 观感

**图片**
- [ ] 更激进的邻居预取（±2）与显存/内存双层缓存配额
- [ ] 大图分块上传 / 渐进清晰（>8K 场景）
- [ ] （可选）引入 `zune-jpeg` / 系统 WIC 做 JPEG 快速缩放解码
- [ ] SVG 基础渲染（可选，参考 ImageGlass）

**视频**
- [ ] MF + DXVA2/D3D11 硬解路径探测与开关
- [ ] 精确 seek（关键帧对齐，参考 mpv `hr-seek` 语义）
- [ ] 音频轨播放（WASAPI）
- [ ] 评估可选 **libmpv** 后端（作为“兼容模式”，默认仍 MF）

**3D**
- [ ] **wgpu 真 3D 管线**替换软件光栅（最大里程碑）
- [ ] 基础 PBR（albedo + metallic-roughness + normal）
- [ ] 双灯 + IBL 简化版（学习 F3D / glTF Sample Viewer）
- [ ] 大网格简化/分批绘制，保证 60fps 交互

**验收标准**
- 4K 图切换平均 < 100ms（缓存命中）
- 1080p H.264 硬解开时 CPU < 15%
- 中等 FBX/GLTF（<200k 三角）旋转流畅 60fps

---

### Phase C — 专业能力（约 3–4 周）· 对标 nomacs / VLC / F3D 进阶

**图片**
- [ ] RAW 预览（可走外部解码器或精简依赖）
- [ ] 无损旋转 / 翻转
- [ ] 批量重命名（轻量，参考 nomacs，不做完整 DAM）

**视频**
- [ ] 字幕（软字幕 SRT/ASS 优先）
- [ ] 多音轨切换
- [ ] A-B 循环、播放速度（0.5x–2x）

**3D**
- [ ] 材质贴图完整预览
- [ ] 网格/线框/法线/UV 调试视图
- [ ] 动画 glTF 简易播放（若成本可控）
- [ ] 更多格式：PLY / USDZ（评估体积）

**验收标准**
- 常见创作资产（PNG/JPG/MP4/GLTF/FBX/STL）“打开即用”
- 信息面板足以替代 80% 外部属性查看需求

---

### Phase D — 产品化（与功能并行）· 对标成熟桌面软件

- [ ] 安装包（MSI / 便携 zip 双通道）
- [ ] 文件关联稳定性测试矩阵
- [ ] 自动更新（可选）
- [ ] 崩溃上报 / 日志开关
- [ ] 浅色主题 + 跟随系统
- [ ] 文档：快捷键表、格式支持表、故障排查（缺编解码器等）

---

## 6. 建议的“学习作业”（实现前必做）

| 周次 | 动作 | 产出 |
|------|------|------|
| 第 1 天 | 安装并实测 qView、ImageGlass、mpv/Celluloid、F3D | 每人 1 页「交互笔记」：打开速度、快捷键、失败体验 |
| 第 2 天 | 对比 LookEveryting 同目录同文件 | 差距清单（已反映到本计划勾选） |
| 第 3 天 | 精读：ImageGlass 缓存策略、mpv OSC、F3D 默认渲染参数 | 技术备忘 → Issue 拆分 |
| 持续 | 每个 Phase 结束后回归竞品对照 | 更新本文「状态」列 |

**推荐本地对照样本**
- 图片：24MP JPG、透明 PNG、长图、GIF、HEIC（若系统可解）
- 视频：H.264 MP4、H.265、无声视频、长时长需精确 seek
- 模型：STL、带贴图 GLTF、FBX、超大 OBJ

---

## 7. 优先级排序（下一步先做这 8 项）

1. **EXIF / 文件信息完善**（ImageGlass）— 低成本高感知  
2. **幻灯片**（ImageGlass / qView）  
3. **视频音量 + 静音**（Celluloid）  
4. **视频失败兜底打开**（VLC 思维）  
5. **GIF 动画**（qView）  
6. **3D HUD + 重置视角**（F3D）  
7. **wgpu 3D 管线开工**（F3D 观感的技术前提）  
8. **MF 硬解探测**（mpv 性能语义）

---

## 8. 明确不做 / 延后

| 项目 | 原因 |
|------|------|
| digiKam 级图库 / 人脸分类 | 偏离“本地查看器”定位 |
| 完整视频编辑 / 转码 | VLC 能力过重 |
| VTK / 完整 CAD(STEP) 内核 | 体积与维护成本过高 |
| 云同步 / 账号体系 | 与本地工具定位冲突 |
| 多窗同步对比（nomacs） | Phase C 后再评估 |

---

## 9. 版本映射（更新）

| 版本 | 主题 | 主要对标 |
|------|------|----------|
| **v0.4**（已完成） | 进度条 seek、全屏、原图按需、状态栏、缩略图滚轮 | ImageGlass / mpv 基础 |
| **v0.5** | Phase A：EXIF、幻灯片、GIF、音量、3D HUD | qView + Celluloid + F3D |
| **v0.6** | Phase B：wgpu 3D、硬解、音频、PBR 基础 | mpv + F3D |
| **v0.7** | Phase C：字幕、多音轨、材质贴图、批量轻能力 | VLC + nomacs + F3D |
| **v1.0** | Phase D：安装包、关联、文档、主题 | 成熟桌面产品 |

---

## 10. 成功标准（产品级）

用户能用 LookEveryting **替代日常 80% 场景**：

- 替代 Windows 照片：翻图、全屏、EXIF、幻灯片  
- 替代轻量播放器：本地 MP4 预览、seek、音量（复杂片源可外抛）  
- 替代“双击模型却打不开”：STL/OBJ/GLTF/FBX 打开即转、看得见  

若某功能不能服务上述三者之一，默认不进主线。

---

## 附录：关键仓库

- 图片：https://github.com/jurplel/qView · https://github.com/d2phap/ImageGlass · https://github.com/nomacs/nomacs · https://github.com/easymodo/qimgv  
- 视频：https://github.com/mpv-player/mpv · https://github.com/celluloid-player/celluloid · https://code.videolan.org/videolan/vlc  
- 模型：https://github.com/f3d-app/f3d · https://github.com/sharjith/ModelViewer-Qt · https://github.com/KhronosGroup/glTF-Sample-Viewer  
