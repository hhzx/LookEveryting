//! Soft-raster a mesh into an RGBA thumbnail (CPU, no wgpu).

use cap_model::MeshData;
use glam::{Mat4, Vec3, Vec4};

/// Render an orthographic lit preview into `size x size` RGBA.
pub fn render_mesh_thumbnail(mesh: &MeshData, size: u32) -> Option<Vec<u8>> {
    let size = size.max(32);
    if mesh.vertices.is_empty() {
        return None;
    }

    let eye = Vec3::new(1.4, 1.1, 1.6);
    let target = Vec3::from_array(mesh.bounds.center());
    let view = Mat4::look_at_rh(eye, target, Vec3::Y);
    let extent = mesh.bounds.size().max(0.001);
    let half = extent * 0.75;
    let proj = Mat4::orthographic_rh(-half, half, -half, half, 0.01, 100.0);
    let mvp = proj * view;

    let mut depth = vec![f32::INFINITY; (size * size) as usize];
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    // Dark blue-gray background
    for px in rgba.chunks_exact_mut(4) {
        px[0] = 0x18;
        px[1] = 0x1A;
        px[2] = 0x22;
        px[3] = 0xFF;
    }

    let key = Vec3::new(0.35, 0.75, 0.45).normalize();
    let fill = Vec3::new(-0.5, 0.3, -0.6).normalize();
    let ambient = 0.22_f32;
    let base = [0.35_f32, 0.72, 0.95];

    let mut draw_tri = |ia: usize, ib: usize, ic: usize| {
        let va = Vec3::from_array(*mesh.vertices.get(ia)?);
        let vb = Vec3::from_array(*mesh.vertices.get(ib)?);
        let vc = Vec3::from_array(*mesh.vertices.get(ic)?);
        let n = (vb - va).cross(vc - va).normalize_or_zero();
        if n.length_squared() < f32::EPSILON {
            return None::<()>;
        }
        let intensity = (ambient + n.dot(key).max(0.0) * 0.55 + n.dot(fill).max(0.0) * 0.25)
            .clamp(0.12, 1.0);
        let color = [
            (base[0] * intensity * 255.0) as u8,
            (base[1] * intensity * 255.0) as u8,
            (base[2] * intensity * 255.0) as u8,
        ];

        let pa = project(va, mvp, size)?;
        let pb = project(vb, mvp, size)?;
        let pc = project(vc, mvp, size)?;
        raster_tri(&mut rgba, &mut depth, size, pa, pb, pc, color);
        Some(())
    };

    if mesh.indices.is_empty() {
        for i in (0..mesh.vertices.len()).step_by(3) {
            if i + 2 < mesh.vertices.len() {
                let _ = draw_tri(i, i + 1, i + 2);
            }
        }
    } else {
        for chunk in mesh.indices.chunks(3) {
            if chunk.len() == 3 {
                let _ = draw_tri(chunk[0] as usize, chunk[1] as usize, chunk[2] as usize);
            }
        }
    }

    Some(rgba)
}

fn project(v: Vec3, mvp: Mat4, size: u32) -> Option<(f32, f32, f32)> {
    let clip = mvp * Vec4::new(v.x, v.y, v.z, 1.0);
    if clip.w <= 0.001 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    let x = (ndc.x * 0.5 + 0.5) * size as f32;
    let y = (1.0 - (ndc.y * 0.5 + 0.5)) * size as f32;
    Some((x, y, ndc.z))
}

fn raster_tri(
    rgba: &mut [u8],
    depth: &mut [f32],
    size: u32,
    a: (f32, f32, f32),
    b: (f32, f32, f32),
    c: (f32, f32, f32),
    color: [u8; 3],
) {
    let min_x = a.0.min(b.0).min(c.0).floor().max(0.0) as i32;
    let max_x = a.0.max(b.0).max(c.0).ceil().min(size as f32 - 1.0) as i32;
    let min_y = a.1.min(b.1).min(c.1).floor().max(0.0) as i32;
    let max_y = a.1.max(b.1).max(c.1).ceil().min(size as f32 - 1.0) as i32;
    let area = edge(a.0, a.1, b.0, b.1, c.0, c.1);
    if area.abs() < 1e-6 {
        return;
    }
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let w0 = edge(b.0, b.1, c.0, c.1, px, py) / area;
            let w1 = edge(c.0, c.1, a.0, a.1, px, py) / area;
            let w2 = edge(a.0, a.1, b.0, b.1, px, py) / area;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let z = w0 * a.2 + w1 * b.2 + w2 * c.2;
            let idx = (y as u32 * size + x as u32) as usize;
            if z < depth[idx] {
                depth[idx] = z;
                let o = idx * 4;
                rgba[o] = color[0];
                rgba[o + 1] = color[1];
                rgba[o + 2] = color[2];
                rgba[o + 3] = 255;
            }
        }
    }
}

fn edge(ax: f32, ay: f32, bx: f32, by: f32, cx: f32, cy: f32) -> f32 {
    (cx - ax) * (by - ay) - (cy - ay) * (bx - ax)
}
