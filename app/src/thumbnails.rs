//! Thumbnail cache and filmstrip rendering.

use std::collections::HashMap;
use std::path::Path;

use cap_core::{classify_extension, MediaKind};
use cap_image::DecodedImage;
use cap_ui::colors::{Palette, Semantic};
use cap_ui::spacing::component;
use egui::{ColorImage, TextureHandle, Ui, Vec2};

use crate::app::LookApp;

const THUMBS_PER_FRAME: usize = 2;

pub struct ThumbnailCache {
    pub textures: HashMap<String, TextureHandle>,
    pub pending: HashMap<String, DecodedImage>,
}

impl ThumbnailCache {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
            pending: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.textures.clear();
        self.pending.clear();
    }
}

impl Default for ThumbnailCache {
    fn default() -> Self {
        Self::new()
    }
}

pub fn ensure_thumbnails(app: &mut LookApp, ctx: &egui::Context) {
    // Upload decoded thumbs from background thread.
    let keys: Vec<String> = app.thumbnails.pending.keys().cloned().collect();
    for key in keys.into_iter().take(THUMBS_PER_FRAME) {
        if let Some(decoded) = app.thumbnails.pending.remove(&key) {
            let image = ColorImage::from_rgba_unmultiplied(
                [decoded.width as usize, decoded.height as usize],
                &decoded.rgba,
            );
            let handle = ctx.load_texture(
                format!("thumb-{key}"),
                image,
                egui::TextureOptions::LINEAR,
            );
            app.thumbnails.textures.insert(key, handle);
        }
    }

    // Queue background thumbnail jobs — current file and neighbors first.
    let len = app.folder_files.len();
    if len == 0 {
        return;
    }
    let cur = app.current_index;
    let mut order = Vec::with_capacity(len);
    order.push(cur);
    for d in 1..len {
        order.push((cur + d) % len);
        if order.len() >= len {
            break;
        }
        order.push((cur + len - d) % len);
        if order.len() >= len {
            break;
        }
    }

    let mut queued = 0usize;
    for idx in order {
        if queued >= THUMBS_PER_FRAME {
            break;
        }
        let path = &app.folder_files[idx];
        let key = path.to_string_lossy().to_string();
        if app.thumbnails.textures.contains_key(&key)
            || app.thumbnails.pending.contains_key(&key)
        {
            continue;
        }
        if classify_extension(path) == Some(MediaKind::Image) {
            app.loader.request_thumbnail(path.clone());
            queued += 1;
        }
    }
}

pub fn draw_thumbnail_strip(app: &mut LookApp, ui: &mut Ui) {
    if app.folder_files.is_empty() {
        return;
    }

    let thumb = component::THUMBNAIL_STRIP_SIZE;
    let gap = component::THUMBNAIL_GAP;
    let cell = Vec2::new(thumb, thumb);

    let mut open_index = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for (idx, path) in app.folder_files.iter().enumerate() {
            let selected = idx == app.current_index;
            let key = path.to_string_lossy().to_string();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?");

            let (cell_rect, response) = ui.allocate_exact_size(cell, egui::Sense::click());

            if ui.is_rect_visible(cell_rect) {
                let bg = if selected {
                    Palette::ACCENT_MUTED
                } else {
                    Palette::SURFACE_RAISED
                };
                let stroke = egui::Stroke::new(
                    if selected { 2.0_f32 } else { 1.0_f32 },
                    if selected {
                        Palette::ACCENT
                    } else {
                        Palette::BORDER_SUBTLE
                    },
                );
                ui.painter()
                    .rect(cell_rect, 4.0_f32, bg, stroke, egui::StrokeKind::Inside);

                let inner = cell_rect.shrink(3.0);
                if let Some(tex) = app.thumbnails.textures.get(&key) {
                    paint_cover_image(ui, inner, tex.id(), tex.size());
                } else {
                    paint_kind_placeholder(ui, inner, path);
                }
            }

            if response.clicked() {
                open_index = Some(idx);
            }
            response.on_hover_text(name);
        }
    });

    if let Some(idx) = open_index {
        app.navigate_to_index(idx);
    }

    scroll_to_selected(app, ui);
}

fn paint_cover_image(
    ui: &Ui,
    rect: egui::Rect,
    tex_id: egui::TextureId,
    tex_size: [usize; 2],
) {
    let [tw, th] = tex_size;
    if tw == 0 || th == 0 {
        return;
    }
    let tex_aspect = tw as f32 / th as f32;
    let rect_aspect = rect.width() / rect.height().max(1.0);
    let uv = if tex_aspect > rect_aspect {
        let visible = rect_aspect / tex_aspect;
        let pad = (1.0 - visible) * 0.5;
        egui::Rect::from_min_max(egui::pos2(pad, 0.0), egui::pos2(1.0 - pad, 1.0))
    } else {
        let visible = tex_aspect / rect_aspect;
        let pad = (1.0 - visible) * 0.5;
        egui::Rect::from_min_max(egui::pos2(0.0, pad), egui::pos2(1.0, 1.0 - pad))
    };
    ui.painter().with_clip_rect(rect).image(tex_id, rect, uv, egui::Color32::WHITE);
}

fn paint_kind_placeholder(ui: &Ui, rect: egui::Rect, path: &Path) {
    let (label, color) = match classify_extension(path) {
        Some(MediaKind::Video) => ("VID", Palette::ACCENT),
        Some(MediaKind::Model) => ("3D", color32_model()),
        Some(MediaKind::Image) => ("IMG", Semantic::FG_MUTED),
        _ => ("?", Semantic::FG_MUTED),
    };
    let center = rect.center();
    ui.painter().text(
        center + Vec2::new(0.0, -6.0),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(14.0),
        color,
    );
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        ui.painter().text(
            center + Vec2::new(0.0, 10.0),
            egui::Align2::CENTER_CENTER,
            ext.to_ascii_uppercase(),
            egui::FontId::proportional(9.0),
            Semantic::FG_MUTED,
        );
    }
}

fn color32_model() -> egui::Color32 {
    egui::Color32::from_rgb(0x22, 0xC5, 0x5E)
}

fn scroll_to_selected(app: &mut LookApp, ui: &Ui) {
    if app.folder_files.is_empty() {
        return;
    }
    if app.thumb_scroll_synced_index == Some(app.current_index) {
        return;
    }
    let thumb = component::THUMBNAIL_STRIP_SIZE + component::THUMBNAIL_GAP;
    let offset = thumb * app.current_index as f32;
    ui.scroll_to_rect(
        egui::Rect::from_min_size(
            egui::pos2(offset, 0.0),
            Vec2::new(thumb, component::THUMBNAIL_STRIP_HEIGHT),
        ),
        Some(egui::Align::Center),
    );
    app.thumb_scroll_synced_index = Some(app.current_index);
}
