//! Spacing, radius, and component dimension tokens.

/// Spacing scale (logical points).
pub mod space {
    pub const PX: f32 = 1.0;
    pub const S0_5: f32 = 2.0;
    pub const S1: f32 = 4.0;
    pub const S1_5: f32 = 6.0;
    pub const S2: f32 = 8.0;
    pub const S2_5: f32 = 10.0;
    pub const S3: f32 = 12.0;
    pub const S4: f32 = 16.0;
    pub const S5: f32 = 20.0;
    pub const S6: f32 = 24.0;
    pub const S8: f32 = 32.0;
    pub const S10: f32 = 40.0;
    pub const S12: f32 = 48.0;
    pub const S16: f32 = 64.0;
}

/// Border radius tokens.
pub mod radius {
    pub const NONE: f32 = 0.0;
    pub const SM: f32 = 4.0;
    pub const MD: f32 = 6.0;
    pub const LG: f32 = 8.0;
    pub const XL: f32 = 12.0;
    pub const XXL: f32 = 16.0;
    pub const FULL: f32 = 9999.0;
}

/// Fixed component dimensions.
pub mod component {
    pub const TITLEBAR_HEIGHT: f32 = 40.0;
    pub const TOOLBAR_HEIGHT: f32 = 44.0;
    pub const SIDEBAR_WIDTH: f32 = 240.0;
    pub const SIDEBAR_COLLAPSED: f32 = 128.0;
    pub const INFO_PANEL_WIDTH: f32 = 280.0;
    pub const DRAWER_WIDTH: f32 = 320.0;
    pub const THUMBNAIL_SIZE: f32 = 120.0;
    pub const THUMBNAIL_STRIP_SIZE: f32 = 96.0;
    pub const THUMBNAIL_STRIP_HEIGHT: f32 = 120.0;
    pub const THUMBNAIL_GAP: f32 = 8.0;
    pub const ICON_BUTTON: f32 = 32.0;
    pub const ICON_BUTTON_SM: f32 = 28.0;
    pub const INPUT_HEIGHT: f32 = 36.0;
    pub const PANEL_PADDING: f32 = 16.0;
    pub const FLOATING_TOOLBAR_MARGIN: f32 = 12.0;
    pub const MIN_WINDOW_WIDTH: f32 = 480.0;
    pub const MIN_WINDOW_HEIGHT: f32 = 360.0;
}

/// Z-order layers.
pub mod z_index {
    pub const VIEWPORT: i32 = 0;
    pub const TOOLBAR: i32 = 10;
    pub const SIDEBAR: i32 = 20;
    pub const DRAWER: i32 = 30;
    pub const DROPDOWN: i32 = 40;
    pub const MODAL: i32 = 50;
    pub const TOAST: i32 = 60;
    pub const TOOLTIP: i32 = 70;
}
