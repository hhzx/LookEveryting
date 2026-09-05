//! wgpu mesh PaintCallback for egui (eframe wgpu backend).

use std::num::NonZeroU64;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use cap_model::MeshData;
use eframe::egui_wgpu::{wgpu, CallbackResources, CallbackTrait, ScreenDescriptor};
use glam::{Mat4, Vec3};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuVertex {
    position: [f32; 3],
    normal: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    mvp: [[f32; 4]; 4],
    light_dir: [f32; 4],
    base_color: [f32; 4],
}

/// Persistent GPU resources stored in egui_wgpu callback_resources.
pub struct MeshRenderResources {
    pipeline: wgpu::RenderPipeline,
    blit_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    blit_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    target_format: wgpu::TextureFormat,
    mesh: Option<UploadedMesh>,
    offscreen: Option<OffscreenTargets>,
    pending_mvp: [[f32; 4]; 4],
    pending_wireframe: bool,
}

struct UploadedMesh {
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    index_count: u32,
    uniform_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    base_color: [f32; 4],
}

struct OffscreenTargets {
    width: u32,
    height: u32,
    color_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    blit_bind_group: wgpu::BindGroup,
}

impl MeshRenderResources {
    pub fn create(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mesh3d"),
            source: wgpu::ShaderSource::Wgsl(MESH_SHADER.into()),
        });
        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mesh_blit"),
            source: wgpu::ShaderSource::Wgsl(BLIT_SHADER.into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mesh3d_uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(std::mem::size_of::<Uniforms>() as u64),
                },
                count: None,
            }],
        });

        let blit_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("mesh_blit"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mesh3d"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let blit_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mesh_blit"),
            bind_group_layouts: &[&blit_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mesh3d"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GpuVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mesh_blit"),
            layout: Some(&blit_layout),
            vertex: wgpu::VertexState {
                module: &blit_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &blit_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(target_format.into())],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            pipeline,
            blit_pipeline,
            bind_group_layout,
            blit_bind_group_layout,
            sampler,
            target_format,
            mesh: None,
            offscreen: None,
            pending_mvp: Mat4::IDENTITY.to_cols_array_2d(),
            pending_wireframe: false,
        }
    }

    pub fn upload_mesh(&mut self, device: &wgpu::Device, mesh: &MeshData) {
        let (vertices, indices) = build_gpu_mesh(mesh);
        if vertices.is_empty() || indices.is_empty() {
            self.mesh = None;
            return;
        }

        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh_vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh_indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh_uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mesh_uniforms"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        self.mesh = Some(UploadedMesh {
            vertex_buf,
            index_buf,
            index_count: indices.len() as u32,
            uniform_buf,
            bind_group,
            base_color: mesh.base_color,
        });
    }

    pub fn clear_mesh(&mut self) {
        self.mesh = None;
    }

    fn ensure_offscreen(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        if self
            .offscreen
            .as_ref()
            .is_some_and(|o| o.width == width && o.height == height)
        {
            return;
        }

        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("mesh_color"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.target_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("mesh_depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let color_view = color.create_view(&Default::default());
        let depth_view = depth.create_view(&Default::default());
        let blit_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mesh_blit_bg"),
            layout: &self.blit_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        self.offscreen = Some(OffscreenTargets {
            width,
            height,
            color_view,
            depth_view,
            blit_bind_group,
        });
    }
}

use wgpu::util::DeviceExt;

fn build_gpu_mesh(mesh: &MeshData) -> (Vec<GpuVertex>, Vec<u32>) {
    let mut indices: Vec<u32> = if mesh.indices.is_empty() {
        (0..mesh.vertices.len() as u32).collect()
    } else {
        mesh.indices.clone()
    };
    // Drop incomplete trailing triangle.
    let n = indices.len() - (indices.len() % 3);
    indices.truncate(n);

    let mut normals = vec![Vec3::ZERO; mesh.vertices.len()];
    for tri in indices.chunks_exact(3) {
        let ia = tri[0] as usize;
        let ib = tri[1] as usize;
        let ic = tri[2] as usize;
        let Some(a) = mesh.vertices.get(ia).copied() else {
            continue;
        };
        let Some(b) = mesh.vertices.get(ib).copied() else {
            continue;
        };
        let Some(c) = mesh.vertices.get(ic).copied() else {
            continue;
        };
        let va = Vec3::from_array(a);
        let vb = Vec3::from_array(b);
        let vc = Vec3::from_array(c);
        let nrm = (vb - va).cross(vc - va).normalize_or_zero();
        normals[ia] += nrm;
        normals[ib] += nrm;
        normals[ic] += nrm;
    }

    let vertices: Vec<GpuVertex> = mesh
        .vertices
        .iter()
        .enumerate()
        .map(|(i, p)| GpuVertex {
            position: *p,
            normal: normals[i].normalize_or_zero().to_array(),
        })
        .collect();
    (vertices, indices)
}

/// Per-frame paint callback.
pub struct MeshPaintCallback {
    pub mvp: Mat4,
    pub wireframe: bool,
    pub size_px: (u32, u32),
    pub mesh_to_upload: Option<Arc<MeshData>>,
    pub clear_mesh: bool,
}

impl CallbackTrait for MeshPaintCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &ScreenDescriptor,
        encoder: &mut wgpu::CommandEncoder,
        resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let Some(res) = resources.get_mut::<MeshRenderResources>() else {
            return Vec::new();
        };

        if self.clear_mesh {
            res.clear_mesh();
        }
        if let Some(mesh) = self.mesh_to_upload.as_ref() {
            res.upload_mesh(device, mesh);
        }

        res.pending_mvp = self.mvp.to_cols_array_2d();
        res.pending_wireframe = self.wireframe;
        res.ensure_offscreen(device, self.size_px.0, self.size_px.1);

        let Some(mesh) = res.mesh.as_ref() else {
            return Vec::new();
        };
        let Some(off) = res.offscreen.as_ref() else {
            return Vec::new();
        };

        let uniforms = Uniforms {
            mvp: res.pending_mvp,
            light_dir: [0.35, 0.75, 0.45, 0.0],
            base_color: mesh.base_color,
        };
        queue.write_buffer(&mesh.uniform_buf, 0, bytemuck::bytes_of(&uniforms));

        if !res.pending_wireframe {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("mesh3d_offscreen"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &off.color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &off.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&res.pipeline);
            pass.set_bind_group(0, &mesh.bind_group, &[]);
            pass.set_vertex_buffer(0, mesh.vertex_buf.slice(..));
            pass.set_index_buffer(mesh.index_buf.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
        }

        Vec::new()
    }

    fn paint(
        &self,
        info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        resources: &CallbackResources,
    ) {
        let Some(res) = resources.get::<MeshRenderResources>() else {
            return;
        };
        if res.pending_wireframe || res.mesh.is_none() {
            return;
        }
        let Some(off) = res.offscreen.as_ref() else {
            return;
        };

        let vp = info.viewport_in_pixels();
        let left = vp.left_px.max(0) as f32;
        let top = vp.top_px.max(0) as f32;
        let width = vp.width_px.max(1) as f32;
        let height = vp.height_px.max(1) as f32;
        render_pass.set_viewport(left, top, width, height, 0.0, 1.0);
        render_pass.set_scissor_rect(
            vp.left_px.max(0) as u32,
            vp.top_px.max(0) as u32,
            vp.width_px.max(1) as u32,
            vp.height_px.max(1) as u32,
        );
        render_pass.set_pipeline(&res.blit_pipeline);
        render_pass.set_bind_group(0, &off.blit_bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

const MESH_SHADER: &str = r#"
struct Uniforms {
    mvp: mat4x4<f32>,
    light_dir: vec4<f32>,
    base_color: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: Uniforms;

struct VsIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};
struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) normal: vec3<f32>,
};

@vertex
fn vs_main(v: VsIn) -> VsOut {
    var o: VsOut;
    o.clip_pos = u.mvp * vec4<f32>(v.position, 1.0);
    o.normal = normalize(v.normal);
    return o;
}

@fragment
fn fs_main(i: VsOut) -> @location(0) vec4<f32> {
    let n = normalize(i.normal);
    let l = normalize(u.light_dir.xyz);
    let ambient = 0.22;
    let diff = max(dot(n, l), 0.0) * 0.7;
    let fill = max(dot(n, normalize(vec3<f32>(-0.5, 0.3, -0.6))), 0.0) * 0.25;
    let intensity = clamp(ambient + diff + fill, 0.12, 1.0);
    return vec4<f32>(u.base_color.rgb * intensity, 1.0);
}
"#;

const BLIT_SHADER: &str = r#"
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var o: VsOut;
    let p = positions[idx];
    o.pos = vec4<f32>(p, 0.0, 1.0);
    o.uv = vec2<f32>(p.x * 0.5 + 0.5, 1.0 - (p.y * 0.5 + 0.5));
    return o;
}

@fragment
fn fs_main(i: VsOut) -> @location(0) vec4<f32> {
    return textureSample(tex, samp, i.uv);
}
"#;
