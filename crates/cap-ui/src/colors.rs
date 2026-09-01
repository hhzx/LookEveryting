//! Color palette and semantic colors — dark minimal theme.

use egui::Color32;

/// Raw palette tokens from `design/tokens/colors.json`.
pub struct Palette;

impl Palette {
    pub const CANVAS: Color32 = Color32::from_rgb(0x0A, 0x0A, 0x0B);
    pub const SURFACE: Color32 = Color32::from_rgb(0x11, 0x11, 0x13);
    pub const SURFACE_RAISED: Color32 = Color32::from_rgb(0x18, 0x18, 0x1B);
    pub const SURFACE_OVERLAY: Color32 = Color32::from_rgb(0x1F, 0x1F, 0x23);
    pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(0x27, 0x27, 0x2A);
    pub const BORDER_DEFAULT: Color32 = Color32::from_rgb(0x3F, 0x3F, 0x46);
    pub const BORDER_STRONG: Color32 = Color32::from_rgb(0x52, 0x52, 0x5B);

    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xFA, 0xFA, 0xFA);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0xA1, 0xA1, 0xAA);
    pub const TEXT_TERTIARY: Color32 = Color32::from_rgb(0x71, 0x71, 0x7A);
    pub const TEXT_DISABLED: Color32 = Color32::from_rgb(0x52, 0x52, 0x5B);

    pub const ACCENT: Color32 = Color32::from_rgb(0x3B, 0x82, 0xF6);
    pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0x60, 0xA5, 0xFA);
    pub const ACCENT_MUTED: Color32 = Color32::from_rgb(0x1E, 0x3A, 0x5F);
    pub const ACCENT_SUBTLE: Color32 = Color32::from_rgb(0x17, 0x25, 0x54);

    pub const SUCCESS: Color32 = Color32::from_rgb(0x22, 0xC5, 0x5E);
    pub const WARNING: Color32 = Color32::from_rgb(0xF5, 0x9E, 0x0B);
    pub const DANGER: Color32 = Color32::from_rgb(0xEF, 0x44, 0x44);
    pub const DANGER_HOVER: Color32 = Color32::from_rgb(0xF8, 0x71, 0x71);

    pub const VIEWPORT: Color32 = Color32::from_rgb(0x00, 0x00, 0x00);
    pub const FOCUS_RING: Color32 = Color32::from_rgba_premultiplied(0x3B, 0x82, 0xF6, 0x80);
    pub const SCRIM: Color32 = Color32::from_rgba_premultiplied(0x00, 0x00, 0x00, 0xB3);
    pub const TOOLBAR: Color32 = Color32::from_rgba_premultiplied(0x11, 0x11, 0x13, 0xE6);
}

/// Semantic color roles for widgets.
pub struct Semantic;

impl Semantic {
    pub const BG_APP: Color32 = Palette::CANVAS;
    pub const BG_PANEL: Color32 = Palette::SURFACE;
    pub const BG_ELEVATED: Color32 = Palette::SURFACE_RAISED;
    pub const BG_VIEWPORT: Color32 = Palette::VIEWPORT;

    pub const FG_PRIMARY: Color32 = Palette::TEXT_PRIMARY;
    pub const FG_SECONDARY: Color32 = Palette::TEXT_SECONDARY;
    pub const FG_MUTED: Color32 = Palette::TEXT_TERTIARY;
    pub const FG_DISABLED: Color32 = Palette::TEXT_DISABLED;

    pub const INTERACTIVE: Color32 = Palette::ACCENT;
    pub const INTERACTIVE_HOVER: Color32 = Palette::ACCENT_HOVER;
    pub const INTERACTIVE_MUTED: Color32 = Palette::ACCENT_MUTED;
    pub const DESTRUCTIVE: Color32 = Palette::DANGER;
}
