//! Image viewport: wheel zoom, middle-button pan, drag pan when zoomed.

use egui::{PointerButton, Rect, Response, Sense, Ui, Vec2};

use crate::app::LookApp;

const ZOOM_STEP: f32 = 1.12;
const MIN_ZOOM: f32 = 0.05;
const MAX_ZOOM: f32 = 32.0;

impl LookApp {
    pub fn fit_scale(&self, avail: Vec2, img_size: Vec2) -> f32 {
        if img_size.x <= 0.0 || img_size.y <= 0.0 {
            return 1.0;
        }
        (avail.x / img_size.x).min(avail.y / img_size.y)
    }

    pub fn display_scale(&self, avail: Vec2, img_size: Vec2) -> f32 {
        if self.fit_mode {
            self.fit_scale(avail, img_size)
        } else {
            self.zoom
        }
    }

    pub fn fit_image(&mut self) {
        self.fit_mode = true;
        self.zoom = 1.0;
        self.pan = Vec2::ZERO;
    }

    pub fn actual_size_image(&mut self) {
        self.fit_mode = false;
        self.zoom = 1.0;
        self.pan = Vec2::ZERO;
    }

    pub fn reset_image_view(&mut self) {
        self.fit_image();
    }

    pub fn zoom_image(&mut self, avail: Vec2, img_size: Vec2, factor: f32) {
        if self.fit_mode {
            self.fit_mode = false;
            self.zoom = self.fit_scale(avail, img_size);
        }
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
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
    }

    pub fn ensure_zoomed_for_pan(&mut self, avail: Vec2, img_size: Vec2) {
        if self.fit_mode {
            self.fit_mode = false;
            self.zoom = self.fit_scale(avail, img_size);
        }
    }
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
        let scroll = raw * 1.15 + smooth * 0.5;
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

    response
}
