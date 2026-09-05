//! Scene display options for the 3D viewport (model-studio parity).

/// Shading / material preview modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MaterialMode {
    #[default]
    Original,
    Clay,
    Checker,
    Wireframe,
    Normals,
}

impl MaterialMode {
    pub fn as_shader_code(self) -> f32 {
        match self {
            Self::Original => 0.0,
            Self::Clay => 1.0,
            Self::Checker => 2.0,
            Self::Normals => 3.0,
            Self::Wireframe => 4.0,
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Original => "model-mat-original",
            Self::Clay => "model-mat-clay",
            Self::Checker => "model-mat-checker",
            Self::Wireframe => "model-mat-wireframe",
            Self::Normals => "model-mat-normals",
        }
    }

    pub const ALL: [MaterialMode; 5] = [
        Self::Original,
        Self::Clay,
        Self::Checker,
        Self::Wireframe,
        Self::Normals,
    ];
}

/// Key / fill / ambient lighting (degrees + intensities).
#[derive(Debug, Clone, Copy)]
pub struct SceneLighting {
    pub key_intensity: f32,
    pub key_azimuth_deg: f32,
    pub key_elevation_deg: f32,
    pub ambient: f32,
    pub fill: f32,
}

impl Default for SceneLighting {
    fn default() -> Self {
        Self {
            key_intensity: 1.25,
            key_azimuth_deg: 45.0,
            key_elevation_deg: 55.0,
            ambient: 1.0,
            fill: 0.35,
        }
    }
}

impl SceneLighting {
    /// Direction from surface toward the key light (world space).
    pub fn key_dir(&self) -> [f32; 3] {
        let az = self.key_azimuth_deg.to_radians();
        let el = self.key_elevation_deg.to_radians();
        let x = el.cos() * az.sin();
        let y = el.sin();
        let z = el.cos() * az.cos();
        let len = (x * x + y * y + z * z).sqrt().max(1e-5);
        [x / len, y / len, z / len]
    }
}

/// Full viewport scene settings.
#[derive(Debug, Clone)]
pub struct SceneSettings {
    pub material_mode: MaterialMode,
    pub lighting: SceneLighting,
    pub show_grid: bool,
    pub show_axes: bool,
    pub show_stats: bool,
    pub auto_rotate: bool,
    pub bg: super::ViewportBg,
}

impl Default for SceneSettings {
    fn default() -> Self {
        Self {
            material_mode: MaterialMode::Original,
            lighting: SceneLighting::default(),
            show_grid: true,
            show_axes: true,
            show_stats: true,
            auto_rotate: false,
            bg: super::ViewportBg::Gradient,
        }
    }
}
