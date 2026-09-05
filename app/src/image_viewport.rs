//! Image viewport: wheel zoom, middle-button pan, drag pan when zoomed.

use std::time::{Duration, Instant};

use egui::{PointerButton, Rect, Response, Sense, Ui, Vec2};

use crate::app::LookApp;

const _ZOOM_STEP: f32 = 1.12;
const MIN_ZOOM: f32 = 0.05;
const MAX_ZOOM: f32 = 32.0;
const TWEEN_MS: u64 = 120;

impl LookApp {
    pub fn fit_scale(&self, avail: Vec2, img_size: Vec2) -> f32 {
        if img_size.x <= 0.0 || img_size.y <= 0.0 {
            return 1.0;
        }
        (avail.x / img_size.x).min(avail.y / img_size.y)
    }

    pub fn display_scale(&self, avail: Vec2, img_size: Vec2) -> f32 {
        let target = if self.fit_mode {
            self.fit_scale(avail, img_size)
        } else {
            self.zoom
        };
        if let Some(tween) = &self.zoom_tween {
            let t = tween
                .started
                .elapsed()
                .as_secs_f32()
                / (TWEEN_MS as f32 / 1000.0);
            if t >= 1.0 {
                return target;
            }
            let ease = 1.0 - (1.0 - t).powi(3);
            return tween.from + (tween.to - tween.from) * ease;
        }
        target
    }

    pub fn prefer_nearest_filter(&self) -> bool {
        !self.fit_mode && (self.zoom - 1.0).abs() < 0.02
    }

    pub fn fit_image(&mut self) {
        let from = self.zoom.max(0.01);
        self.fit_mode = true;
        self.zoom = 1.0;
        self.pan = Vec2::ZERO;
        self.zoom_tween = Some(ZoomTween {
            from,
            to: 1.0,
            started: Instant::now(),
        });
        self.window_fit = false;
    }

    pub fn actual_size_image(&mut self) {
        let from = if self.fit_mode { 0.5 } else { self.zoom };
        self.fit_mode = false;
        self.zoom = 1.0;
        self.pan = Vec2::ZERO;
        self.zoom_tween = Some(ZoomTween {
            from,
            to: 1.0,
            started: Instant::now(),
        });
        self.window_fit = false;
        self.request_full_res_if_needed();
    }

    pub fn reset_image_view(&mut self) {
        self.fit_image();
    }

    /// Resize the OS window to the image aspect (ImageGlass-style Window Fit).
    pub fn toggle_window_fit(&mut self, ctx: &egui::Context) {
        self.window_fit = !self.window_fit;
        if !self.window_fit {
            self.touch();
            return;
        }
        let Some(size) = self.media.as_ref().and_then(|m| m.image_size()) else {
            self.window_fit = false;
            return;
        };
        self.apply_window_fit_size(ctx, size);
        self.fit_image();
        self.touch();
    }

    pub fn apply_window_fit_size(&self, ctx: &egui::Context, img_size: Vec2) {
        if img_size.x <= 0.0 || img_size.y <= 0.0 {
            return;
        }
        let screen = ctx.input(|i| i.viewport().monitor_size).unwrap_or(Vec2::new(1920.0, 1080.0));
        let max_w = (screen.x * 0.9).max(480.0);
        let max_h = (screen.y * 0.85).max(360.0);
        let chrome_h = 160.0_f32; // title + toolbar + strip approx
        let chrome_w = 40.0_f32;
        let scale = ((max_w - chrome_w) / img_size.x)
            .min((max_h - chrome_h) / img_size.y)
            .clamp(0.05, 8.0);
        let inner = Vec2::new(
            (img_size.x * scale + chrome_w).clamp(480.0, max_w),
            (img_size.y * scale + chrome_h).clamp(360.0, max_h),
        );
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(inner));
    }

    pub fn tick_zoom_tween(&mut self) {
        if let Some(tween) = &self.zoom_tween {
            if tween.started.elapsed() >= Duration::from_millis(TWEEN_MS) {
                self.zoom_tween = None;
            }
        }
    }

    pub fn zoom_image(&mut self, avail: Vec2, img_size: Vec2, factor: f32) {
        if self.fit_mode {
            self.fit_mode = false;
            self.zoom = self.fit_scale(avail, img_size);
        }
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        self.zoom_tween = None;
        self.window_fit = false;
    }

    pub fn zoom_image_at(&mut self, avail: Vec2, img_size: Vec2, pointer: Vec2, center: Vec2, factor: f32) {
        if self.fit_mode {
            self.fit_mode = false;
            self.zoom = self.fit_scale(avail, img_size);
        }
        let new = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        if (new - self.zoom).abs() < f32::EPSILON {
            return;
        }
        let ratio = new / self.zoom;
        self.zoom = new;
        self.pan = pointer - center - (pointer - center - self.pan) * ratio;
        self.zoom_tween = None;
        self.window_fit = false;
    }

    pub fn ensure_zoomed_for_pan(&mut self, avail: Vec2, img_size: Vec2) {
        if self.fit_mode {
            self.fit_mode = false;
            self.zoom = self.fit_scale(avail, img_size);
        }
        self.window_fit = false;
    }
}

#[derive(Clone)]
pub struct ZoomTween {
    pub from: f32,
    pub to: f32,
    pub started: Instant,
}

/// Allocate the viewport rect and apply zoom / pan interactions.
pub fn interact_image_viewport(app: &mut LookApp, ui: &mut Ui, rect: Rect, img_size: Vec2) -> Response {
    let id = ui.id().with("image_viewport");
    let response = ui.interact(rect, id, Sense::click_and_drag());

    let avail = rect.size();
    let center = rect.center();
    let active = response.hovered() || response.dragged();

    if active {
        ui.ctx().request_repaint();
        let (raw, smooth) = ui.input(|i| (i.raw_scroll_delta.y, i.smooth_scroll_delta.y));
        // egui: positive Y = scroll up; apps expect that to zoom in → invert legacy sign.
        let scroll = -(raw * 1.15 + smooth * 0.5);
        if scroll.abs() > f32::EPSILON {
            let factor = (1.0 + scroll * 0.0028).clamp(0.90, 1.10);
            let pointer = ui
                .input(|i| i.pointer.hover_pos())
                .unwrap_or(center)
                .to_vec2();
            app.zoom_image_at(avail, img_size, pointer, center.to_vec2(), factor);
            app.touch();
            ui.ctx().request_repaint();
        }
    }

    if response.hovered() {
        let delta = ui.input(|i| i.pointer.delta());
        let middle = ui.input(|i| i.pointer.button_down(PointerButton::Middle));
        if middle && delta != Vec2::ZERO {
            app.ensure_zoomed_for_pan(avail, img_size);
            app.pan += delta;
            app.touch();
            ui.ctx().request_repaint();
        }
    }

    if response.dragged() {
        let middle = ui.input(|i| i.pointer.button_down(PointerButton::Middle));
        let left = ui.input(|i| i.pointer.button_down(PointerButton::Primary));
        if middle || left {
            app.ensure_zoomed_for_pan(avail, img_size);
            app.pan += response.drag_delta();
            app.touch();
            ui.ctx().request_repaint();
        }
    }

    if response.double_clicked() {
        app.fit_image();
        app.touch();
    }

    if app.zoom_tween.is_some() {
        ui.ctx().request_repaint();
    }

    response
}
