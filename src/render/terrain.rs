use std::collections::HashMap;
use wgpu::*;
use glam::*;
use crate::{
    gpu::Gpu,
    math::{*,
        hasher::*,
        octree::*,
    }
};

use super::{*,
    globals::Globals,
};

pub struct TerrainConfig {

}

pub struct TerrainPass {
    //Uniforms, textures, render pipeline, camera, buffers
    pub globals: Globals,
    pub bind_group_layout: BindGroupLayout,
    pub bind_groups: HashMap<usize, BindGroup>,
    pub vertex_buffer_general: Buffer,
    pub buffers: IndexedBufferManager,
    pub depth_texture: SimpleTexture,
    pub render_pipeline: RenderPipeline,
    pub verts_count: u32,
}

impl TerrainPass {
// new {{{
    pub fn new(
        config: &TerrainConfig,
        device: &Device,
        surface_config: &SurfaceConfiguration,
    ) -> Self {
        let num_buffers = 10;
        let max_buffer_size = Limits::downlevel_defaults().max_buffer_size;

        // buffers

        let vertex_buffer_general = device.create_buffer(
            &BufferDescriptor {
                size: max_buffer_size,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
                label: Some("General Vertex Buffer"),
        });

        let depth_texture = SimpleTexture::create_depth_texture(device, surface_config, "depth_texture");

        // bindgroups

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(Vertex::size_of() as BufferAddress)
                    },
                    count: None,
                },
            ],
            label: Some("Terrain Local Layout"),
        });

        let shader_desc = ShaderModuleDescriptor {
            source: ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("terrain.wgsl"))),
            label: Some("shader"),
        };
        let shader = device.create_shader_module(shader_desc);

        let vertex_layouts = [
            VertexBufferLayout {
                array_stride: Vertex::size_of() as BufferAddress,
                step_mode: VertexStepMode::Vertex,
                attributes: &vertex_attr_array![
                    0 => Float32x4,
                    1 => Float32x4,
                    2 => Float32x4,
                ]
            }
        ];

        // Render Pipeline
        
        let globals = Globals::new(device, surface_config);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            bind_group_layouts: &[
                &globals.bind_group_layout,
                //&bind_group_layout
            ],
            push_constant_ranges: &[],
            label: Some("Render Pipeline Layout"),
        });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                buffers: &vertex_layouts,
                module: &shader,
                entry_point: "vs_main",
            },
            fragment: Some(FragmentState {
                targets: &[Some(ColorTargetState {
                    format: surface_config.format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                module: &shader,
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
            label: None,
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
            bind_groups: Default::default(),
            vertex_buffer_general,
            buffers: IndexedBufferManager::new(device, num_buffers),
            depth_texture,
            render_pipeline,
            verts_count: 0,
        }
    }
//}}}

}

impl Pass for TerrainPass {

    fn update(&mut self, queue: &Queue, gamedata: &GameData) {
        // general purpose triangles
        let mut i = 0;
        for tri in &gamedata.general_triangles {
            if i + 3 * Vertex::size_of() as u64 > Gpu::max_verts() { break; }
            let dat = tri.to_array();
            queue.write_buffer(&self.vertex_buffer_general, i, &dat);
            i += 3 * Vertex::size_of() as u64;
        }
        self.verts_count = i as u32;
        self.buffers.update(queue, gamedata);
        self.globals.update(queue, &gamedata.camera, &gamedata.light);
    }

    fn draw(&mut self, view: &TextureView, encoder: &mut CommandEncoder) -> Result<(), SurfaceError>
    {
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
        //TODO: general purpose triangles not drawn
        
        let max_inds = self.buffers.num_buckets as u32 * IndexedMesh::MAX_INDEX as u32;
        for i in 0..self.buffers.num_buffers {
            rpass.set_vertex_buffer(0, self.buffers.vertex_buffers[i].slice(..));
            rpass.set_index_buffer(self.buffers.index_buffers[i].slice(..), IndexFormat::Uint32);
            rpass.draw_indexed(0..max_inds, 0, 0..1);
        }

        Ok(())
    }
}

