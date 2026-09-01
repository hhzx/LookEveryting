//! Typography tokens.

/// Font size scale (logical points).
pub mod size {
    pub const XS: f32 = 11.0;
    pub const SM: f32 = 13.0;
    pub const BASE: f32 = 15.0;
    pub const LG: f32 = 18.0;
    pub const XL: f32 = 22.0;
    pub const XXL: f32 = 28.0;
}

/// Font family names (resolved at runtime via egui FontDefinitions).
pub mod family {
    pub const SANS: &str = "sans";
    pub const MONO: &str = "mono";
}

/// Pre-built text styles for egui.
pub fn text_styles() -> Vec<(egui::TextStyle, egui::FontId)> {
    use egui::{FontId, TextStyle};
    vec![
        (
            TextStyle::Heading,
            FontId::new(size::LG, egui::FontFamily::Proportional),
        ),
        (
            TextStyle::Body,
            FontId::new(size::BASE, egui::FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(size::SM, egui::FontFamily::Monospace),
        ),
        (
            TextStyle::Button,
            FontId::new(size::SM, egui::FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(size::SM, egui::FontFamily::Proportional),
        ),
    ]
}
