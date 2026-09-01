//! Motion and animation tokens.

/// Duration in milliseconds.
pub mod duration {
    pub const INSTANT: u64 = 0;
    pub const FAST: u64 = 120;
    pub const NORMAL: u64 = 200;
    pub const SLOW: u64 = 300;
    pub const SLOWER: u64 = 400;
}

/// Behavioral constants.
pub mod behavior {
    /// Milliseconds before toolbar auto-hides in immersive mode.
    pub const TOOLBAR_AUTO_HIDE_MS: u64 = 3000;
    /// Bottom edge hover zone height to reveal toolbar.
    pub const TOOLBAR_REVEAL_ZONE: f32 = 80.0;
    /// Model hint overlay fade delay.
    pub const MODEL_HINT_FADE_MS: u64 = 5000;
}
