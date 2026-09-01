//! Parse 3D model metadata for display in the info panel.

use std::path::Path;

use cap_core::MediaKind;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("unsupported model format")]
    UnsupportedFormat,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(#[from] gltf::Error),
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
            "glb" | "gltf" => parse_gltf(path, &ext),
            "obj" | "stl" | "fbx" | "ply" | "dae" | "3mf" => Ok(basic_stub(&ext, path)),
            _ => Err(ModelError::UnsupportedFormat),
        }
    }
}

fn parse_gltf(path: &Path, ext: &str) -> Result<ModelInfo, ModelError> {
    if cap_core::classify_extension(path) != Some(MediaKind::Model) {
        return Err(ModelError::UnsupportedFormat);
    }

    let (document, buffers, _images) = gltf::import(path)?;

    let mut vertex_count = 0usize;
    let mut triangle_count = 0usize;

    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| {
                buffers
                    .get(buffer.index())
                    .map(|data| data.0.as_slice())
            });
            if let Some(iter) = reader.read_positions() {
                vertex_count += iter.count();
            }
            if let Some(indices) = reader.read_indices() {
                triangle_count += indices.into_u32().count() / 3;
            } else if let Some(iter) = reader.read_positions() {
                triangle_count += iter.count() / 3;
            }
        }
    }

    let has_textures = document.textures().next().is_some() || document.images().next().is_some();

    Ok(ModelInfo {
        format: ext.to_ascii_uppercase(),
        mesh_count: document.meshes().count(),
        material_count: document.materials().count(),
        node_count: document.nodes().count(),
        vertex_count,
        triangle_count,
        has_textures,
        notes: "GLTF metadata parsed successfully.".to_string(),
    })
}

fn basic_stub(ext: &str, path: &Path) -> ModelInfo {
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    ModelInfo {
        format: ext.to_ascii_uppercase(),
        notes: format!(
            "{} file ({} KB). Use Open to view in your system 3D application.",
            ext.to_ascii_uppercase(),
            size / 1024
        ),
        ..Default::default()
    }
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
    fn stub_for_obj() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cube.obj");
        std::fs::write(&path, b"v 0 0 0").unwrap();
        let info = ModelInfo::from_path(&path).unwrap();
        assert_eq!(info.format, "OBJ");
    }
}
