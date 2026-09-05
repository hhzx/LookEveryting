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

/// Triangle mesh for preview rendering.
#[derive(Debug, Clone)]
pub struct MeshData {
    pub vertices: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub bounds: Bounds,
    /// Optional per-vertex UVs (same length as vertices when present).
    pub uvs: Vec<[f32; 2]>,
    /// PBR base color factor (linear RGB + alpha).
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    /// Optional albedo texture (RGBA8).
    pub albedo: Option<TextureMap>,
    /// Optional tangent-space normal map (RGBA8, XYZ in RGB).
    pub normal: Option<TextureMap>,
    /// Source mesh / material counts for HUD (best-effort).
    pub mesh_count: usize,
    pub material_count: usize,
}

/// RGBA8 texture map (albedo or normal).
#[derive(Debug, Clone)]
pub struct TextureMap {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Alias kept for older call sites.
pub type AlbedoMap = TextureMap;

impl Default for MeshData {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            bounds: Bounds::default(),
            uvs: Vec::new(),
            base_color: [0.35, 0.72, 0.95, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            albedo: None,
            normal: None,
            mesh_count: 0,
            material_count: 0,
        }
    }
}

impl MeshData {
    pub fn triangle_count(&self) -> usize {
        if self.indices.is_empty() {
            self.vertices.len() / 3
        } else {
            self.indices.len() / 3
        }
    }

    pub fn extent_xyz(&self) -> [f32; 3] {
        [
            (self.bounds.max[0] - self.bounds.min[0]).abs(),
            (self.bounds.max[1] - self.bounds.min[1]).abs(),
            (self.bounds.max[2] - self.bounds.min[2]).abs(),
        ]
    }
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
        "3mf" => load_3mf(path)?,
        "max" => {
            return Err(ModelError::Message(
                "3ds Max (.max) is proprietary. Export to FBX, OBJ, or GLTF.".to_string(),
            ))
        }
        "ply" | "dae" => {
            return Err(ModelError::Message(format!(
                "{} preview is not implemented yet. Export to OBJ, STL, GLTF, FBX, or 3MF.",
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
        mesh_count: 1,
        ..Default::default()
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
        mesh_count: 1,
        ..Default::default()
    })
}

fn gltf_image_to_map(img: &gltf::image::Data) -> Option<TextureMap> {
    let rgba = match img.format {
        gltf::image::Format::R8G8B8A8 => img.pixels.clone(),
        gltf::image::Format::R8G8B8 => img
            .pixels
            .chunks(3)
            .flat_map(|c| [c[0], c[1], c[2], 255])
            .collect(),
        gltf::image::Format::R8 => img
            .pixels
            .iter()
            .flat_map(|&c| [c, c, c, 255])
            .collect(),
        gltf::image::Format::R8G8 => img
            .pixels
            .chunks(2)
            .flat_map(|c| [c[0], c[1], 0, 255])
            .collect(),
        _ => return None,
    };
    if rgba.is_empty() {
        return None;
    }
    Some(TextureMap {
        width: img.width,
        height: img.height,
        rgba,
    })
}

fn load_gltf(path: &Path) -> Result<MeshData, ModelError> {
    let (document, buffers, images) = gltf::import(path)?;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut uvs = Vec::new();
    let mut base = 0u32;
    let mut base_color = [0.35_f32, 0.72, 0.95, 1.0];
    let mut metallic = 0.0_f32;
    let mut roughness = 0.5_f32;
    let mut albedo = None;
    let mut normal = None;

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
            let count = positions.len();
            let tex: Vec<[f32; 2]> = reader
                .read_tex_coords(0)
                .map(|tc| tc.into_f32().map(|[u, v]| [u, v]).collect())
                .unwrap_or_else(|| vec![[0.0, 0.0]; count]);

            if albedo.is_none() || normal.is_none() {
                let mat = primitive.material();
                let pbr = mat.pbr_metallic_roughness();
                if albedo.is_none() {
                    base_color = pbr.base_color_factor();
                    metallic = pbr.metallic_factor();
                    roughness = pbr.roughness_factor();
                    if let Some(info) = pbr.base_color_texture() {
                        let tex_idx = info.texture().source().index();
                        if let Some(img) = images.get(tex_idx) {
                            albedo = gltf_image_to_map(img);
                        }
                    }
                }
                if normal.is_none() {
                    if let Some(info) = mat.normal_texture() {
                        let tex_idx = info.texture().source().index();
                        if let Some(img) = images.get(tex_idx) {
                            normal = gltf_image_to_map(img);
                        }
                    }
                }
            }

            let local_base = base;
            vertices.extend(positions);
            uvs.extend(tex);
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
        uvs,
        base_color,
        metallic,
        roughness,
        albedo,
        normal,
        mesh_count: document.meshes().count().max(1),
        material_count: document.materials().count(),
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
    let mut uvs = Vec::new();
    let mut mesh_count = 0usize;
    let mut material_count = scene.materials.as_ref().len();

    // Prefer nodes so instance transforms (geometry_to_world) are applied.
    let mut saw_node_mesh = false;
    for node in scene.nodes.as_ref() {
        let Some(mesh) = node.mesh.as_ref() else {
            continue;
        };
        if !mesh.vertex_position.exists {
            continue;
        }
        saw_node_mesh = true;
        mesh_count += 1;
        material_count = material_count.max(mesh.materials.as_ref().len());
        append_ufbx_mesh(
            mesh,
            Some(&node.geometry_to_world),
            &mut vertices,
            &mut indices,
            &mut uvs,
        );
    }

    if !saw_node_mesh {
        for mesh in scene.meshes.as_ref() {
            if !mesh.vertex_position.exists && mesh.vertices.as_ref().is_empty() {
                continue;
            }
            mesh_count += 1;
            material_count = material_count.max(mesh.materials.as_ref().len());
            append_ufbx_mesh(mesh, None, &mut vertices, &mut indices, &mut uvs);
        }
    }

    if vertices.is_empty() {
        return Err(ModelError::Message(
            "No geometry found in file (empty or unsupported FBX geometry)."
                .to_string(),
        ));
    }

    Ok(MeshData {
        vertices,
        indices,
        bounds: Bounds::default(),
        uvs,
        mesh_count: mesh_count.max(1),
        material_count,
        ..Default::default()
    })
}

fn append_ufbx_mesh(
    mesh: &ufbx::Mesh,
    xform: Option<&ufbx::Matrix>,
    vertices: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    uvs: &mut Vec<[f32; 2]>,
) {
    const MAX_VERTS: usize = 400_000;
    if vertices.len() >= MAX_VERTS {
        return;
    }

    let use_stream = mesh.vertex_position.exists;
    if use_stream {
        let pos_len = mesh.vertex_position.indices.as_ref().len();
        let uv_ok = mesh.vertex_uv.exists;
        for face in mesh.faces.as_ref() {
            if face.num_indices < 3 {
                continue;
            }
            let begin = face.index_begin as usize;
            let last = begin + face.num_indices as usize;
            if last > pos_len {
                continue;
            }
            let i0 = begin;
            for t in 1..(face.num_indices as usize - 1) {
                if vertices.len() + 3 > MAX_VERTS {
                    return;
                }
                let corners = [i0, begin + t, begin + t + 1];
                for &corner in &corners {
                    let idx = mesh.vertex_position.indices.as_ref()[corner] as usize;
                    let Some(mut p) = mesh.vertex_position.values.as_ref().get(idx).copied() else {
                        continue;
                    };
                    if let Some(m) = xform {
                        p = ufbx::transform_position(m, p);
                    }
                    vertices.push([p.x as f32, p.y as f32, p.z as f32]);
                    let uv = if uv_ok {
                        let uvidx = mesh
                            .vertex_uv
                            .indices
                            .as_ref()
                            .get(corner)
                            .copied()
                            .unwrap_or(0) as usize;
                        mesh.vertex_uv
                            .values
                            .as_ref()
                            .get(uvidx)
                            .map(|u| [u.x as f32, u.y as f32])
                            .unwrap_or([0.0, 0.0])
                    } else {
                        [0.0, 0.0]
                    };
                    uvs.push(uv);
                    indices.push((vertices.len() as u32) - 1);
                }
            }
        }
        return;
    }

    let local_base = vertices.len() as u32;
    let src_verts = mesh.vertices.as_ref();
    if vertices.len() + src_verts.len() > MAX_VERTS {
        return;
    }
    for v in src_verts {
        let mut p = *v;
        if let Some(m) = xform {
            p = ufbx::transform_position(m, p);
        }
        vertices.push([p.x as f32, p.y as f32, p.z as f32]);
        uvs.push([0.0, 0.0]);
    }
    let vidx = mesh.vertex_indices.as_ref();
    for face in mesh.faces.as_ref() {
        if face.num_indices < 3 {
            continue;
        }
        let begin = face.index_begin as usize;
        let end = begin + face.num_indices as usize;
        if end > vidx.len() {
            continue;
        }
        let a = vidx[begin] + local_base;
        for i in 1..(face.num_indices as usize - 1) {
            let b = vidx[begin + i] + local_base;
            let c = vidx[begin + i + 1] + local_base;
            indices.extend_from_slice(&[a, b, c]);
        }
    }
}

fn load_3mf(path: &Path) -> Result<MeshData, ModelError> {
    let file = std::fs::File::open(path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| ModelError::Message(format!("3MF zip: {e}")))?;

    // Prefer standard model path; otherwise first *.model entry.
    let mut model_names: Vec<String> = Vec::new();
    for i in 0..archive.len() {
        let Ok(entry) = archive.by_index(i) else {
            continue;
        };
        let name = entry.name().replace('\\', "/");
        if name.to_ascii_lowercase().ends_with(".model") {
            model_names.push(name);
        }
    }
    if model_names.is_empty() {
        return Err(ModelError::Message(
            "3MF package has no .model document.".to_string(),
        ));
    }
    model_names.sort_by_key(|n| {
        let lower = n.to_ascii_lowercase();
        if lower.contains("3dmodel.model") {
            0
        } else if lower.starts_with("3d/") {
            1
        } else {
            2
        }
    });

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut mesh_count = 0usize;

    for name in model_names {
        let mut entry = archive
            .by_name(&name)
            .map_err(|e| ModelError::Message(format!("3MF entry {name}: {e}")))?;
        let mut xml = String::new();
        std::io::Read::read_to_string(&mut entry, &mut xml)
            .map_err(|e| ModelError::Message(format!("3MF read: {e}")))?;
        let (v, i, meshes) = parse_3mf_model_xml(&xml)?;
        let base = vertices.len() as u32;
        vertices.extend(v);
        indices.extend(i.into_iter().map(|x| x + base));
        mesh_count += meshes;
        if vertices.len() > 400_000 {
            break;
        }
    }

    if vertices.is_empty() {
        return Err(ModelError::Message(
            "No triangle mesh found in 3MF file.".to_string(),
        ));
    }

    // 3MF is often Z-up; convert to Y-up like STL.
    for v in &mut vertices {
        let (x, y, z) = (v[0], v[1], v[2]);
        *v = [x, z, -y];
    }

    Ok(MeshData {
        vertices,
        indices,
        bounds: Bounds::default(),
        mesh_count: mesh_count.max(1),
        ..Default::default()
    })
}

fn parse_3mf_model_xml(xml: &str) -> Result<(Vec<[f32; 3]>, Vec<u32>, usize), ModelError> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| ModelError::Message(format!("3MF XML: {e}")))?;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut mesh_count = 0usize;

    for mesh in doc.descendants().filter(|n| n.tag_name().name() == "mesh") {
        let local_base = vertices.len() as u32;
        let mut local_count = 0u32;
        if let Some(verts) = mesh.children().find(|n| n.tag_name().name() == "vertices") {
            for v in verts.children().filter(|n| n.tag_name().name() == "vertex") {
                let x = attr_f32(v, "x").unwrap_or(0.0);
                let y = attr_f32(v, "y").unwrap_or(0.0);
                let z = attr_f32(v, "z").unwrap_or(0.0);
                vertices.push([x, y, z]);
                local_count += 1;
            }
        }
        if local_count == 0 {
            continue;
        }
        let mut tris = 0usize;
        if let Some(tris_node) = mesh.children().find(|n| n.tag_name().name() == "triangles") {
            for t in tris_node.children().filter(|n| n.tag_name().name() == "triangle") {
                let Some(a) = attr_u32(t, "v1") else {
                    continue;
                };
                let Some(b) = attr_u32(t, "v2") else {
                    continue;
                };
                let Some(c) = attr_u32(t, "v3") else {
                    continue;
                };
                if a >= local_count || b >= local_count || c >= local_count {
                    continue;
                }
                indices.extend_from_slice(&[local_base + a, local_base + b, local_base + c]);
                tris += 1;
            }
        }
        if tris > 0 {
            mesh_count += 1;
        }
    }

    Ok((vertices, indices, mesh_count))
}

fn attr_f32(node: roxmltree::Node<'_, '_>, name: &str) -> Option<f32> {
    node.attribute(name)?.parse().ok()
}

fn attr_u32(node: roxmltree::Node<'_, '_>, name: &str) -> Option<u32> {
    node.attribute(name)?.parse().ok()
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

    #[test]
    fn loads_minimal_3mf() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("box.3mf");
        let xml = r#"<?xml version="1.0"?>
<model unit="millimeter"
 xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
 <resources>
  <object id="1" type="model">
   <mesh>
    <vertices>
     <vertex x="0" y="0" z="0"/>
     <vertex x="1" y="0" z="0"/>
     <vertex x="0" y="1" z="0"/>
    </vertices>
    <triangles>
     <triangle v1="0" v2="1" v3="2"/>
    </triangles>
   </mesh>
  </object>
 </resources>
 <build><item objectid="1"/></build>
</model>"#;
        {
            let file = std::fs::File::create(&path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let opts = zip::write::SimpleFileOptions::default();
            zip.start_file("3D/3dmodel.model", opts).unwrap();
            use std::io::Write;
            zip.write_all(xml.as_bytes()).unwrap();
            zip.finish().unwrap();
        }
        let mesh = load_mesh(&path).unwrap();
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.triangle_count(), 1);
    }
}
