//! Parse 3D model metadata and geometry for viewing.

mod mesh;

use std::path::Path;

use cap_core::MediaKind;
use thiserror::Error;

pub use mesh::{load_mesh, AlbedoMap, Bounds, MeshData, TextureMap};

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("unsupported model format")]
    UnsupportedFormat,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(#[from] gltf::Error),
    #[error("{0}")]
    Message(String),
}

/// Summary information about a 3D asset.
#[derive(Debug, Clone, Default)]
pub struct ModelInfo {
    pub format: String,
    pub mesh_count: usize,
    pub material_count: usize,
    pub node_count: usize,
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub has_textures: bool,
    pub notes: String,
}

impl ModelInfo {
    pub fn from_path(path: &Path) -> Result<Self, ModelError> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .ok_or(ModelError::UnsupportedFormat)?;

        match ext.as_str() {
            "glb" | "gltf" => parse_gltf_meta(path, &ext),
            "obj" | "stl" | "fbx" | "ply" | "dae" | "3mf" | "max" => {
                if ext == "max" {
                    return Ok(ModelInfo {
                        format: "MAX".to_string(),
                        notes: "3ds Max (.max) is proprietary. Export to FBX, OBJ, or GLTF to preview."
                            .to_string(),
                        ..Default::default()
                    });
                }
                match load_mesh(path) {
                    Ok(mesh) => Ok(info_from_mesh(&ext, &mesh)),
                    Err(ModelError::Message(msg)) => Ok(ModelInfo {
                        format: ext.to_ascii_uppercase(),
                        notes: msg,
                        ..Default::default()
                    }),
                    Err(err) => Err(err),
                }
            }
            _ => Err(ModelError::UnsupportedFormat),
        }
    }
}

pub fn info_from_mesh(ext: &str, mesh: &MeshData) -> ModelInfo {
    ModelInfo {
        format: ext.to_ascii_uppercase(),
        mesh_count: mesh.mesh_count.max(1),
        material_count: mesh.material_count,
        vertex_count: mesh.vertices.len(),
        triangle_count: mesh.triangle_count(),
        has_textures: mesh.albedo.is_some() || mesh.normal.is_some(),
        notes: "Loaded for in-app preview.".to_string(),
        ..Default::default()
    }
}

fn parse_gltf_meta(path: &Path, ext: &str) -> Result<ModelInfo, ModelError> {
    if cap_core::classify_extension(path) != Some(MediaKind::Model) {
        return Err(ModelError::UnsupportedFormat);
    }

    let mesh = load_mesh(path)?;
    let mut info = info_from_mesh(ext, &mesh);
    let (document, _, _) = gltf::import(path)?;
    info.material_count = document.materials().count();
    info.node_count = document.nodes().count();
    info.has_textures =
        document.textures().next().is_some() || document.images().next().is_some();
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_extension() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mesh.xyz");
        std::fs::write(&path, b"data").unwrap();
        let err = ModelInfo::from_path(&path).unwrap_err();
        assert!(matches!(err, ModelError::UnsupportedFormat));
    }

    #[test]
    fn stub_for_max() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("scene.max");
        std::fs::write(&path, b"data").unwrap();
        let info = ModelInfo::from_path(&path).unwrap();
        assert_eq!(info.format, "MAX");
    }
}
