//! Reusable egui widgets styled with design tokens.

use egui::{Color32, Frame, Margin, Response, RichText, Ui, Vec2};

use crate::colors::{Palette, Semantic};
use crate::spacing::{component, radius, space};

/// Standard inner padding for chrome panels (sidebar, title bar, info).
pub fn panel_margin() -> Margin {
    Margin::symmetric(component::PANEL_PADDING as i8, space::S2 as i8)
}

/// Panel frame with consistent edge insets.
pub fn panel_frame(fill: Color32) -> Frame {
    Frame::NONE.fill(fill).inner_margin(panel_margin())
}

/// Title bar frame — horizontal inset only, content is vertically centered.
pub fn titlebar_frame(fill: Color32) -> Frame {
    Frame::NONE.fill(fill).inner_margin(Margin {
        left: component::PANEL_PADDING as i8,
        right: component::PANEL_PADDING as i8,
        top: 0,
        bottom: 0,
    })
}

/// Icon-only button used in toolbars.
pub fn icon_button(ui: &mut Ui, label: &str, tooltip: &str) -> Response {
    let size = Vec2::splat(component::ICON_BUTTON);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let bg = if response.hovered() {
            Palette::SURFACE_RAISED
        } else {
            Color32::TRANSPARENT
        };
        if response.is_pointer_button_down_on() {
            ui.painter()
                .rect_filled(rect, radius::MD, Palette::SURFACE_OVERLAY);
        } else {
            ui.painter().rect_filled(rect, radius::MD, bg);
        }
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(14.0),
            if response.hovered() {
                Semantic::FG_PRIMARY
            } else {
                Semantic::FG_SECONDARY
            },
        );
    }
    response.on_hover_text(tooltip)
}

/// Primary action button.
pub fn primary_button(ui: &mut Ui, text: &str) -> Response {
    let text = RichText::new(text).color(Semantic::FG_PRIMARY);
    ui.add(
        egui::Button::new(text)
            .fill(Palette::ACCENT)
            .min_size(Vec2::new(96.0, component::INPUT_HEIGHT - 4.0)),
    )
}

/// Ghost button for secondary actions.
pub fn ghost_button(ui: &mut Ui, text: &str) -> Response {
    ui.add(
        egui::Button::new(RichText::new(text).color(Semantic::FG_SECONDARY))
            .frame(false)
            .min_size(Vec2::new(0.0, component::ICON_BUTTON)),
    )
}

/// Floating panel background painter helper.
pub fn paint_floating_panel(ui: &Ui, rect: egui::Rect) {
    ui.painter().rect_filled(rect, radius::XL, Palette::TOOLBAR);
    ui.painter().rect_stroke(
        rect,
        radius::XL,
        egui::Stroke::new(1.0_f32, Palette::BORDER_SUBTLE),
        egui::StrokeKind::Inside,
    );
}
