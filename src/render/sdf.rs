
use std::collections::HashMap;
use wgpu::*;
use glam::*;
use crate::{
    gpu::Gpu,
    hasher::*,
    game::Game,
    octree::*,
    math::*,
    world::{*,
        sdftest::*,
    },
};
use super::{*,
    globals::Globals,
};

pub struct SdfPass {
    //Uniforms, textures, render pipeline, camera, buffers
    pub globals: Globals,
    pub bind_group_layout: BindGroupLayout,
    pub bind_group: BindGroup,
    pub compute_buffer: Buffer,
    pub vertex_buffer: Buffer,
    pub depth_texture: SimpleTexture,
    pub compute_pipeline: ComputePipeline,
    pub render_pipeline: RenderPipeline,
}

impl SdfPass {
    pub fn new(
        device: &Device,
        surface_config: &SurfaceConfiguration,
    ) -> Self {
        let compute_shader_desc = ShaderModuleDescriptor {
            source: ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("sdfcompute.wgsl"))),
            label: Some("Sdf Compute Shader"),
        };
        let compute_shader = device.create_shader_module(compute_shader_desc);

        let render_shader_desc = ShaderModuleDescriptor {
            source: ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("sdfrender.wgsl"))),
            label: Some("Sdf Render Shader"),
        };
        let render_shader = device.create_shader_module(render_shader_desc);

        let compute_buffer = device.create_buffer(
            &BufferDescriptor {
                size: Limits::downlevel_defaults().max_buffer_size / 2,
                usage: BufferUsages::STORAGE
                    | BufferUsages::COPY_DST,
                mapped_at_creation: false,
                label: Some("Sdf Compute Buffer"),
        });

        let vertex_buffer = device.create_buffer(
            &BufferDescriptor {
                size: Limits::downlevel_defaults().max_buffer_size / 2,
                usage: BufferUsages::STORAGE
                    | BufferUsages::VERTEX
                    | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                label: Some("Sdf Vertex Buffer"),
        });

        let depth_texture = SimpleTexture::create_depth_texture(device, surface_config, "depth_texture");

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage {read_only: true},
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(Mesh::MAX_MEM_SIZE as BufferAddress)
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE | ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage {read_only: false},
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(Mesh::MAX_MEM_SIZE as BufferAddress)
                    },
                    count: None,
                },
            ],
            label: Some("Sdf Local Layout"),
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Sdf Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: compute_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: vertex_buffer.as_entire_binding(),
                },
            ],
        });

        let vertex_layouts = [
            VertexBufferLayout {
                array_stride: 16 as BufferAddress,
                step_mode: VertexStepMode::Vertex,
                attributes: &vertex_attr_array![
                    0 => Float32x4,
                ]
            }
        ];

        let globals = Globals::new(device, surface_config);

        // Compute Pipeline

        let compute_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            bind_group_layouts: &[
                &globals.bind_group_layout,
                &bind_group_layout
            ],
            push_constant_ranges: &[],
            label: Some("Sdf Compute Pipeline Layout"),
        });

        let compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("Sdf Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "cs_main",
        });

        // Render Pipeline

        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            bind_group_layouts: &[
                &globals.bind_group_layout,
            ],
            push_constant_ranges: &[],
            label: Some("Sdf Render Pipeline Layout"),
        });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Sdf Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                buffers: &vertex_layouts,
                module: &render_shader,
                entry_point: "vs_main",
            },
            fragment: Some(FragmentState {
                targets: &[Some(ColorTargetState {
                    format: surface_config.format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                module: &render_shader,
                entry_point: "fs_main",
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Front),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: match Some(SimpleTexture::DEPTH_FORMAT) {
                None => None,
                Some(f) => Some(DepthStencilState {
                    format: f,
                    depth_write_enabled: true,
                    depth_compare: CompareFunction::Greater,
                    stencil: StencilState::default(),
                    bias: DepthBiasState::default(),
                }),
            },
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            globals,
            bind_group_layout,
            bind_group,
            compute_buffer,
            vertex_buffer,
            depth_texture,
            compute_pipeline,
            render_pipeline,
        }
    }

}

impl Pass for SdfPass {

    fn update(&mut self, queue: &Queue, gamedata: &GameData) {

        // load sdf directly into compute buffer
        // let data = game.world.get_data();
        //        queue.write_buffer(&self.vertex_buffer_world, v as u64, mesh.vertex_array());

        self.globals.update(queue, &gamedata.camera, &gamedata.light);
    }

    fn draw(&mut self, view: &TextureView, encoder: &mut CommandEncoder) -> Result<(), SurfaceError>
    {
        // compute pass
        let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {label: None});
        cpass.set_pipeline(&self.compute_pipeline);
        cpass.set_bind_group(0, &self.globals.bind_group, &[]);
        cpass.set_bind_group(1, &self.bind_group, &[]);
        cpass.dispatch_workgroups(4, 4, 4);
        drop(cpass);
        // render pass
        let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: Operations {
                    //load: LoadOp::Clear(Color::BLUE),
                    load: LoadOp::Clear(Color {r:0.1, g:0.2, b:0.3, a:1.0}),
                    store: true,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                 view: &self.depth_texture.view,
                 depth_ops: Some(Operations {
                     load: LoadOp::Clear(0.0),
                     store: true,
                 }),
                 stencil_ops: None,
             }),
            label: None,
        });
        rpass.set_pipeline(&self.render_pipeline);
        rpass.set_bind_group(0, &self.globals.bind_group, &[]);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.draw(0..Gpu::max_verts() as u32, 0..1);
        drop(rpass);
        Ok(())
    }
}

