//! Unified mesh geometry for 3D preview.

use std::path::Path;

use crate::ModelError;

/// Axis-aligned bounding box.
#[derive(Debug, Clone, Copy, Default)]
pub struct Bounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl Bounds {
    pub fn from_vertices(vertices: &[[f32; 3]]) -> Self {
        let mut bounds = Self {
            min: [f32::INFINITY; 3],
            max: [f32::NEG_INFINITY; 3],
        };
        for v in vertices {
            for i in 0..3 {
                bounds.min[i] = bounds.min[i].min(v[i]);
                bounds.max[i] = bounds.max[i].max(v[i]);
            }
        }
        bounds
    }

    pub fn center(self) -> [f32; 3] {
        [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
            (self.min[2] + self.max[2]) * 0.5,
        ]
    }

    pub fn size(self) -> f32 {
        let dx = self.max[0] - self.min[0];
        let dy = self.max[1] - self.min[1];
        let dz = self.max[2] - self.min[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Triangle mesh for software rendering.
#[derive(Debug, Clone, Default)]
pub struct MeshData {
    pub vertices: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub bounds: Bounds,
}

const MAX_TRIANGLES: usize = 120_000;

impl MeshData {
    pub fn normalize_to_unit(&mut self) {
        if self.vertices.is_empty() {
            return;
        }
        let bounds = Bounds::from_vertices(&self.vertices);
        let center = bounds.center();
        let scale = bounds.size().max(0.001);
        for v in &mut self.vertices {
            for i in 0..3 {
                v[i] = (v[i] - center[i]) / scale;
            }
        }
        self.bounds = Bounds::from_vertices(&self.vertices);
    }

    /// Reduce triangle count for real-time preview.
    pub fn simplify_for_preview(&mut self) {
        if self.indices.is_empty() {
            return;
        }
        let tri_count = self.indices.len() / 3;
        if tri_count <= MAX_TRIANGLES {
            return;
        }
        let step = tri_count.div_ceil(MAX_TRIANGLES);
        self.indices = self
            .indices
            .chunks(3)
            .enumerate()
            .filter(|(i, _)| i % step == 0)
            .flat_map(|(_, tri)| tri.iter().copied())
            .collect();
    }
}

/// Load mesh geometry from a supported model file.
pub fn load_mesh(path: &Path) -> Result<MeshData, ModelError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .ok_or(ModelError::UnsupportedFormat)?;

    let mut mesh = match ext.as_str() {
        "obj" => load_obj(path)?,
        "stl" => load_stl(path)?,
        "glb" | "gltf" => load_gltf(path)?,
        "fbx" => load_fbx(path)?,
        "max" => {
            return Err(ModelError::Message(
                "3ds Max (.max) is proprietary. Export to FBX, OBJ, or GLTF.".to_string(),
            ))
        }
        "ply" | "dae" | "3mf" => {
            return Err(ModelError::Message(format!(
                "{} preview is not implemented yet. Export to OBJ, STL, GLTF, or FBX.",
                ext.to_ascii_uppercase()
            )));
        }
        _ => return Err(ModelError::UnsupportedFormat),
    };

    if mesh.vertices.is_empty() {
        return Err(ModelError::Message("No geometry found in file.".to_string()));
    }
    mesh.bounds = Bounds::from_vertices(&mesh.vertices);
    mesh.normalize_to_unit();
    mesh.bounds = Bounds::from_vertices(&mesh.vertices);
    mesh.simplify_for_preview();
    Ok(mesh)
}

fn load_obj(path: &Path) -> Result<MeshData, ModelError> {
    let (models, _) = tobj::load_obj(path, &tobj::LoadOptions::default())
        .map_err(|e| ModelError::Message(e.to_string()))?;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for model in models {
        let offset = vertices.len() as u32;
        for chunk in model.mesh.positions.chunks(3) {
            vertices.push([chunk[0], chunk[1], chunk[2]]);
        }
        for idx in &model.mesh.indices {
            indices.push(offset + *idx);
        }
    }
    Ok(MeshData {
        vertices,
        indices,
        bounds: Bounds::default(),
    })
}

fn load_stl(path: &Path) -> Result<MeshData, ModelError> {
    let bytes = std::fs::read(path)?;
    let mut cursor = std::io::Cursor::new(bytes);
    let mesh = stl_io::read_stl(&mut cursor).map_err(|e| ModelError::Message(e.to_string()))?;
    // STL is typically Z-up; convert to Y-up for the viewer.
    let vertices = mesh
        .vertices
        .iter()
        .map(|v| [v[0], v[2], -v[1]])
        .collect();
    let mut indices = Vec::new();
    for face in &mesh.faces {
        indices.extend(face.vertices.iter().map(|&i| i as u32));
    }
    Ok(MeshData {
        vertices,
        indices,
        bounds: Bounds::default(),
    })
}

fn load_gltf(path: &Path) -> Result<MeshData, ModelError> {
    let (document, buffers, _) = gltf::import(path)?;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut base = 0u32;

    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| {
                buffers
                    .get(buffer.index())
                    .map(|data| data.0.as_slice())
            });
            let positions: Vec<[f32; 3]> = reader
                .read_positions()
                .map(|iter| iter.map(|[x, y, z]| [x, y, z]).collect())
                .unwrap_or_default();
            let local_base = base;
            vertices.extend(positions);
            if let Some(idx) = reader.read_indices() {
                for i in idx.into_u32() {
                    indices.push(local_base + i);
                }
            } else {
                for i in 0..(vertices.len() as u32 - local_base) {
                    indices.push(local_base + i);
                }
            }
            base = vertices.len() as u32;
        }
    }

    Ok(MeshData {
        vertices,
        indices,
        bounds: Bounds::default(),
    })
}

fn load_fbx(path: &Path) -> Result<MeshData, ModelError> {
    let bytes = std::fs::read(path)?;
    let scene = ufbx::load_memory(
        &bytes,
        ufbx::LoadOpts {
            target_axes: ufbx::CoordinateAxes {
                right: ufbx::CoordinateAxis::PositiveX,
                up: ufbx::CoordinateAxis::PositiveY,
                front: ufbx::CoordinateAxis::PositiveZ,
            },
            ..Default::default()
        },
    )
    .map_err(|e| ModelError::Message(format!("FBX load failed: {e:?}")))?;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for mesh in scene.meshes.as_ref() {
        let base = vertices.len() as u32;
        for v in mesh.vertices.as_ref() {
            vertices.push([v.x as f32, v.y as f32, v.z as f32]);
        }
        for face in mesh.faces.as_ref() {
            if face.num_indices < 3 {
                continue;
            }
            let a = mesh.vertex_indices[face.index_begin as usize] + base;
            for i in 1..(face.num_indices as usize - 1) {
                let b = mesh.vertex_indices[face.index_begin as usize + i] + base;
                let c = mesh.vertex_indices[face.index_begin as usize + i + 1] + base;
                indices.extend_from_slice(&[a, b, c]);
            }
        }
    }

    Ok(MeshData {
        vertices,
        indices,
        bounds: Bounds::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_obj_cube() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cube.obj");
        std::fs::write(
            &path,
            "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n",
        )
        .unwrap();
        let mesh = load_mesh(&path).unwrap();
        assert_eq!(mesh.vertices.len(), 3);
    }
}
