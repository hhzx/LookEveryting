//! Responsive layout breakpoints and density.

/// Window width breakpoints (logical points).
pub mod breakpoint {
    pub const COMPACT_MAX: f32 = 719.0;
    pub const COMFORTABLE_MIN: f32 = 720.0;
    pub const COMFORTABLE_MAX: f32 = 1279.0;
    pub const SPACIOUS_MIN: f32 = 1280.0;
}

/// UI density multipliers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Density {
  Compact,
  #[default]
  Comfortable,
  Accessibility,
}

impl Density {
    pub fn scale(self) -> f32 {
        match self {
            Self::Compact => 0.9,
            Self::Comfortable => 1.0,
            Self::Accessibility => 1.15,
        }
    }
}

/// Layout mode derived from window width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    Compact,
    Comfortable,
    Spacious,
}

impl LayoutMode {
    pub fn from_width(width: f32) -> Self {
        if width <= breakpoint::COMPACT_MAX {
            Self::Compact
        } else if width <= breakpoint::COMFORTABLE_MAX {
            Self::Comfortable
        } else {
            Self::Spacious
        }
    }

    pub fn sidebar_width(self, collapsed: bool) -> f32 {
        use super::spacing::component;
        match self {
            Self::Compact => 0.0,
            Self::Comfortable => component::SIDEBAR_COLLAPSED,
            Self::Spacious if collapsed => component::SIDEBAR_COLLAPSED,
            Self::Spacious => component::SIDEBAR_WIDTH,
        }
    }

    pub fn thumbnail_size(self) -> f32 {
        use super::spacing::component;
        match self {
            Self::Compact => 80.0,
            Self::Comfortable => 100.0,
            Self::Spacious => component::THUMBNAIL_SIZE,
        }
    }
}

/// Viewer UI state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewerMode {
    #[default]
    Viewer,
    Immersive,
    ViewerWithInfo,
    Browse,
    Fullscreen,
}
