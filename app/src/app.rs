//! Application state and media session management.

use std::path::{Path, PathBuf};
use std::time::Instant;

use cap_core::{classify_extension, load_settings, save_settings, AppSettings, MediaKind};
use cap_i18n::{I18n, Locale};
use cap_image::DecodedImage;
use cap_model::{load_mesh, MeshData, ModelInfo};
use cap_ui::layout::{LayoutMode, ViewerMode};
use cap_video::{VideoInfo, VideoPlayer};
use cap_viewer::OrbitCamera;
use egui::{ColorImage, TextureHandle, Vec2};

use crate::thumbnails::ThumbnailCache;

/// Loaded media content for the viewport.
pub enum LoadedMedia {
    Image {
        decoded: DecodedImage,
        texture: Option<TextureHandle>,
    },
    Video {
        info: VideoInfo,
        path: PathBuf,
        player: Option<VideoPlayer>,
        texture: Option<TextureHandle>,
        playing: bool,
    },
    Model {
        info: ModelInfo,
        path: PathBuf,
        wireframe: bool,
        mesh: Option<MeshData>,
        camera: OrbitCamera,
    },
}

/// Top-level application state.
pub struct LookApp {
    pub settings: AppSettings,
    pub i18n: I18n,
    pub viewer_mode: ViewerMode,
    pub info_open: bool,
    pub settings_open: bool,
    pub toolbar_visible: bool,
    pub last_interaction: Instant,
    pub current_path: Option<PathBuf>,
    pub folder_files: Vec<PathBuf>,
    pub current_index: usize,
    pub media: Option<LoadedMedia>,
    pub zoom: f32,
    pub fit_mode: bool,
    pub pan: Vec2,
    pub toast: Option<String>,
    pub error: Option<String>,
    pub thumbnails: ThumbnailCache,
    pub association_message: Option<String>,
    /// Avoid re-scrolling the filmstrip every frame (prevents vertical jump).
    pub thumb_scroll_synced_index: Option<usize>,
}

impl LookApp {
    pub fn new() -> Self {
        let settings = load_settings();
        let locale = Locale::from_id(&settings.locale).unwrap_or(Locale::ZhHans);
        Self {
            i18n: I18n::new(locale),
            settings,
            viewer_mode: ViewerMode::Viewer,
            info_open: false,
            settings_open: false,
            toolbar_visible: true,
            last_interaction: Instant::now(),
            current_path: None,
            folder_files: Vec::new(),
            current_index: 0,
            media: None,
            zoom: 1.0,
            fit_mode: true,
            pan: Vec2::ZERO,
            toast: None,
            error: None,
            thumbnails: ThumbnailCache::new(),
            association_message: None,
            thumb_scroll_synced_index: None,
        }
    }

    pub fn touch(&mut self) {
        self.last_interaction = Instant::now();
        self.toolbar_visible = true;
    }

    pub fn maybe_hide_toolbar(&mut self) {
        if !self.settings.toolbar_auto_hide {
            return;
        }
        if self.viewer_mode == ViewerMode::Immersive
            || self.last_interaction.elapsed().as_millis() > 3000
        {
            self.toolbar_visible = false;
        }
    }

    pub fn open_path(&mut self, path: PathBuf) {
        if self.current_path.as_ref() == Some(&path) {
            return;
        }
        self.touch();
        self.error = None;

        if let Some(parent) = path.parent() {
            self.settings.last_directory = Some(parent.to_path_buf());
            let _ = save_settings(&self.settings);
            self.refresh_folder(parent, &path);
        }

        match classify_extension(&path) {
            Some(MediaKind::Image) => match DecodedImage::from_path(&path) {
                Ok(decoded) => {
                    self.media = Some(LoadedMedia::Image {
                        decoded,
                        texture: None,
                    });
                    self.current_path = Some(path);
                    self.fit_mode = true;
                    self.zoom = 1.0;
                    self.pan = Vec2::ZERO;
                    self.viewer_mode = ViewerMode::Viewer;
                }
                Err(err) => self.error = Some(err.to_string()),
            },
            Some(MediaKind::Video) => match VideoInfo::from_path(&path) {
                Ok(info) => match VideoPlayer::open(path.clone()) {
                    Ok(player) => {
                        self.media = Some(LoadedMedia::Video {
                            info,
                            path: path.clone(),
                            player: Some(player),
                            texture: None,
                            playing: false,
                        });
                        self.current_path = Some(path);
                        self.viewer_mode = ViewerMode::Viewer;
                    }
                    Err(err) => self.error = Some(err.to_string()),
                },
                Err(err) => self.error = Some(err.to_string()),
            },
            Some(MediaKind::Model) => match ModelInfo::from_path(&path) {
                Ok(info) => {
                    let mesh = match load_mesh(&path) {
                        Ok(m) => Some(m),
                        Err(err) => {
                            self.error = Some(err.to_string());
                            None
                        }
                    };
                    let camera = mesh
                        .as_ref()
                        .map(|m| OrbitCamera::fit_bounds(&m.bounds))
                        .unwrap_or_default();
                    self.media = Some(LoadedMedia::Model {
                        info,
                        path: path.clone(),
                        wireframe: false,
                        mesh,
                        camera,
                    });
                    self.current_path = Some(path);
                    self.viewer_mode = ViewerMode::Viewer;
                }
                Err(err) => self.error = Some(err.to_string()),
            },
            None | Some(MediaKind::Unknown) => {
                self.error = Some(self.i18n.t("toast-file-not-supported").to_string())
            }
        }
    }

    pub fn refresh_folder(&mut self, folder: &Path, current: &Path) {
        let same_folder = self
            .current_path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p == folder)
            .unwrap_or(false);

        let mut files: Vec<PathBuf> = walkdir::WalkDir::new(folder)
            .max_depth(1)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .filter(|e| classify_extension(e.path()).is_some())
            .map(|e| e.path().to_path_buf())
            .collect();
        files.sort();
        self.current_index = files
            .iter()
            .position(|p| p == current)
            .unwrap_or(0);
        self.folder_files = files;
        if !same_folder {
            self.thumbnails.clear();
            self.thumb_scroll_synced_index = None;
        }
    }

    pub fn navigate(&mut self, delta: isize) {
        if self.folder_files.is_empty() {
            return;
        }
        let len = self.folder_files.len();
        let next = (self.current_index as isize + delta).rem_euclid(len as isize) as usize;
        let path = self.folder_files[next].clone();
        self.open_path(path);
    }

    pub fn navigate_to_index(&mut self, index: usize) {
        if index >= self.folder_files.len() {
            return;
        }
        let path = self.folder_files[index].clone();
        self.open_path(path);
    }

    pub fn ensure_texture(&mut self, ctx: &egui::Context) {
        if let Some(LoadedMedia::Image { decoded, texture }) = &mut self.media {
            if texture.is_none() {
                let size = [decoded.width as usize, decoded.height as usize];
                let image = ColorImage::from_rgba_unmultiplied(size, &decoded.rgba);
                let handle = ctx.load_texture(
                    format!("img-{}", self.current_index),
                    image,
                    egui::TextureOptions::LINEAR,
                );
                *texture = Some(handle);
            }
        }
        self.ensure_video_texture(ctx);
    }

    pub fn tick_video(&mut self, ctx: &egui::Context) {
        let mut frame = None;
        let mut needs_texture = false;
        if let Some(LoadedMedia::Video { player, playing, texture, .. }) = &mut self.media {
            needs_texture = texture.is_none();
            if let Some(player) = player {
                if *playing {
                    frame = player.tick();
                } else if needs_texture {
                    frame = player
                        .current_frame()
                        .cloned()
                        .or_else(|| player.seek_start());
                }
            }
        }
        if let Some(frame) = frame {
            self.upload_video_texture(ctx, &frame);
            ctx.request_repaint();
        }
    }

    fn upload_video_texture(&mut self, ctx: &egui::Context, frame: &cap_video::VideoFrame) {
        if let Some(LoadedMedia::Video { texture, .. }) = &mut self.media {
            let image = ColorImage::from_rgba_unmultiplied(
                [frame.width as usize, frame.height as usize],
                &frame.rgba,
            );
            let handle = ctx.load_texture(
                format!("video-{}", self.current_index),
                image,
                egui::TextureOptions::LINEAR,
            );
            *texture = Some(handle);
        }
    }

    fn ensure_video_texture(&mut self, ctx: &egui::Context) {
        if let Some(LoadedMedia::Video { player, texture, .. }) = &mut self.media {
            if texture.is_none() {
                if let Some(player) = player {
                    if let Some(frame) = player.current_frame().cloned() {
                        self.upload_video_texture(ctx, &frame);
                    }
                }
            }
        }
    }

    pub fn toggle_video_playback(&mut self) {
        if let Some(LoadedMedia::Video { player, playing, .. }) = &mut self.media {
            if let Some(player) = player {
                player.toggle();
                *playing = player.is_playing();
                self.touch();
            }
        }
    }

    pub fn video_is_playing(&self) -> bool {
        matches!(
            &self.media,
            Some(LoadedMedia::Video { playing: true, .. })
        )
    }

    pub fn file_name(&self) -> &str {
        self.current_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("")
    }

    pub fn counter_label(&self) -> String {
        if self.folder_files.is_empty() {
            return String::new();
        }
        self.i18n
            .t("counter")
            .replace("{current}", &(self.current_index + 1).to_string())
            .replace("{total}", &self.folder_files.len().to_string())
    }

    pub fn layout_mode(&self, width: f32) -> LayoutMode {
        LayoutMode::from_width(width)
    }

    pub fn play_video_externally(&self) {
        if let Some(LoadedMedia::Video { path, .. }) = &self.media {
            let _ = open::that(path);
        }
    }

    pub fn open_model_externally(&self) {
        if let Some(LoadedMedia::Model { path, .. }) = &self.media {
            let _ = open::that(path);
        }
    }

    pub fn apply_file_associations(&mut self) {
        self.association_message = None;
        let prefs = self.settings.file_associations.clone();
        let result = if prefs.images || prefs.videos || prefs.models {
            cap_shell::current_exe()
                .and_then(|exe| cap_shell::apply_file_associations(&exe, &prefs))
        } else {
            cap_shell::clear_file_associations()
        };
        match result {
            Ok(()) => {
                self.association_message = Some(self.i18n.t("settings-assoc-success").to_string());
                let _ = save_settings(&self.settings);
            }
            Err(err) => {
                self.association_message = Some(
                    self.i18n
                        .t("settings-assoc-failed")
                        .replace("{error}", &err.to_string()),
                );
            }
        }
    }
}

impl Default for LookApp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    fn write_png(path: &Path) {
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_fn(8, 8, |x, y| Rgba([x as u8 * 20, y as u8 * 20, 128, 255]));
        img.save(path).unwrap();
    }

    #[test]
    fn opens_image_and_indexes_folder() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.png");
        let b = dir.path().join("b.png");
        write_png(&a);
        write_png(&b);
        let mut app = LookApp::new();
        app.open_path(a);
        assert!(matches!(app.media, Some(LoadedMedia::Image { .. })));
        assert_eq!(app.folder_files.len(), 2);
        app.navigate(1);
        assert_eq!(app.current_index, 1);
    }
}
