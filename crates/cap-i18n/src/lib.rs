//! Lightweight runtime localization (no per-frame allocation).

use std::collections::HashMap;

/// BCP 47 locale identifiers supported by the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    EnUs,
    ZhHans,
}

impl Locale {
    pub const ALL: &'static [Locale] = &[Locale::EnUs, Locale::ZhHans];

    pub fn id(self) -> &'static str {
        match self {
            Self::EnUs => "en-US",
            Self::ZhHans => "zh-Hans",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::EnUs => "English",
            Self::ZhHans => "简体中文",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "en-US" | "en" => Some(Self::EnUs),
            "zh-Hans" | "zh-CN" | "zh" => Some(Self::ZhHans),
            _ => None,
        }
    }

    pub fn fallback() -> Self {
        Self::EnUs
    }
}

/// Translation bundle for one locale.
#[derive(Debug, Clone, Default)]
pub struct Bundle {
    strings: HashMap<&'static str, &'static str>,
}

impl Bundle {
    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.strings.get(key).copied().unwrap_or(key)
    }
}

/// Active localization context.
#[derive(Debug, Clone)]
pub struct I18n {
    locale: Locale,
    bundle: Bundle,
}

impl I18n {
    pub fn new(locale: Locale) -> Self {
        Self {
            bundle: locale.bundle(),
            locale,
        }
    }

    pub fn locale(&self) -> Locale {
        self.locale
    }

    pub fn set_locale(&mut self, locale: Locale) {
        self.locale = locale;
        self.bundle = locale.bundle();
    }

    pub fn t<'a>(&'a self, key: &'a str) -> &'a str {
        self.bundle.get(key)
    }
}

impl Locale {
    pub fn bundle(self) -> Bundle {
        let strings: HashMap<&'static str, &'static str> = match self {
            Self::EnUs => en_us(),
            Self::ZhHans => zh_hans(),
        }
        .into_iter()
        .collect();
        Bundle { strings }
    }
}

fn en_us() -> Vec<(&'static str, &'static str)> {
    vec![
        ("app-title", "LookEveryting"),
        ("empty-title", "Drop files here"),
        ("empty-subtitle", "or press Ctrl+O to open"),
        ("empty-open-button", "Open File"),
        ("toolbar-prev", "Previous"),
        ("toolbar-next", "Next"),
        ("toolbar-fit", "Fit"),
        ("toolbar-actual-size", "100%"),
        ("toolbar-zoom-in", "Zoom In"),
        ("toolbar-zoom-out", "Zoom Out"),
        ("toolbar-info", "Info"),
        ("video-play", "Play"),
        ("video-pause", "Pause"),
        ("video-loading", "Loading video…"),
        ("model-solid", "Solid"),
        ("model-wireframe", "Wireframe"),
        ("browse-grid", "Grid"),
        ("browse-list", "List"),
        ("settings-title", "Settings"),
        ("settings-language", "Language"),
        ("settings-theme", "Theme"),
        ("settings-density", "UI Density"),
        ("settings-toolbar-auto-hide", "Auto-hide toolbar"),
        ("common-open", "Open"),
        ("common-close", "Close"),
        ("panel-file-info", "File Info"),
        ("panel-model-info", "Model Info"),
        ("toast-open-failed", "Failed to open file"),
        ("mode-image", "Image"),
        ("mode-video", "Video"),
        ("mode-model", "3D Model"),
        ("counter", "{current} / {total}"),
        ("browse-thumbnails", "Files in folder"),
        ("settings-associations", "Open with LookEveryting"),
        ("settings-associations-hint", "Register as default handler for selected file types (Windows)."),
        ("settings-assoc-images", "Images (jpg, png, webp…)"),
        ("settings-assoc-videos", "Videos (mp4, mkv, mov…)"),
        ("settings-assoc-models", "3D models (fbx, obj, gltf, stl…)"),
        ("settings-assoc-apply", "Apply associations"),
        ("settings-assoc-success", "File associations updated."),
        ("settings-assoc-failed", "Failed: {error}"),
        ("video-external-hint", "Embedded playback coming soon — click to open in system player"),
        ("model-hint", "Drag: rotate · Scroll: zoom"),
        ("toast-file-not-supported", "Unsupported file format"),
    ]
}

fn zh_hans() -> Vec<(&'static str, &'static str)> {
    vec![
        ("app-title", "LookEveryting"),
        ("empty-title", "拖放文件到此处"),
        ("empty-subtitle", "或按 Ctrl+O 打开"),
        ("empty-open-button", "打开文件"),
        ("toolbar-prev", "上一张"),
        ("toolbar-next", "下一张"),
        ("toolbar-fit", "适应"),
        ("toolbar-actual-size", "100%"),
        ("toolbar-zoom-in", "放大"),
        ("toolbar-zoom-out", "缩小"),
        ("toolbar-info", "信息"),
        ("video-play", "播放"),
        ("video-pause", "暂停"),
        ("video-loading", "正在加载视频…"),
        ("model-solid", "实体"),
        ("model-wireframe", "线框"),
        ("browse-grid", "网格"),
        ("browse-list", "列表"),
        ("settings-title", "设置"),
        ("settings-language", "语言"),
        ("settings-theme", "主题"),
        ("settings-density", "界面密度"),
        ("settings-toolbar-auto-hide", "工具栏自动隐藏"),
        ("common-open", "打开"),
        ("common-close", "关闭"),
        ("panel-file-info", "文件信息"),
        ("panel-model-info", "模型信息"),
        ("toast-open-failed", "打开文件失败"),
        ("mode-image", "图片"),
        ("mode-video", "视频"),
        ("mode-model", "3D 模型"),
        ("counter", "第 {current} 张，共 {total} 张"),
        ("browse-thumbnails", "文件夹"),
        ("settings-associations", "关联打开"),
        ("settings-associations-hint", "将选中的文件类型注册为用本软件打开（Windows）。"),
        ("settings-assoc-images", "图片（jpg、png、webp 等）"),
        ("settings-assoc-videos", "视频（mp4、mkv、mov 等）"),
        ("settings-assoc-models", "3D 模型（fbx、obj、gltf、stl 等）"),
        ("settings-assoc-apply", "应用关联"),
        ("settings-assoc-success", "文件关联已更新。"),
        ("settings-assoc-failed", "设置失败：{error}"),
        ("video-external-hint", "内嵌播放即将支持 — 点击用系统播放器打开"),
        ("model-hint", "拖动旋转 · 滚轮缩放"),
        ("toast-file-not-supported", "不支持的文件格式"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_zh() {
        let i18n = I18n::new(Locale::ZhHans);
        assert_eq!(i18n.t("empty-title"), "拖放文件到此处");
    }

    #[test]
    fn falls_back_to_key() {
        let i18n = I18n::new(Locale::EnUs);
        assert_eq!(i18n.t("missing-key"), "missing-key");
    }
}
