//! 3D model viewport rendering for LookEveryting (CPU fallback + wgpu).

mod gpu_mesh;
mod scene;
mod thumb;

use std::sync::Arc;

use cap_model::MeshData;
use cap_ui::colors::{Palette, Semantic};
use egui::{Color32, PointerButton, Pos2, Rect, Response, Sense, Ui, Vec2};
use glam::{Mat4, Vec3, Vec4};

pub use gpu_mesh::{MeshPaintCallback, MeshRenderResources};
pub use scene::{MaterialMode, SceneLighting, SceneSettings};
pub use thumb::render_mesh_thumbnail;

/// Orbit camera around a model centroid.
#[derive(Debug, Clone)]
pub struct OrbitCamera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub target: [f32; 3],
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            yaw: 0.5,
            pitch: 0.4,
            distance: 2.5,
            target: [0.0, 0.0, 0.0],
        }
    }
}

impl OrbitCamera {
    pub fn fit_bounds(bounds: &cap_model::Bounds) -> Self {
        let dx = bounds.max[0] - bounds.min[0];
        let dy = bounds.max[1] - bounds.min[1];
        let dz = bounds.max[2] - bounds.min[2];
        let max_extent = dx.max(dy).max(dz).max(0.001);
        Self {
            yaw: 0.6,
            pitch: 0.35,
            distance: max_extent * 2.2,
            target: bounds.center(),
        }
    }

    pub fn rotate(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        self.pitch = (self.pitch + delta_pitch).clamp(-1.4, 1.4);
    }

    pub fn zoom(&mut self, factor: f32) {
        self.distance = (self.distance * factor).clamp(0.2, 50.0);
    }

    pub fn pan_screen(&mut self, dx: f32, dy: f32) {
        let eye = orbit_eye(self);
        let target = Vec3::from_array(self.target);
        let forward = (target - eye).normalize_or_zero();
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();
        let scale = self.distance * 0.0025;
        let delta = right * (-dx * scale) + up * (dy * scale);
        self.target = (target + delta).to_array();
    }
}

/// Viewport background fill mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewportBg {
    #[default]
    Solid,
    Gradient,
}

/// Draw options for GPU path.
#[derive(Default)]
pub struct MeshDrawOpts {
    /// When set, solid shading uses wgpu PaintCallback (depth-tested).
    pub gpu_available: bool,
    pub mesh_to_upload: Option<Arc<MeshData>>,
    pub clear_gpu_mesh: bool,
}

/// Draw a mesh inside `rect`, returning interaction response.
pub fn draw_mesh_viewport(
    ui: &mut Ui,
    rect: Rect,
    mesh: &MeshData,
    camera: &mut OrbitCamera,
    wireframe: bool,
    bg: ViewportBg,
) -> Response {
    let mut scene = SceneSettings {
        material_mode: if wireframe {
            MaterialMode::Wireframe
        } else {
            MaterialMode::Original
        },
        bg,
        ..Default::default()
    };
    draw_mesh_viewport_ex(ui, rect, mesh, camera, &mut scene, MeshDrawOpts::default())
}

pub fn draw_mesh_viewport_ex(
    ui: &mut Ui,
    rect: Rect,
    mesh: &MeshData,
    camera: &mut OrbitCamera,
    scene: &mut SceneSettings,
    opts: MeshDrawOpts,
) -> Response {
    let response = ui.allocate_rect(rect, Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    match scene.bg {
        ViewportBg::Solid => {
            painter.rect_filled(rect, 0.0, Semantic::BG_VIEWPORT);
        }
        ViewportBg::Gradient => {
            draw_gradient_bg(&painter, rect);
        }
    }

    if mesh.vertices.is_empty() {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Empty mesh",
            egui::FontId::proportional(14.0),
            Semantic::FG_MUTED,
        );
        return response;
    }

    if scene.auto_rotate {
        let dt = ui.input(|i| i.stable_dt).clamp(0.0, 0.05);
        camera.yaw += dt * 0.45;
        ui.ctx().request_repaint();
    }

    if response.dragged() {
        let delta = response.drag_delta();
        let secondary = ui.input(|i| i.pointer.button_down(PointerButton::Secondary));
        let middle = ui.input(|i| i.pointer.button_down(PointerButton::Middle));
        if secondary || middle {
            camera.pan_screen(delta.x, delta.y);
        } else {
            camera.rotate(delta.x * 0.01, delta.y * 0.01);
        }
    }
    if response.hovered() {
        // Invert: scroll up �?zoom in (closer).
        let scroll = -ui.input(|i| i.smooth_scroll_delta.y + i.raw_scroll_delta.y);
        if scroll != 0.0 {
            camera.zoom((1.0 - scroll * 0.002).clamp(0.85, 1.15));
        }
    }

    let mvp = view_proj(camera, rect);
    let ppp = ui.ctx().pixels_per_point();
    let size_px = (
        (rect.width() * ppp).round().max(1.0) as u32,
        (rect.height() * ppp).round().max(1.0) as u32,
    );

    let wireframe = scene.material_mode == MaterialMode::Wireframe;
    let use_gpu = opts.gpu_available && !wireframe;
    if use_gpu {
        let dir = scene.lighting.key_dir();
        ui.painter().add(eframe::egui_wgpu::Callback::new_paint_callback(
            rect,
            MeshPaintCallback {
                mvp,
                wireframe: false,
                size_px,
                mesh_to_upload: opts.mesh_to_upload,
                clear_mesh: opts.clear_gpu_mesh,
                light_dir: [
                    dir[0],
                    dir[1],
                    dir[2],
                    scene.lighting.key_intensity,
                ],
                env: [
                    scene.lighting.ambient,
                    scene.lighting.fill,
                    scene.material_mode.as_shader_code(),
                    0.0,
                ],
            },
        ));
    } else {
        let mut projected: Vec<Option<Pos2>> = Vec::with_capacity(mesh.vertices.len());
        for v in &mesh.vertices {
            projected.push(project_vertex(*v, mvp, rect));
        }

        let visible = projected.iter().filter(|p| p.is_some()).count();
        if visible == 0 {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Unable to render model",
                egui::FontId::proportional(14.0),
                Semantic::FG_MUTED,
            );
            return response;
        }

        if wireframe {
            draw_wireframe(&painter, mesh, &projected, Palette::ACCENT);
        } else {
            draw_solid(&painter, mesh, &projected, mvp);
        }
    }

    if scene.show_grid {
        draw_ground_grid(&painter, camera, rect);
    }
    if scene.show_axes {
        draw_axis_gizmo(&painter, rect);
    }

    response
}

fn draw_gradient_bg(painter: &egui::Painter, rect: Rect) {
    let top = Color32::from_rgb(0x2C, 0x2C, 0x34);
    let bottom = Color32::from_rgb(0x10, 0x10, 0x14);
    let mut mesh = egui::Mesh::default();
    let i = mesh.vertices.len() as u32;
    let uv = egui::pos2(0.5, 0.5);
    mesh.vertices.push(egui::epaint::Vertex {
        pos: rect.left_top(),
        uv,
        color: top,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: rect.right_top(),
        uv,
        color: top,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: rect.right_bottom(),
        uv,
        color: bottom,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: rect.left_bottom(),
        uv,
        color: bottom,
    });
    mesh.add_triangle(i, i + 1, i + 2);
    mesh.add_triangle(i, i + 2, i + 3);
    painter.add(egui::Shape::mesh(mesh));
}

fn draw_ground_grid(painter: &egui::Painter, camera: &OrbitCamera, rect: Rect) {
    let mvp = view_proj(camera, rect);
    let stroke = egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 28));
    let major = egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 55));
    for i in -6..=6 {
        let t = i as f32 * 0.2;
        let s = if i == 0 { major } else { stroke };
        if let (Some(a), Some(b)) = (
            project_vertex([-1.2, 0.0, t], mvp, rect),
            project_vertex([1.2, 0.0, t], mvp, rect),
        ) {
            painter.line_segment([a, b], s);
        }
        if let (Some(a), Some(b)) = (
            project_vertex([t, 0.0, -1.2], mvp, rect),
            project_vertex([t, 0.0, 1.2], mvp, rect),
        ) {
            painter.line_segment([a, b], s);
        }
    }
}

fn view_proj(camera: &OrbitCamera, rect: Rect) -> Mat4 {
    let eye = orbit_eye(camera);
    let target = Vec3::from_array(camera.target);
    let view = Mat4::look_at_rh(eye, target, Vec3::Y);
    let aspect = (rect.width() / rect.height().max(1.0)).max(0.1);
    let proj = Mat4::perspective_rh(45.0_f32.to_radians(), aspect, 0.01, 100.0);
    proj * view
}

fn orbit_eye(camera: &OrbitCamera) -> Vec3 {
    let cp = camera.pitch.cos();
    let sp = camera.pitch.sin();
    let cy = camera.yaw.cos();
    let sy = camera.yaw.sin();
    let dir = Vec3::new(cp * sy, sp, cp * cy);
    Vec3::from_array(camera.target) + dir * camera.distance
}

fn project_vertex(v: [f32; 3], mvp: Mat4, rect: Rect) -> Option<Pos2> {
    let clip = mvp * Vec4::new(v[0], v[1], v[2], 1.0);
    if clip.w <= 0.001 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    if !ndc.x.is_finite() || !ndc.y.is_finite() {
        return None;
    }
    Some(Pos2::new(
        rect.left() + (ndc.x * 0.5 + 0.5) * rect.width(),
        rect.top() + (1.0 - (ndc.y * 0.5 + 0.5)) * rect.height(),
    ))
}

fn view_depth(v: [f32; 3], mvp: Mat4) -> f32 {
    let clip = mvp * Vec4::new(v[0], v[1], v[2], 1.0);
    clip.z / clip.w.max(0.001)
}

fn draw_wireframe(
    painter: &egui::Painter,
    mesh: &MeshData,
    projected: &[Option<Pos2>],
    color: Color32,
) {
    use std::collections::HashSet;
    let stroke = egui::Stroke::new(1.0_f32, color);
    let mut edges = HashSet::new();

    let mut add_edge = |a: usize, b: usize| {
        if let (Some(pa), Some(pb)) = (
            projected.get(a).and_then(|p| *p),
            projected.get(b).and_then(|p| *p),
        ) {
            let key = if a < b { (a, b) } else { (b, a) };
            if edges.insert(key) {
                painter.line_segment([pa, pb], stroke);
            }
        }
    };

    let mut tri = |a: usize, b: usize, c: usize| {
        add_edge(a, b);
        add_edge(b, c);
        add_edge(c, a);
    };

    if mesh.indices.is_empty() {
        for i in (0..mesh.vertices.len()).step_by(3) {
            if i + 2 < mesh.vertices.len() {
                tri(i, i + 1, i + 2);
            }
        }
    } else {
        for chunk in mesh.indices.chunks(3) {
            if chunk.len() >= 3 {
                tri(chunk[0] as usize, chunk[1] as usize, chunk[2] as usize);
            }
        }
    }
}

fn draw_solid(painter: &egui::Painter, mesh: &MeshData, projected: &[Option<Pos2>], mvp: Mat4) {
    let key = Vec3::new(0.35, 0.75, 0.45).normalize();
    let fill = Vec3::new(-0.5, 0.3, -0.6).normalize();
    let ambient = 0.22_f32;
    let mut tris: Vec<(f32, [Pos2; 3], Color32)> = Vec::new();

    let push_tri = |a: usize,
                    b: usize,
                    c: usize,
                    out: &mut Vec<(f32, [Pos2; 3], Color32)>|
     -> Option<()> {
        let va = Vec3::from_array(*mesh.vertices.get(a)?);
        let vb = Vec3::from_array(*mesh.vertices.get(b)?);
        let vc = Vec3::from_array(*mesh.vertices.get(c)?);
        let pa = (*projected.get(a)?)?;
        let pb = (*projected.get(b)?)?;
        let pc = (*projected.get(c)?)?;
        let n = (vb - va).cross(vc - va).normalize_or_zero();
        if n.length_squared() < 1e-8 {
            return None;
        }
        let lit = (ambient + n.dot(key).max(0.0) * 0.65 + n.dot(fill).max(0.0) * 0.2).clamp(0.08, 1.0);
        let base = mesh.base_color;
        let color = Color32::from_rgb(
            ((base[0] * lit) * 255.0) as u8,
            ((base[1] * lit) * 255.0) as u8,
            ((base[2] * lit) * 255.0) as u8,
        );
        let depth = (view_depth(va.to_array(), mvp)
            + view_depth(vb.to_array(), mvp)
            + view_depth(vc.to_array(), mvp))
            / 3.0;
        out.push((depth, [pa, pb, pc], color));
        Some(())
    };

    if mesh.indices.is_empty() {
        for i in (0..mesh.vertices.len()).step_by(3) {
            if i + 2 < mesh.vertices.len() {
                let _ = push_tri(i, i + 1, i + 2, &mut tris);
            }
        }
    } else {
        for chunk in mesh.indices.chunks(3) {
            if chunk.len() >= 3 {
                let _ = push_tri(
                    chunk[0] as usize,
                    chunk[1] as usize,
                    chunk[2] as usize,
                    &mut tris,
                );
            }
        }
    }

    tris.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    for (_, pts, color) in tris {
        painter.add(egui::Shape::convex_polygon(
            pts.to_vec(),
            color,
            egui::Stroke::NONE,
        ));
    }
}

fn draw_axis_gizmo(painter: &egui::Painter, rect: Rect) {
    let origin = rect.left_bottom() + Vec2::new(28.0, -28.0);
    let len = 18.0_f32;
    painter.line_segment(
        [origin, origin + Vec2::new(len, 0.0)],
        egui::Stroke::new(2.0, Color32::from_rgb(0xEF, 0x44, 0x44)),
    );
    painter.line_segment(
        [origin, origin + Vec2::new(0.0, -len)],
        egui::Stroke::new(2.0, Color32::from_rgb(0x22, 0xC5, 0x5E)),
    );
    painter.line_segment(
        [origin, origin + Vec2::new(len * 0.55, -len * 0.55)],
        egui::Stroke::new(2.0, Color32::from_rgb(0x3B, 0x82, 0xF6)),
    );
}
