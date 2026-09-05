//! Video file metadata and in-app playback.

mod audio;
mod mf_runtime;
mod player;
mod thumb;
mod yuv;

use std::path::Path;

use cap_core::MediaKind;
use thiserror::Error;

pub use audio::{AudioChunk, AudioDecoder, AudioError, AudioFormat};
pub use player::{PlayerError, VideoFrame, VideoPlayer};
pub use thumb::decode_thumbnail;

#[derive(Debug, Error)]
pub enum VideoError {
    #[error("unsupported video format")]
    UnsupportedFormat,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("playback error: {0}")]
    Player(#[from] PlayerError),
}

/// Basic video file information.
#[derive(Debug, Clone)]
pub struct VideoInfo {
    pub format: String,
    pub file_size: u64,
    pub playable_in_app: bool,
    pub notes: String,
    pub duration_secs: f32,
    pub width: u32,
    pub height: u32,
    /// Best-effort: whether DXVA/D3D11 hardware decode may be available on this OS.
    pub hw_decode_available: bool,
}

impl VideoInfo {
    pub fn from_path(path: &Path) -> Result<Self, VideoError> {
        if cap_core::classify_extension(path) != Some(MediaKind::Video) {
            return Err(VideoError::UnsupportedFormat);
        }

        let format = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("video")
            .to_ascii_uppercase();
        let file_size = std::fs::metadata(path)?.len();

        let playable_in_app = matches!(
            format.as_str(),
            "MP4" | "M4V" | "WEBM" | "MOV" | "MKV" | "AVI" | "WMV"
        );

        let notes = if playable_in_app {
            "Press Play for in-app playback.".to_string()
        } else {
            "Format may require external codecs.".to_string()
        };

        Ok(Self {
            format,
            file_size,
            playable_in_app,
            notes,
            duration_secs: 0.0,
            width: 0,
            height: 0,
            hw_decode_available: hw_decode_likely_available(),
        })
    }

    pub fn file_size_label(&self) -> String {
        format_bytes(self.file_size)
    }
}

/// Conservative probe — Windows MF typically has DXVA paths when drivers exist.
fn hw_decode_likely_available() -> bool {
    cfg!(windows)
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.2} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_mp4_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("clip.mp4");
        std::fs::write(&path, vec![0u8; 2048]).unwrap();
        let info = VideoInfo::from_path(&path).unwrap();
        assert_eq!(info.format, "MP4");
        assert!(info.playable_in_app);
        assert!(info.file_size_label().contains("KB"));
    }
}
