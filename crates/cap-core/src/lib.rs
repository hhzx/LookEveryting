//! Core types shared across LookEveryting crates.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// High-level media category inferred from file extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaKind {
    Image,
    Video,
    Model,
    Unknown,
}

impl MediaKind {
    pub fn from_path(path: &Path) -> Self {
        classify_extension(path).unwrap_or(Self::Unknown)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
            Self::Model => "model",
            Self::Unknown => "unknown",
        }
    }
}

/// Classify a file path by extension.
pub fn classify_extension(path: &Path) -> Option<MediaKind> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        return Some(MediaKind::Image);
    }
    if VIDEO_EXTENSIONS.contains(&ext.as_str()) {
        return Some(MediaKind::Video);
    }
    if MODEL_EXTENSIONS.contains(&ext.as_str()) {
        return Some(MediaKind::Model);
    }
    None
}

/// List supported extensions for a media kind.
pub fn extensions_for(kind: MediaKind) -> &'static [&'static str] {
    match kind {
        MediaKind::Image => IMAGE_EXTENSIONS,
        MediaKind::Video => VIDEO_EXTENSIONS,
        MediaKind::Model => MODEL_EXTENSIONS,
        MediaKind::Unknown => &[],
    }
}

/// Build an rfd file-dialog filter for all supported types.
pub fn all_supported_filter() -> (&'static str, Vec<&'static str>) {
    let mut exts: Vec<&'static str> = Vec::new();
    exts.extend_from_slice(IMAGE_EXTENSIONS);
    exts.extend_from_slice(VIDEO_EXTENSIONS);
    exts.extend_from_slice(MODEL_EXTENSIONS);
    ("Supported files", exts)
}

/// Application settings persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub locale: String,
    pub theme: ThemePreference,
    pub density: UiDensity,
    pub toolbar_auto_hide: bool,
    pub last_directory: Option<PathBuf>,
    #[serde(default)]
    pub file_associations: FileAssociations,
    /// Prefer hardware-accelerated video decode when the platform supports it.
    #[serde(default = "default_true")]
    pub prefer_hw_decode: bool,
    #[serde(default = "default_true")]
    pub show_subtitles: bool,
}

fn default_true() -> bool {
    true
}

/// Which media kinds are registered as default open handlers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAssociations {
    pub images: bool,
    pub videos: bool,
    pub models: bool,
}

impl Default for FileAssociations {
    fn default() -> Self {
        Self {
            images: false,
            videos: false,
            models: false,
        }
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            locale: "zh-Hans".to_string(),
            theme: ThemePreference::Dark,
            density: UiDensity::Comfortable,
            toolbar_auto_hide: true,
            last_directory: None,
            file_associations: FileAssociations::default(),
            prefer_hw_decode: true,
            show_subtitles: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    Dark,
    Light,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiDensity {
    Compact,
    Comfortable,
    Accessibility,
}

impl UiDensity {
    pub fn scale(self) -> f32 {
        match self {
            Self::Compact => 0.9,
            Self::Comfortable => 1.0,
            Self::Accessibility => 1.15,
        }
    }
}

/// Load settings from the default config path.
pub fn load_settings() -> AppSettings {
    let path = settings_path();
    if !path.exists() {
        return AppSettings::default();
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| toml::from_str(&text).ok())
        .unwrap_or_default()
}

/// Save settings to the default config path.
pub fn save_settings(settings: &AppSettings) -> std::io::Result<()> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = toml::to_string_pretty(settings).unwrap_or_default();
    std::fs::write(path, text)
}

fn settings_path() -> PathBuf {
    directories_path().join("settings.toml")
}

fn directories_path() -> PathBuf {
    if let Some(dirs) = option_env!("LOOKEVERYTING_CONFIG_DIR").map(PathBuf::from) {
        return dirs;
    }
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("LookEveryting");
        }
    }
    dirs_fallback()
}

fn dirs_fallback() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".lookeveryting")
}

const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "webp", "bmp", "tif", "tiff", "ico", "avif",
    "cr2", "cr3", "nef", "arw", "dng", "orf", "rw2", "raf", "pef", "srw",
];

const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "m4v", "mov", "mkv", "webm", "avi", "wmv", "flv", "mpg", "mpeg",
];

const MODEL_EXTENSIONS: &[&str] = &[
    "glb", "gltf", "obj", "stl", "fbx", "ply", "dae", "3mf", "max",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_common_extensions() {
        assert_eq!(
            classify_extension(Path::new("photo.JPG")),
            Some(MediaKind::Image)
        );
        assert_eq!(
            classify_extension(Path::new("clip.mp4")),
            Some(MediaKind::Video)
        );
        assert_eq!(
            classify_extension(Path::new("mesh.glb")),
            Some(MediaKind::Model)
        );
        assert_eq!(classify_extension(Path::new("readme.txt")), None);
    }

    #[test]
    fn settings_roundtrip() {
        let mut settings = AppSettings::default();
        settings.locale = "en-US".to_string();
        settings.toolbar_auto_hide = false;
        let text = toml::to_string_pretty(&settings).unwrap();
        let parsed: AppSettings = toml::from_str(&text).unwrap();
        assert_eq!(parsed.locale, "en-US");
        assert!(!parsed.toolbar_auto_hide);
    }
}
