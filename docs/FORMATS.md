# LookEveryting — 支持的文件格式

## 图片

| 扩展名 | 说明 |
|--------|------|
| jpg / jpeg | 预览优先；大图上限 4096；按 `1` 加载原图 |
| png | 透明棋盘格 |
| gif | 多帧动画播放 |
| webp / bmp / tiff / ico | 静态解码 |
| cr2 / cr3 / nef / arw / dng / orf / rw2 / raf / pef / srw | RAW 预览（粗 demosaic） |

旋转：`Ctrl+R`；翻转：`H` / `Shift+H`。

## 视频（Windows Media Foundation）

| 扩展名 | 说明 |
|--------|------|
| mp4 / m4v / mov / mkv / avi / wmv / webm | 内嵌播放；音频；seek；±5s；逐帧；侧车 `.srt`/`.ass` 优先，否则 MF 内嵌文本字幕；倍速；A-B；多音轨 `T` |

失败时可「用系统应用打开」。设置中可开关「优先硬件解码」与「显示字幕」。

## 3D 模型

| 扩展名 | 说明 |
|--------|------|
| stl | 常见切片网格 |
| obj | tobj 加载 |
| gltf / glb | glTF（albedo + metallic/roughness + normal map） |
| 3mf | 3MF（ZIP/XML 网格预览） |
| fbx | ufbx |

轨道相机、线框/实体、渐变背景、HUD。实体着色走 **wgpu PaintCallback**（albedo + normal + metallic/roughness + 解析 IBL）；filmstrip 有 CPU 软光栅缩略图。

## 打包

- 便携：`scripts/build.ps1` → `dist/LookEveryting-portable.zip`
- 安装：`scripts/install.ps1`（每用户 LocalAppData + 开始菜单）
- MSI：`scripts/build-msi.ps1`（需安装 [WiX Toolset](https://wixtoolset.org/)）
- 更新：`scripts/check-update.ps1 [-Download]`

## 快捷键摘要

见应用内 `?` 面板。常用：`←→` 导航、`空格` 幻灯/播放、`V` 字幕、`T` 音轨、`M` 静音、`[` `]` 倍速、`A`/`B` 循环、`Ctrl+R`/`H` 旋转翻转、`F11` 全屏、`F2` 重命名。
