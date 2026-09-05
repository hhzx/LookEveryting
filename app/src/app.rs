//! Application state and media session management.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use cap_core::{classify_extension, load_settings, save_settings, AppSettings, MediaKind};
use cap_i18n::{I18n, Locale};
use cap_image::DecodedImage;
use cap_model::{MeshData, ModelInfo};
use cap_ui::layout::{LayoutMode, ViewerMode};
use cap_video::VideoInfo;
use cap_viewer::OrbitCamera;
use egui::{ColorImage, TextureHandle, Vec2};

use crate::loader::{paths_equal, normalize_path, ImageCache, LoadedPayload, LoadMessage, MediaLoader, next_generation};
use crate::thumbnails::ThumbnailCache;
use crate::video_thread::{VideoEvent, VideoThread};

/// Loaded media content for the viewport.
pub enum LoadedMedia {
    Loading {
        path: PathBuf,
    },
    Image {
        width: u32,
        height: u32,
        native_width: u32,
        native_height: u32,
        full_res_loading: bool,
        rgba: Option<Vec<u8>>,
        texture: Option<TextureHandle>,
    },
    Video {
        info: VideoInfo,
        path: PathBuf,
        texture: Option<TextureHandle>,
        playing: bool,
        ready: bool,
        duration_secs: f32,
        position_secs: f32,
        position_fraction: f32,
    },
    Model {
        info: ModelInfo,
        path: PathBuf,
        wireframe: bool,
        mesh: Option<MeshData>,
        camera: OrbitCamera,
    },
}

impl LoadedMedia {
    pub fn image_size(&self) -> Option<Vec2> {
        match self {
            LoadedMedia::Image { width, height, .. } => Some(Vec2::new(*width as f32, *height as f32)),
            _ => None,
        }
    }
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
    pub cached_folder: Option<PathBuf>,
    pub current_index: usize,
    pub media: Option<LoadedMedia>,
    pub zoom: f32,
    pub fit_mode: bool,
    pub pan: Vec2,
    pub toast: Option<String>,
    pub error: Option<String>,
    pub thumbnails: ThumbnailCache,
    pub association_message: Option<String>,
    pub thumb_scroll_synced_index: Option<usize>,
    pub loader: MediaLoader,
    pub load_generation: u64,
    pub image_cache: ImageCache,
    pub texture_cache: HashMap<String, TextureHandle>,
    pub video_engine: VideoThread,
    pub load_started: Instant,
    pub settings_dirty: bool,
    pub thumb_queue_cursor: usize,
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
            cached_folder: None,
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
            loader: MediaLoader::spawn(),
            load_generation: 0,
            image_cache: ImageCache::default(),
            texture_cache: HashMap::new(),
            video_engine: VideoThread::spawn(),
            load_started: Instant::now(),
            settings_dirty: false,
            thumb_queue_cursor: 0,
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

    pub fn flush_settings_if_dirty(&mut self) {
        if self.settings_dirty {
            let _ = save_settings(&self.settings);
            self.settings_dirty = false;
        }
    }

    pub fn open_path(&mut self, path: PathBuf) {
        let path = normalize_path(path);
        if self
            .current_path
            .as_ref()
            .is_some_and(|p| paths_equal(p, &path))
        {
            return;
        }
        self.touch();
        self.error = None;

        if let Some(parent) = path.parent() {
            self.settings.last_directory = Some(parent.to_path_buf());
            self.settings_dirty = true;
            self.refresh_folder(parent, &path);
        }

        self.current_path = Some(path.clone());
        self.load_generation = next_generation();
        self.thumb_queue_cursor = self.current_index;

        if classify_extension(&path) == Some(MediaKind::Video) {
            let info = VideoInfo::from_path(&path).unwrap_or_else(|_| VideoInfo {
                format: path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("VIDEO")
                    .to_ascii_uppercase(),
                file_size: std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0),
                playable_in_app: true,
                notes: String::new(),
                duration_secs: 0.0,
                width: 0,
                height: 0,
            });
            self.video_engine.open(path.clone());
            self.media = Some(LoadedMedia::Video {
                info,
                path,
                texture: None,
                playing: false,
                ready: false,
                duration_secs: 0.0,
                position_secs: 0.0,
                position_fraction: 0.0,
            });
            self.viewer_mode = ViewerMode::Viewer;
            return;
        }

        self.media = Some(LoadedMedia::Loading { path: path.clone() });
        self.load_started = Instant::now();

        let cache_key = path.clone();
        if let Some(cached) = self.image_cache.get(&cache_key).cloned() {
            let tex_key = cache_key.to_string_lossy().to_string();
            if let Some(tex) = self.texture_cache.get(&tex_key).cloned() {
                self.media = Some(LoadedMedia::Image {
                    width: cached.width,
                    height: cached.height,
                    native_width: cached.native_width,
                    native_height: cached.native_height,
                    full_res_loading: false,
                    rgba: None,
                    texture: Some(tex),
                });
                self.viewer_mode = ViewerMode::Viewer;
                return;
            }
            self.apply_image(cached, false);
            return;
        }

        self.loader
            .request_media(path, self.load_generation);
        self.prefetch_neighbors();
    }

    pub fn poll_loader(&mut self) {
        let messages = self.loader.poll();
        let current = self.current_path.clone();
        let generation = self.load_generation;

        for msg in messages {
            match msg {
                LoadMessage::Preview {
                    path,
                    generation: gen,
                    decoded,
                } => {
                    if gen != generation {
                        continue;
                    }
                    if current.as_ref().is_some_and(|c| paths_equal(c, &path))
                        && !matches!(self.media, Some(LoadedMedia::Image { .. }))
                    {
                        self.apply_image(decoded, false);
                    }
                }
                LoadMessage::Ready {
                    path,
                    generation: gen,
                    result,
                } => {
                    if gen != u64::MAX && gen != generation {
                        continue;
                    }
                    if gen == u64::MAX {
                        if let Ok(LoadedPayload::Image(decoded)) = &result {
                            self.cache_image(path, decoded.clone());
                        }
                        continue;
                    }
                    if current.as_ref().is_none_or(|c| !paths_equal(c, &path)) {
                        if let Ok(LoadedPayload::Image(decoded)) = &result {
                            self.cache_image(path, decoded.clone());
                        }
                        continue;
                    }
                    match result {
                        Ok(payload) => self.apply_payload(path, payload),
                        Err(err) => {
                            self.error = Some(err);
                            self.media = None;
                        }
                    }
                }
                LoadMessage::Thumbnail { path, decoded } => {
                    self.thumbnails.pending.insert(
                        path.to_string_lossy().to_string(),
                        decoded,
                    );
                }
            }
        }
    }

    fn poll_video_events(&mut self, ctx: &egui::Context) {
        let events = self.video_engine.poll();
        if !matches!(self.media, Some(LoadedMedia::Video { .. })) {
            return;
        }

        for evt in events {
            match evt {
                VideoEvent::Opened {
                    info,
                    duration_secs,
                    width: _,
                    height: _,
                    first_frame,
                } => {
                    if let Some(LoadedMedia::Video {
                        info: slot,
                        ready,
                        duration_secs: dur_slot,
                        ..
                    }) = &mut self.media
                    {
                        *slot = info;
                        *ready = true;
                        *dur_slot = duration_secs;
                    }
                    if let Some(frame) = first_frame {
                        self.upload_video_texture(ctx, &frame);
                    }
                    ctx.request_repaint();
                }
                VideoEvent::Frame(frame) => {
                    self.upload_video_texture(ctx, &frame);
                    ctx.request_repaint();
                }
                VideoEvent::Playing(playing) => {
                    if let Some(LoadedMedia::Video {
                        playing: slot, ..
                    }) = &mut self.media
                    {
                        *slot = playing;
                    }
                }
                VideoEvent::Position { fraction, secs } => {
                    if let Some(LoadedMedia::Video {
                        position_fraction,
                        position_secs,
                        ..
                    }) = &mut self.media
                    {
                        *position_fraction = fraction;
                        *position_secs = secs;
                    }
                }
                VideoEvent::Error(err) => {
                    self.error = Some(err);
                    self.media = None;
                }
            }
        }
    }

    fn prefetch_neighbors(&mut self) {
        if self.folder_files.is_empty() {
            return;
        }
        let idx = self.current_index;
        let len = self.folder_files.len();
        for delta in [1isize, -1] {
            let ni = (idx as isize + delta).rem_euclid(len as isize) as usize;
            let path = &self.folder_files[ni];
            if classify_extension(path) == Some(MediaKind::Image)
                && self.image_cache.get(path).is_none()
            {
                self.loader.request_prefetch(path.clone());
            }
        }
    }

    fn apply_payload(&mut self, path: PathBuf, payload: LoadedPayload) {
        match payload {
            LoadedPayload::Image(decoded) => {
                self.cache_image(path, decoded.clone());
                let upgrade = matches!(self.media, Some(LoadedMedia::Image { .. }));
                self.apply_image(decoded, upgrade);
            }
            LoadedPayload::Model {
                info,
                mesh,
                camera,
            } => {
                if mesh.is_none() {
                    self.error = Some(self.i18n.t("toast-open-failed").to_string());
                }
                self.media = Some(LoadedMedia::Model {
                    info,
                    path,
                    wireframe: false,
                    mesh,
                    camera,
                });
                self.viewer_mode = ViewerMode::Viewer;
            }
        }
    }

    fn cache_image(&mut self, path: PathBuf, decoded: DecodedImage) {
        for evicted in self.image_cache.insert(path.clone(), decoded) {
            let key = evicted.to_string_lossy().to_string();
            self.texture_cache.remove(&key);
        }
    }

    fn apply_image(&mut self, decoded: DecodedImage, upgrade: bool) {
        if upgrade {
            if let Some(LoadedMedia::Image {
                width,
                height,
                native_width,
                native_height,
                full_res_loading,
                rgba,
                texture,
            }) = &mut self.media
            {
                *width = decoded.width;
                *height = decoded.height;
                *native_width = decoded.native_width;
                *native_height = decoded.native_height;
                *full_res_loading = false;
                *rgba = Some(decoded.rgba);
                *texture = None;
                return;
            }
        }
        self.media = Some(LoadedMedia::Image {
            width: decoded.width,
            height: decoded.height,
            native_width: decoded.native_width,
            native_height: decoded.native_height,
            full_res_loading: false,
            rgba: Some(decoded.rgba),
            texture: None,
        });
        self.fit_mode = true;
        self.zoom = 1.0;
        self.pan = Vec2::ZERO;
        self.viewer_mode = ViewerMode::Viewer;
    }

    pub fn request_full_res_if_needed(&mut self) {
        let Some(path) = self.current_path.clone() else {
            return;
        };
        let needs_full = matches!(
            &self.media,
            Some(LoadedMedia::Image {
                full_res_loading: false,
                ..
            }) if self.image_needs_full_res()
        );
        if !needs_full {
            return;
        }
        if let Some(LoadedMedia::Image {
            full_res_loading, ..
        }) = &mut self.media
        {
            *full_res_loading = true;
        }
        self.loader
            .request_full_res(path, self.load_generation);
    }

    pub fn image_needs_full_res(&self) -> bool {
        matches!(
            &self.media,
            Some(LoadedMedia::Image {
                width,
                height,
                native_width,
                native_height,
                ..
            }) if *width != *native_width || *height != *native_height
        )
    }

    pub fn image_is_capped(&self) -> bool {
        self.image_needs_full_res()
    }

    pub fn seek_video(&mut self, fraction: f32, ctx: &egui::Context) {
        if matches!(self.media, Some(LoadedMedia::Video { .. })) {
            self.video_engine.seek(fraction);
            self.touch();
            self.poll_video_events(ctx);
        }
    }

    pub fn toggle_fullscreen(&mut self, ctx: &egui::Context) {
        let entering = self.viewer_mode != ViewerMode::Fullscreen;
        self.viewer_mode = if entering {
            ViewerMode::Fullscreen
        } else {
            ViewerMode::Viewer
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(entering));
        self.touch();
    }

    pub fn is_fullscreen(&self) -> bool {
        self.viewer_mode == ViewerMode::Fullscreen
    }

    const TEXTURE_CACHE_MAX: usize = 16;

    fn store_texture(&mut self, key: String, handle: TextureHandle) -> TextureHandle {
        if self.texture_cache.len() >= Self::TEXTURE_CACHE_MAX
            && !self.texture_cache.contains_key(&key)
        {
            if let Some(oldest) = self.texture_cache.keys().next().cloned() {
                self.texture_cache.remove(&oldest);
            }
        }
        self.texture_cache.insert(key.clone(), handle.clone());
        handle
    }

    pub fn is_loading(&self) -> bool {
        matches!(self.media, Some(LoadedMedia::Loading { .. }))
    }

    pub fn refresh_folder(&mut self, folder: &Path, current: &Path) {
        let same_folder = self.cached_folder.as_deref() == Some(folder);

        if !same_folder {
            let mut files: Vec<PathBuf> = walkdir::WalkDir::new(folder)
                .max_depth(1)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| e.file_type().is_file())
                .filter(|e| classify_extension(e.path()).is_some())
                .map(|e| e.path().to_path_buf())
                .collect();
            files.sort();
            self.folder_files = files;
            self.cached_folder = Some(folder.to_path_buf());
            self.thumbnails.clear();
            self.thumb_scroll_synced_index = None;
            self.thumb_queue_cursor = 0;
        }

        self.current_index = self
            .folder_files
            .iter()
            .position(|p| paths_equal(p, current))
            .unwrap_or(0);
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
        let upload = if let Some(LoadedMedia::Image {
            width,
            height,
            rgba,
            texture,
            ..
        }) = &mut self.media
        {
            if texture.is_none() {
                rgba.take()
                    .map(|pixels| (*width as usize, *height as usize, pixels))
            } else {
                None
            }
        } else {
            None
        };

        if let Some((width, height, pixels)) = upload {
            let path_key = self
                .current_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("img-{}", self.current_index));
            let image = ColorImage::from_rgba_unmultiplied([width, height], &pixels);
            let handle = ctx.load_texture(
                path_key.clone(),
                image,
                egui::TextureOptions::LINEAR,
            );
            let stored = self.store_texture(path_key, handle);
            if let Some(LoadedMedia::Image { texture, .. }) = &mut self.media {
                *texture = Some(stored);
            }
        }
        self.ensure_video_texture(ctx);
    }

    pub fn tick_video(&mut self, ctx: &egui::Context) {
        if let Some(LoadedMedia::Video { playing, texture, .. }) = &self.media {
            if *playing || texture.is_none() {
                self.video_engine.tick();
            }
        }
        self.poll_video_events(ctx);
    }

    pub fn poll_video(&mut self, ctx: &egui::Context) {
        self.poll_video_events(ctx);
    }

    fn upload_video_texture(&mut self, ctx: &egui::Context, frame: &cap_video::VideoFrame) {
        if let Some(LoadedMedia::Video { texture, .. }) = &mut self.media {
            let image = ColorImage::from_rgba_unmultiplied(
                [frame.width as usize, frame.height as usize],
                &frame.rgba,
            );
            if let Some(handle) = texture {
                handle.set(image, egui::TextureOptions::LINEAR);
            } else {
                *texture = Some(ctx.load_texture(
                    format!(
                        "video-{}",
                        self.current_path
                            .as_ref()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|| self.current_index.to_string())
                    ),
                    image,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }
    }

    fn ensure_video_texture(&mut self, _ctx: &egui::Context) {
        // First frame arrives via VideoEvent from the video thread.
    }

    pub fn toggle_video_playback(&mut self) {
        if matches!(self.media, Some(LoadedMedia::Video { .. })) {
            self.video_engine.toggle();
            self.touch();
        }
    }

    pub fn step_video_frame(&mut self, forward: bool, ctx: &egui::Context) {
        if matches!(self.media, Some(LoadedMedia::Video { .. })) {
            if forward {
                self.video_engine.step_forward();
            } else {
                self.video_engine.step_backward();
            }
            self.touch();
            self.poll_video_events(ctx);
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
                self.settings_dirty = true;
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
        for _ in 0..50 {
            app.poll_loader();
            if matches!(app.media, Some(LoadedMedia::Image { .. })) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(matches!(app.media, Some(LoadedMedia::Image { .. })));
        assert_eq!(app.folder_files.len(), 2);
        app.navigate(1);
        for _ in 0..50 {
            app.poll_loader();
            if app.current_index == 1 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(app.current_index, 1);
    }
}
