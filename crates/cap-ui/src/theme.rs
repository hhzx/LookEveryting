//! Applies the LookEveryting dark minimal theme to an egui context.

use egui::{CornerRadius, Stroke, Style, Visuals};

use crate::colors::{Palette, Semantic};
use crate::spacing::radius;
use crate::typography;

/// Complete theme bundle.
pub struct Theme {
    pub density: crate::layout::Density,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            density: crate::layout::Density::Comfortable,
        }
    }
}

impl Theme {
    pub fn dark() -> Self {
        Self::default()
    }

    /// Install theme into egui context (call once at startup, re-call on theme change).
    pub fn install(&self, ctx: &egui::Context) {
        let mut style = Style::default();
        style.visuals = dark_visuals();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 6.0);
        style.spacing.window_margin = egui::Margin::same(12);
        style.spacing.indent = 16.0;
        style.text_styles = typography::text_styles().into_iter().collect();
        ctx.set_style(style);
    }

    pub fn effective_scale(&self, system_ppp: f32) -> f32 {
        (system_ppp * self.density.scale()).clamp(0.85, 1.5)
    }
}

fn dark_visuals() -> Visuals {
    let mut v = Visuals::dark();
    v.dark_mode = true;

    // Backgrounds
    v.panel_fill = Semantic::BG_PANEL;
    v.window_fill = Semantic::BG_ELEVATED;
    v.extreme_bg_color = Semantic::BG_VIEWPORT;
    v.faint_bg_color = Palette::SURFACE_RAISED;

    // Widgets
    v.widgets.noninteractive.bg_fill = Palette::SURFACE;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, Semantic::FG_SECONDARY);
    v.widgets.inactive.bg_fill = Palette::SURFACE_RAISED;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, Semantic::FG_SECONDARY);
    v.widgets.hovered.bg_fill = Palette::SURFACE_OVERLAY;
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Semantic::FG_PRIMARY);
    v.widgets.active.bg_fill = Palette::ACCENT_MUTED;
    v.widgets.active.fg_stroke = Stroke::new(1.0_f32, Palette::ACCENT);
    v.widgets.open.bg_fill = Palette::ACCENT_MUTED;

    // Selection
    v.selection.bg_fill = Palette::ACCENT_MUTED;
    v.selection.stroke = Stroke::new(1.0_f32, Palette::ACCENT);

    // Window
    v.window_corner_radius = CornerRadius::same(radius::LG as u8);
    v.window_shadow = egui::epaint::Shadow {
        offset: [0, 4],
        blur: 12,
        spread: 0,
        color: egui::Color32::from_rgba_premultiplied(0, 0, 0, 128),
    };

    // Hyperlinks
    v.hyperlink_color = Palette::ACCENT;

    // Stroke
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Palette::BORDER_SUBTLE);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Palette::BORDER_SUBTLE);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Palette::BORDER_DEFAULT);

    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn density_scale_bounds() {
        let theme = Theme::dark();
        assert!((theme.effective_scale(1.0) - 1.0).abs() < f32::EPSILON);
        assert!(theme.effective_scale(2.0) <= 1.5);
        assert!(theme.effective_scale(0.5) >= 0.85);
    }
}
