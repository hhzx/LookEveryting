//! Application state and media session management.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use cap_core::{classify_extension, load_settings, save_settings, AppSettings, MediaKind};
use cap_i18n::{I18n, Locale};
use cap_image::{DecodedImage, ImageMeta};
use cap_model::{MeshData, ModelInfo};
use cap_ui::layout::{LayoutMode, ViewerMode};
use cap_ui::motion::behavior as motion_behavior;
use cap_video::VideoInfo;
use cap_viewer::{OrbitCamera, SceneSettings};
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
        animation: Option<GifPlayback>,
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
        subtitles: crate::subtitles::Subtitles,
    },
    Model {
        info: ModelInfo,
        path: PathBuf,
        scene: SceneSettings,
        mesh: Option<std::sync::Arc<MeshData>>,
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
    pub window_fit: bool,
    pub zoom_tween: Option<crate::image_viewport::ZoomTween>,
    pub rename_open: bool,
    pub rename_pattern: String,
    pub rename_message: Option<String>,
    pub toast: Option<String>,
    pub error: Option<String>,
    /// Optional recovery actions for the current error (open externally, etc.).
    pub error_actions: Vec<ErrorAction>,
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
    pub image_meta: Option<ImageMeta>,
    pub slideshow_active: bool,
    pub slideshow_interval: Duration,
    pub slideshow_last_tick: Instant,
    /// Previous image held on screen while the next file loads (no black flash).
    pub held_frame: Option<HeldFrame>,
    pub shortcuts_open: bool,
    pub volume: f32,
    pub muted: bool,
    /// Video playback rate (0.5 / 1.0 / 1.5 / 2.0).
    pub playback_rate: f32,
    /// A-B loop markers (seconds). Active when both set and B > A.
    pub ab_a: Option<f32>,
    pub ab_b: Option<f32>,
    pub audio_tracks: Vec<cap_video::AudioTrackInfo>,
    pub audio_track_index: usize,
    /// wgpu mesh PaintCallback path is initialized.
    pub gpu_mesh: bool,
    /// Upload current model mesh to GPU on next draw.
    pub mesh_upload_pending: bool,
    pub drag_hover: bool,
    pub play_flash_until: Option<Instant>,
    pub first_run_hint_shown: bool,
}

/// Texture kept visible during an image-to-image transition.
#[derive(Clone)]
pub struct HeldFrame {
    pub texture: TextureHandle,
    pub size: Vec2,
}

#[derive(Debug, Clone)]
pub enum ErrorAction {
    OpenExternally,
    Dismiss,
}

/// In-memory GIF (or multi-frame) playback state.
pub struct GifPlayback {
    pub frames: Vec<cap_image::AnimFrame>,
    /// Lazily uploaded egui textures, parallel to `frames`.
    pub textures: Vec<Option<TextureHandle>>,
    pub index: usize,
    pub last_tick: Instant,
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
            window_fit: false,
            zoom_tween: None,
            rename_open: false,
            rename_pattern: "{name}".into(),
            rename_message: None,
            toast: None,
            error: None,
            error_actions: Vec::new(),
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
            image_meta: None,
            slideshow_active: false,
            slideshow_interval: Duration::from_secs(3),
            slideshow_last_tick: Instant::now(),
            held_frame: None,
            shortcuts_open: false,
            volume: 1.0,
            muted: false,
            playback_rate: 1.0,
            ab_a: None,
            ab_b: None,
            audio_tracks: Vec::new(),
            audio_track_index: 0,
            gpu_mesh: false,
            mesh_upload_pending: false,
            drag_hover: false,
            play_flash_until: None,
            first_run_hint_shown: false,
        }
    }

    pub fn touch(&mut self) {
        self.last_interaction = Instant::now();
        self.toolbar_visible = true;
    }

    pub fn maybe_hide_toolbar(&mut self, ctx: &egui::Context) {
        if !self.settings.toolbar_auto_hide && !self.is_fullscreen() {
            self.toolbar_visible = true;
            return;
        }
        let screen = ctx.screen_rect();
        let pointer = ctx.input(|i| i.pointer.hover_pos());
        let near_bottom = pointer.is_some_and(|p| {
            p.y >= screen.bottom() - motion_behavior::TOOLBAR_REVEAL_ZONE
        });
        let near_top = pointer.is_some_and(|p| p.y <= screen.top() + 48.0);
        if near_bottom || near_top {
            self.toolbar_visible = true;
            return;
        }
        let hide_ms = motion_behavior::TOOLBAR_AUTO_HIDE_MS as u128;
        if self.is_fullscreen()
            || self.viewer_mode == ViewerMode::Immersive
            || self.last_interaction.elapsed().as_millis() > hide_ms
        {
            self.toolbar_visible = false;
        }
    }

    pub fn set_error(&mut self, message: String, actions: Vec<ErrorAction>) {
        self.error = Some(message);
        self.error_actions = actions;
    }

    pub fn clear_error(&mut self) {
        self.error = None;
        self.error_actions.clear();
    }

    pub fn open_current_externally(&self) {
        if let Some(path) = &self.current_path {
            let _ = open::that(path);
        }
    }

    pub fn stash_held_frame(&mut self) {
        if let Some(LoadedMedia::Image {
            texture: Some(tex),
            width,
            height,
            ..
        }) = &self.media
        {
            self.held_frame = Some(HeldFrame {
                texture: tex.clone(),
                size: Vec2::new(*width as f32, *height as f32),
            });
        } else if let Some(LoadedMedia::Video {
            texture: Some(tex), ..
        }) = &self.media
        {
            let [w, h] = tex.size();
            self.held_frame = Some(HeldFrame {
                texture: tex.clone(),
                size: Vec2::new(w as f32, h as f32),
            });
        }
    }

    pub fn clear_held_frame(&mut self) {
        self.held_frame = None;
    }

    pub fn effective_volume(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            self.volume.clamp(0.0, 1.0)
        }
    }

    pub fn push_volume(&self) {
        self.video_engine.set_volume(self.effective_volume());
    }

    pub fn cycle_playback_rate(&mut self) {
        self.playback_rate = match self.playback_rate {
            r if (r - 0.5).abs() < 0.01 => 1.0,
            r if (r - 1.0).abs() < 0.01 => 1.5,
            r if (r - 1.5).abs() < 0.01 => 2.0,
            _ => 0.5,
        };
        self.video_engine.set_rate(self.playback_rate);
        self.touch();
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        self.push_volume();
        self.touch();
    }

    /// Rotate current image 90° clockwise (in-memory pixel buffer).
    pub fn rotate_image_cw(&mut self) {
        self.ensure_image_rgba_for_edit();
        let Some(LoadedMedia::Image {
            width,
            height,
            rgba,
            texture,
            native_width,
            native_height,
            animation,
            ..
        }) = &mut self.media
        else {
            return;
        };
        // Stop GIF playback when editing pixels.
        *animation = None;
        let Some(src) = rgba.as_ref() else {
            return;
        };
        let (w, h) = (*width as usize, *height as usize);
        let mut dst = vec![0u8; src.len()];
        for y in 0..h {
            for x in 0..w {
                let si = (y * w + x) * 4;
                let dx = h - 1 - y;
                let dy = x;
                let di = (dy * h + dx) * 4;
                dst[di..di + 4].copy_from_slice(&src[si..si + 4]);
            }
        }
        *rgba = Some(dst);
        std::mem::swap(width, height);
        std::mem::swap(native_width, native_height);
        *texture = None;
        self.fit_image();
        self.touch();
    }

    pub fn flip_image(&mut self, horizontal: bool) {
        self.ensure_image_rgba_for_edit();
        let Some(LoadedMedia::Image {
            width,
            height,
            rgba,
            texture,
            animation,
            ..
        }) = &mut self.media
        else {
            return;
        };
        *animation = None;
        let Some(src) = rgba.as_mut() else {
            return;
        };
        let (w, h) = (*width as usize, *height as usize);
        if horizontal {
            for y in 0..h {
                for x in 0..(w / 2) {
                    let a = (y * w + x) * 4;
                    let b = (y * w + (w - 1 - x)) * 4;
                    for i in 0..4 {
                        src.swap(a + i, b + i);
                    }
                }
            }
        } else {
            for y in 0..(h / 2) {
                for x in 0..w {
                    let a = (y * w + x) * 4;
                    let b = ((h - 1 - y) * w + x) * 4;
                    for i in 0..4 {
                        src.swap(a + i, b + i);
                    }
                }
            }
        }
        *texture = None;
        self.touch();
    }

    /// Restore pixel buffer from the current GIF frame when rgba was dropped during playback.
    fn ensure_image_rgba_for_edit(&mut self) {
        let Some(LoadedMedia::Image {
            rgba,
            animation,
            width,
            height,
            ..
        }) = &mut self.media
        else {
            return;
        };
        if rgba.is_some() {
            return;
        }
        let Some(anim) = animation.as_ref() else {
            return;
        };
        let Some(frame) = anim.frames.get(anim.index) else {
            return;
        };
        *width = frame.image.width;
        *height = frame.image.height;
        *rgba = Some(frame.image.rgba.clone());
    }

    pub fn flash_play_pause(&mut self) {
        self.play_flash_until = Some(Instant::now() + Duration::from_millis(400));
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
        self.clear_error();
        self.stash_held_frame();

        if let Some(parent) = path.parent() {
            self.settings.last_directory = Some(parent.to_path_buf());
            self.settings_dirty = true;
            self.refresh_folder(parent, &path);
        }

        self.current_path = Some(path.clone());
        self.load_generation = next_generation();
        self.thumb_queue_cursor = self.current_index;
        self.image_meta = if classify_extension(&path) == Some(MediaKind::Image) {
            cap_image::read_image_meta(&path).ok()
        } else {
            None
        };

        if classify_extension(&path) == Some(MediaKind::Video) {
            self.ab_a = None;
            self.ab_b = None;
            self.audio_tracks = cap_video::AudioDecoder::list_tracks(&path);
            self.audio_track_index = 0;
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
                hw_decode_available: cfg!(windows),
            });
            self.video_engine
                .open(path.clone(), self.settings.prefer_hw_decode);
            self.push_volume();
            self.video_engine.set_rate(self.playback_rate);
            let subtitles = crate::subtitles::Subtitles::load_sidecar(&path).unwrap_or_default();
            self.media = Some(LoadedMedia::Video {
                info,
                path,
                texture: None,
                playing: false,
                ready: false,
                duration_secs: 0.0,
                position_secs: 0.0,
                position_fraction: 0.0,
                subtitles,
            });
            self.viewer_mode = ViewerMode::Viewer;
            self.clear_held_frame();
            return;
        }

        // Hold previous frame while loading — avoids black flash (EX-1).
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
                    animation: None,
                });
                self.viewer_mode = ViewerMode::Viewer;
                self.clear_held_frame();
                return;
            }
            self.apply_image(cached, false, Vec::new());
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
                        self.apply_image(decoded, false, Vec::new());
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
                        if let Ok(LoadedPayload::Image { decoded, .. }) = &result {
                            self.cache_image(path, decoded.clone());
                        }
                        continue;
                    }
                    if current.as_ref().is_none_or(|c| !paths_equal(c, &path)) {
                        if let Ok(LoadedPayload::Image { decoded, .. }) = &result {
                            self.cache_image(path, decoded.clone());
                        }
                        continue;
                    }
                    match result {
                        Ok(payload) => self.apply_payload(path, payload),
                        Err(err) => {
                            self.set_error(
                                err,
                                vec![ErrorAction::OpenExternally, ErrorAction::Dismiss],
                            );
                            self.media = None;
                            self.clear_held_frame();
                        }
                    }
                }
                LoadMessage::Thumbnail { path, decoded } => {
                    let key = crate::thumbnails::thumb_key(&path);
                    self.thumbnails.in_flight.remove(&key);
                    // Prefer an already-seeded/uploaded thumb over a late worker result.
                    if self.thumbnails.textures.contains_key(&key)
                        || self.thumbnails.pending.contains_key(&key)
                    {
                        continue;
                    }
                    match decoded {
                        Some(decoded) => {
                            self.thumbnails.failed.remove(&key);
                            self.thumbnails.pending.insert(key, decoded);
                        }
                        None => {
                            self.thumbnails.failed.insert(key);
                        }
                    }
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
                    has_audio: _,
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
                    self.set_error(
                        err,
                        vec![ErrorAction::OpenExternally, ErrorAction::Dismiss],
                    );
                    self.media = None;
                    self.clear_held_frame();
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
        for delta in [1isize, -1, 2, -2] {
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
            LoadedPayload::Image { decoded, animation } => {
                self.cache_image(path, decoded.clone());
                let upgrade = matches!(self.media, Some(LoadedMedia::Image { .. }));
                self.apply_image(decoded, upgrade, animation);
            }
            LoadedPayload::Model {
                info,
                mesh,
                camera,
            } => {
                if mesh.is_none() {
                    self.set_error(
                        self.i18n.t("toast-open-failed").to_string(),
                        vec![ErrorAction::OpenExternally, ErrorAction::Dismiss],
                    );
                }
                self.media = Some(LoadedMedia::Model {
                    info,
                    path,
                    scene: SceneSettings::default(),
                    mesh,
                    camera,
                });
                self.mesh_upload_pending = true;
                self.viewer_mode = ViewerMode::Viewer;
                self.clear_held_frame();
            }
        }
    }

    fn cache_image(&mut self, path: PathBuf, decoded: DecodedImage) {
        crate::thumbnails::seed_from_decoded(&mut self.thumbnails, &path, &decoded);
        for evicted in self.image_cache.insert(path.clone(), decoded) {
            let key = evicted.to_string_lossy().to_string();
            self.texture_cache.remove(&key);
        }
    }

    fn apply_image(
        &mut self,
        decoded: DecodedImage,
        upgrade: bool,
        animation: Vec<cap_image::AnimFrame>,
    ) {
        if let Some(path) = self.current_path.clone() {
            crate::thumbnails::seed_from_decoded(&mut self.thumbnails, &path, &decoded);
        }
        let anim = if animation.len() > 1 {
            let n = animation.len();
            Some(GifPlayback {
                frames: animation,
                textures: vec![None; n],
                index: 0,
                last_tick: Instant::now(),
            })
        } else {
            None
        };
        if upgrade {
            if let Some(LoadedMedia::Image {
                width,
                height,
                native_width,
                native_height,
                full_res_loading,
                rgba,
                texture,
                animation: slot,
            }) = &mut self.media
            {
                *width = decoded.width;
                *height = decoded.height;
                *native_width = decoded.native_width;
                *native_height = decoded.native_height;
                *full_res_loading = false;
                *rgba = Some(decoded.rgba);
                *texture = None;
                if anim.is_some() {
                    *slot = anim;
                }
            }
            self.clear_held_frame();
            return;
        }
        self.media = Some(LoadedMedia::Image {
            width: decoded.width,
            height: decoded.height,
            native_width: decoded.native_width,
            native_height: decoded.native_height,
            full_res_loading: false,
            rgba: Some(decoded.rgba),
            texture: None,
            animation: anim,
        });
        self.fit_mode = true;
        self.zoom = 1.0;
        self.pan = Vec2::ZERO;
        self.viewer_mode = ViewerMode::Viewer;
        self.clear_held_frame();
    }

    /// Advance GIF / multi-frame animation when present.
    pub fn tick_animation(&mut self, ctx: &egui::Context) {
        let Some(LoadedMedia::Image {
            animation: Some(anim),
            width,
            height,
            rgba,
            texture,
            ..
        }) = &mut self.media
        else {
            return;
        };
        if anim.frames.len() < 2 {
            return;
        }
        let delay = Duration::from_millis(anim.frames[anim.index].delay_ms.max(20) as u64);
        if anim.last_tick.elapsed() < delay {
            ctx.request_repaint_after(delay.saturating_sub(anim.last_tick.elapsed()));
            return;
        }
        // Catch up if we fell behind (avoid accumulating lag).
        let steps = (anim.last_tick.elapsed().as_millis() / delay.as_millis().max(1)).max(1) as usize;
        anim.last_tick = Instant::now();
        anim.index = (anim.index + steps) % anim.frames.len();
        let idx = anim.index;
        let frame = &anim.frames[idx];
        *width = frame.image.width;
        *height = frame.image.height;

        if anim.textures.get(idx).and_then(|t| t.as_ref()).is_none() {
            let image = ColorImage::from_rgba_unmultiplied(
                [frame.image.width as usize, frame.image.height as usize],
                &frame.image.rgba,
            );
            let handle = ctx.load_texture(
                format!("gif-{}-{idx}", self.current_index),
                image,
                egui::TextureOptions::LINEAR,
            );
            if let Some(slot) = anim.textures.get_mut(idx) {
                *slot = Some(handle);
            }
        }
        if let Some(Some(tex)) = anim.textures.get(idx) {
            *texture = Some(tex.clone());
            // Keep rgba for rotate/flip; update to current frame cheaply via Arc would be better,
            // but clone only when missing — skip while animating.
            *rgba = None;
        } else {
            *rgba = Some(frame.image.rgba.clone());
            *texture = None;
        }
        ctx.request_repaint();
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

    pub fn seek_video_by(&mut self, delta_secs: f32, ctx: &egui::Context) {
        if matches!(self.media, Some(LoadedMedia::Video { .. })) {
            self.video_engine.seek_relative(delta_secs);
            self.touch();
            self.poll_video_events(ctx);
        }
    }

    pub fn reset_model_camera(&mut self) {
        if let Some(LoadedMedia::Model { mesh, camera, .. }) = &mut self.media {
            *camera = match mesh.as_ref() {
                Some(m) => OrbitCamera::fit_bounds(&m.bounds),
                None => OrbitCamera::default(),
            };
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

    pub fn toggle_slideshow(&mut self) {
        self.slideshow_active = !self.slideshow_active;
        self.slideshow_last_tick = Instant::now();
        self.touch();
    }

    pub fn tick_slideshow(&mut self) {
        if !self.slideshow_active {
            return;
        }
        if self.folder_files.len() < 2 {
            return;
        }
        if self.slideshow_last_tick.elapsed() >= self.slideshow_interval {
            self.slideshow_last_tick = Instant::now();
            self.navigate(1);
        }
    }

    pub fn apply_batch_rename(&mut self) {
        self.rename_message = None;
        let pattern = self.rename_pattern.clone();
        if pattern.trim().is_empty() {
            self.rename_message = Some("Pattern is empty".into());
            return;
        }
        let files = self.folder_files.clone();
        let mut renamed = 0usize;
        let mut errors = 0usize;
        for (i, old) in files.iter().enumerate() {
            let stem = old
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("file");
            let ext = old
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{e}"))
                .unwrap_or_default();
            let new_stem = pattern
                .replace("{n}", &(i + 1).to_string())
                .replace("{name}", stem)
                .replace("{i}", &format!("{i:03}"));
            let new_name = format!("{new_stem}{ext}");
            let new_path = old.with_file_name(new_name);
            if new_path == *old {
                continue;
            }
            if new_path.exists() {
                errors += 1;
                continue;
            }
            match std::fs::rename(old, &new_path) {
                Ok(()) => {
                    renamed += 1;
                    if let Some(slot) = self.folder_files.get_mut(i) {
                        *slot = new_path.clone();
                    }
                    if self.current_path.as_ref() == Some(old) {
                        self.current_path = Some(new_path);
                    }
                }
                Err(_) => errors += 1,
            }
        }
        self.thumbnails.clear();
        self.rename_message = Some(format!("Renamed {renamed}, errors {errors}"));
        self.touch();
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
                rgba
                    .as_ref()
                    .map(|pixels| (*width as usize, *height as usize, pixels.clone()))
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
            let filter = if self.prefer_nearest_filter() {
                egui::TextureOptions::NEAREST
            } else {
                egui::TextureOptions::LINEAR
            };
            let handle = ctx.load_texture(path_key.clone(), image, filter);
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
        self.enforce_ab_loop(ctx);
    }

    fn enforce_ab_loop(&mut self, ctx: &egui::Context) {
        let Some((a, b)) = self.ab_a.zip(self.ab_b) else {
            return;
        };
        if b <= a {
            return;
        }
        let Some(LoadedMedia::Video {
            playing: true,
            position_secs,
            duration_secs,
            ..
        }) = &self.media
        else {
            return;
        };
        let duration = *duration_secs;
        if duration <= 0.0 {
            return;
        }
        if *position_secs >= b - 0.02 {
            let frac = (a / duration).clamp(0.0, 1.0);
            self.video_engine.seek_resume(frac);
            self.poll_video_events(ctx);
        }
    }

    pub fn mark_ab_a(&mut self) {
        if let Some(LoadedMedia::Video { position_secs, .. }) = &self.media {
            self.ab_a = Some(*position_secs);
            if self.ab_b.is_some_and(|b| b <= *position_secs) {
                self.ab_b = None;
            }
            self.touch();
        }
    }

    pub fn mark_ab_b(&mut self) {
        if let Some(LoadedMedia::Video { position_secs, .. }) = &self.media {
            let pos = *position_secs;
            if self.ab_a.is_none_or(|a| a >= pos) {
                self.ab_a = Some((pos - 1.0).max(0.0));
            }
            self.ab_b = Some(pos);
            self.touch();
        }
    }

    pub fn clear_ab_loop(&mut self) {
        self.ab_a = None;
        self.ab_b = None;
        self.touch();
    }

    pub fn cycle_audio_track(&mut self) {
        if self.audio_tracks.len() < 2 {
            return;
        }
        self.audio_track_index = (self.audio_track_index + 1) % self.audio_tracks.len();
        self.video_engine.set_audio_track(self.audio_track_index);
        self.touch();
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
            self.flash_play_pause();
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
