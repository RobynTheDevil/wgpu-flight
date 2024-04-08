#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unreachable_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_must_use)]

use std::collections::HashMap;
use wgpu::*;
use crate::gpu::Gpu;
use super::*;

//{{{ SimpleTexture

pub struct SimpleTexture {
    pub texture: Texture,
    pub view: TextureView,
    pub sampler: Sampler,
}

impl SimpleTexture {
    pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

    pub fn create_depth_texture(device: &Device, config: &SurfaceConfiguration, label: &str) -> Self {
        let size = Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };
        let desc = TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(
            &SamplerDescriptor {
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                address_mode_w: AddressMode::ClampToEdge,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mipmap_filter: FilterMode::Nearest,
                compare: Some(CompareFunction::LessEqual),
                lod_min_clamp: 0.0,
                lod_max_clamp: 100.0,
                ..Default::default()
            }
        );

        Self {
            texture,
            view,
            sampler,
        }
    }
}

//}}}

pub struct TerrainConfig {

}

pub struct TerrainPass {
    //Uniforms, textures, render pipeline, camera, buffers
    pub global_uniform_buffer: Buffer,
    pub global_bind_group_layout: BindGroupLayout,
    pub global_bind_group: BindGroup,
    //pub local_bind_group_layout: BindGroupLayout,
    //pub local_bind_groups: HashMap<usize, BindGroup>,
    pub vertex_buffer_general: Buffer,
    pub vertex_buffer_world: Buffer,
    pub index_buffer_world: Buffer,
    pub depth_texture: SimpleTexture,
    pub render_pipeline: RenderPipeline,
    //pub instance_buffers: HashMap<usize, Buffer>,
}

impl TerrainPass {
    pub fn new(
        config: &TerrainConfig,
        device: &Device,
        surface_config: &SurfaceConfiguration,
    ) -> Self {
        // buffers{{{
        let global_uniform_buffer = device.create_buffer(
            &BufferDescriptor {
                size: (CameraUniform::size_of()
                    + LightUniform::size_of()) as u64,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
                label: Some("Camera and Light Uniform"),
        });

        let vertex_buffer_general = device.create_buffer(
            &BufferDescriptor {
                size: Limits::downlevel_defaults().max_buffer_size,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
                label: Some("General Vertex Buffer"),
        });

        let vertex_buffer_world = device.create_buffer(
            &BufferDescriptor {
                size: Limits::downlevel_defaults().max_buffer_size,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
                label: Some("World Chunk Vertex Buffer"),
        });
        let index_buffer_world = device.create_buffer(
            &BufferDescriptor {
                size: Limits::downlevel_defaults().max_buffer_size,
                usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
                label: Some("World Chunk Index Buffer"),
        });

        //let index_buffer_pool = BufferPool::new(ChunkMesh::MAX_INDEX_MEM);
        //let vertex_buffer_pool = BufferPool::new(ChunkMesh::MAX_INDEX as usize * Vertex::size_of() as usize);
        let depth_texture = SimpleTexture::create_depth_texture(device, surface_config, "depth_texture");

//}}}
        // bindgroups{{{

        let global_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                // camera and light uniform
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("bind_group_layout"),
        });
        let global_bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &global_bind_group_layout,
            entries: &[
                BindGroupEntry{
                    binding: 0,
                    resource: global_uniform_buffer.as_entire_binding(),
                },
            ],
            label: Some("bind_group"),
        });

//}}}

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            bind_group_layouts: &[
                &global_bind_group_layout
            ],
            push_constant_ranges: &[],
            label: Some("Render Pipeline Layout"),
        });

        let shader_desc = ShaderModuleDescriptor {
            source: ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("shader.wgsl"))),
            label: Some("shader"),
        };

        let render_pipeline = Self::create_render_pipeline(
            device,
            &pipeline_layout,
            surface_config.format,
            Some(SimpleTexture::DEPTH_FORMAT),
            &[
                VertexBufferLayout {
                    array_stride: Vertex::size_of() as BufferAddress,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &vertex_attr_array![
                        0 => Float32x4,
                        1 => Float32x4,
                        2 => Float32x4,
                    ]
                }
            ],
            shader_desc,
        );

        Self {
            global_uniform_buffer,
            global_bind_group_layout,
            global_bind_group,
            //local_bind_group_layout,
            //local_bind_groups,
            vertex_buffer_general,
            vertex_buffer_world,
            index_buffer_world,
            depth_texture,
            render_pipeline,
        }
    }

    pub fn create_render_pipeline(
        device: &Device,
        pipeline_layout: &PipelineLayout,
        color_format: TextureFormat,
        depth_format: Option<TextureFormat>,
        vertex_layouts: &[VertexBufferLayout],
        shader_desc: ShaderModuleDescriptor,
    ) -> RenderPipeline {
        let shader = device.create_shader_module(shader_desc);

        device.create_render_pipeline(&RenderPipelineDescriptor {
            layout: Some(pipeline_layout),
            vertex: VertexState {
                buffers: vertex_layouts,
                module: &shader,
                entry_point: "vs_main",
            },
            fragment: Some(FragmentState {
                targets: &[Some(ColorTargetState {
                    format: color_format,
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
            depth_stencil: match depth_format {
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
        })
    }

    pub fn draw_mesh<'a: 'b, 'b>(&'a self, mut rpass: RenderPass<'b>)
    {
        rpass.set_pipeline(&self.render_pipeline);
        rpass.set_bind_group(0, &self.global_bind_group, &[]);
        rpass.set_vertex_buffer(0, self.vertex_buffer_world.slice(..));
        rpass.set_index_buffer(self.index_buffer_world.slice(..), IndexFormat::Uint32);
        rpass.draw_indexed(0..Gpu::max_inds() as u32, 0, 0..1);
    }

}

impl Pass for TerrainPass {
    fn draw(&mut self, frame: &SurfaceTexture, encoder: &mut CommandEncoder) -> Result<(), SurfaceError>
    {
        // render pass
        {
            let output = frame.texture.create_view(&TextureViewDescriptor::default());
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &output,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLUE),
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
            self.draw_mesh(rpass);
        }
        Ok(())
    }
}


