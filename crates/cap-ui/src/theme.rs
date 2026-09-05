//! Applies the LookEveryting theme to an egui context.

use cap_core::ThemePreference;
use egui::{CornerRadius, Stroke, Style, Visuals};

use crate::colors::{LightPalette, Palette, Semantic};
use crate::spacing::radius;
use crate::typography;

/// Complete theme bundle.
pub struct Theme {
    pub density: crate::layout::Density,
    /// `true` = dark visuals, `false` = light visuals.
    pub dark: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            density: crate::layout::Density::Comfortable,
            dark: true,
        }
    }

    pub fn light() -> Self {
        Self {
            density: crate::layout::Density::Comfortable,
            dark: false,
        }
    }

    /// Resolve a stored preference into concrete dark/light visuals.
    pub fn from_preference(pref: ThemePreference, system_dark: bool) -> Self {
        let dark = match pref {
            ThemePreference::Dark => true,
            ThemePreference::Light => false,
            ThemePreference::System => system_dark,
        };
        if dark {
            Self::dark()
        } else {
            Self::light()
        }
    }

    /// Install theme into egui context (call once at startup, re-call on theme change).
    pub fn install(&self, ctx: &egui::Context) {
        let mut style = Style::default();
        style.visuals = if self.dark {
            dark_visuals()
        } else {
            light_visuals()
        };
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

fn light_visuals() -> Visuals {
    let mut v = Visuals::light();
    v.dark_mode = false;

    // Backgrounds — light surfaces with black media viewport
    v.panel_fill = LightPalette::SURFACE;
    v.window_fill = LightPalette::SURFACE_RAISED;
    v.extreme_bg_color = LightPalette::VIEWPORT;
    v.faint_bg_color = LightPalette::SURFACE_OVERLAY;

    // Widgets
    v.widgets.noninteractive.bg_fill = LightPalette::SURFACE;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, LightPalette::TEXT_SECONDARY);
    v.widgets.inactive.bg_fill = LightPalette::SURFACE_RAISED;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, LightPalette::TEXT_SECONDARY);
    v.widgets.hovered.bg_fill = LightPalette::SURFACE_OVERLAY;
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, LightPalette::TEXT_PRIMARY);
    v.widgets.active.bg_fill = LightPalette::ACCENT_MUTED;
    v.widgets.active.fg_stroke = Stroke::new(1.0_f32, LightPalette::ACCENT);
    v.widgets.open.bg_fill = LightPalette::ACCENT_MUTED;

    // Selection — same accent as dark theme
    v.selection.bg_fill = LightPalette::ACCENT_MUTED;
    v.selection.stroke = Stroke::new(1.0_f32, LightPalette::ACCENT);

    // Window
    v.window_corner_radius = CornerRadius::same(radius::LG as u8);
    v.window_shadow = egui::epaint::Shadow {
        offset: [0, 4],
        blur: 12,
        spread: 0,
        color: egui::Color32::from_rgba_premultiplied(0, 0, 0, 64),
    };

    // Hyperlinks / accent match dark theme
    v.hyperlink_color = LightPalette::ACCENT;

    // Stroke
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, LightPalette::BORDER_SUBTLE);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, LightPalette::BORDER_SUBTLE);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, LightPalette::BORDER_DEFAULT);

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

    #[test]
    fn from_preference_resolves() {
        assert!(Theme::from_preference(ThemePreference::Dark, false).dark);
        assert!(!Theme::from_preference(ThemePreference::Light, true).dark);
        assert!(Theme::from_preference(ThemePreference::System, true).dark);
        assert!(!Theme::from_preference(ThemePreference::System, false).dark);
    }
}
