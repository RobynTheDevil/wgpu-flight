use std::collections::HashMap;
use wgpu::*;
use glam::*;
use crate::{
    gpu::Gpu,
    hasher::*,
    game::Game,
    world::*,
    math::*,
};
use super::{*, globals::Globals};

//{{{ ChunkMesh

pub struct ChunkMesh {
    // pub key: SeaHashKey,
    pub inds:  [u32; Self::MAX_INDEX as usize],
    pub verts: [Vertex; Self::MAX_VERTS as usize],
    pub vert_index: SeaHashMap<SeaHashKey, usize>,
    pub next_ind: usize,
    pub next_vert: usize,
}

impl Default for ChunkMesh {
    fn default() -> Self {
        Self {
            // key: World::coord2key(IVec3::ZERO),
            inds: [0; Self::MAX_INDEX as usize],
            verts: [Default::default(); Self::MAX_VERTS as usize],
            vert_index: SeaHashMap::new(),
            next_ind: 0,
            next_vert: 0,
        }
    }

}

impl ChunkMesh {
    const MAX_INDEX: u64 = Mesh::MAX_VERTS; // 3072
    const MAX_VERTS: u64 = 8 * 8 * 8;   //  512
    const MAX_INDEX_MEM: usize = Self::MAX_INDEX as usize * 4; // u32
    const MAX_VERTS_MEM: usize = Self::MAX_VERTS as usize * Vertex::size_of();

    pub fn new() -> Self {
        Self {
            // key,
            ..Default::default()
        }
    }

    pub fn add_positions(&mut self, verts: &[(IVec3, IVec3, SurfacePoint)]) {
        for i in (0 .. verts.len()).step_by(3) {
            if self.next_ind + 2 >= Self::MAX_INDEX as usize {return;}
            for j in 0..3 {
                // chunk coord, vox coord, position, normal
                let (c, v, sfp) = verts[i + j];
                let (ckey, vkey) = (World::coord2key(c), World::coord2key(c + v));
                let vi = self.vert_index.get(&vkey);
                match vi {
                    None => {
                        if self.next_vert >= Self::MAX_VERTS as usize {return;}
                        self.vert_index.insert(vkey, self.next_vert);
                        self.inds[self.next_ind] = self.next_vert as u32;
                        self.verts[self.next_vert] = Vertex{
                            position: [sfp.position.x as f32, sfp.position.y as f32, sfp.position.z as f32, 1.0],
                            normal: [sfp.normal.x as f32, sfp.normal.y as f32, sfp.normal.z as f32, 0.0],
                            ..Default::default()
                        };
                        self.next_vert += 1;
                    }
                    Some(vi) => {
                        self.inds[self.next_ind] = *vi as u32;
                    }
                }
                self.next_ind += 1;
            }
        }
    }

    pub fn calc_plain_normals(&self, ret: &mut [Vertex; Self::MAX_INDEX as usize]) {
        for i in (0 .. self.next_ind).step_by(3) {
            let (v0, v1, v2) = (
                Vec4::from_array(ret[i].position).truncate(),
                Vec4::from_array(ret[i+1].position).truncate(),
                Vec4::from_array(ret[i+2].position).truncate()
            );
            let normal = (v1 - v0).cross(v2 - v0).normalize().extend(0.0).to_array();
            ret[i].normal = normal;
            ret[i+1].normal = normal;
            ret[i+2].normal = normal;
        }
    }

    // indices are relative, need to adjust based on vertex buffer offset given from pool
    // will need to copy
    pub fn index_array(&self, offset: u32) -> [u8; Self::MAX_INDEX_MEM] {
        let mut ret = [0; Self::MAX_INDEX as usize];
        let mut i = 0;
        for ind in self.inds {
            ret[i] = offset + ind;
            i += 1;
        }
        let arr = unsafe { std::mem::transmute::<[u32; Self::MAX_INDEX as usize], [u8; Self::MAX_INDEX_MEM]>(ret) };
        arr
    }

    pub fn vertex_array(&self) -> &[u8; Self::MAX_VERTS_MEM] {
        let arr = unsafe { std::mem::transmute::<&[Vertex; Self::MAX_VERTS as usize], &[u8; Self::MAX_VERTS_MEM]>(&self.verts) };
        arr
    }

    pub const MAX_PLAIN_MEM : usize = Self::MAX_INDEX as usize * Vertex::size_of();

    pub fn plain_vertex_array(&self) -> [u8; Self::MAX_PLAIN_MEM] {
        let mut ret = [Default::default(); Self::MAX_INDEX as usize];
        for i in 0 .. self.inds.len() {
            ret[i] = self.verts[self.inds[i] as usize];
        }
        self.calc_plain_normals(&mut ret);
        let arr = unsafe { std::mem::transmute::<[Vertex; Self::MAX_INDEX as usize], [u8; Self::MAX_PLAIN_MEM]>(ret) };
        arr
    }

}
//}}}

pub struct TerrainConfig {

}

pub struct TerrainPass {
    //Uniforms, textures, render pipeline, camera, buffers
    pub globals: Globals,
    pub local_bind_group_layout: BindGroupLayout,
    pub local_bind_groups: HashMap<usize, BindGroup>,
    pub vertex_buffer_pool: BufferPool,
    pub index_buffer_pool: BufferPool,
    pub vertex_buffer_general: Buffer,
    pub vertex_buffer_world: Buffer,
    pub index_buffer_world: Buffer,
    pub depth_texture: SimpleTexture,
    pub render_pipeline: RenderPipeline,
    pub verts_count: u32,
}

impl TerrainPass {
    pub fn new(
        config: &TerrainConfig,
        device: &Device,
        surface_config: &SurfaceConfiguration,
    ) -> Self {
        // buffers{{{

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

        let depth_texture = SimpleTexture::create_depth_texture(device, surface_config, "depth_texture");

//}}}
        // bindgroups{{{

        let local_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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

//}}}

        let shader_desc = ShaderModuleDescriptor {
            source: ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("shader.wgsl"))),
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

        // Render Pipeline{{{
        let globals = Globals::new(device, surface_config);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            bind_group_layouts: &[
                &globals.bind_group_layout,
                //&local_bind_group_layout
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

//}}}

        Self {
            globals,
            local_bind_group_layout,
            local_bind_groups: Default::default(),
            vertex_buffer_pool: BufferPool::new(ChunkMesh::MAX_INDEX as usize * Vertex::size_of() as usize),
            index_buffer_pool: BufferPool::new(ChunkMesh::MAX_INDEX_MEM),
            vertex_buffer_general,
            vertex_buffer_world,
            index_buffer_world,
            depth_texture,
            render_pipeline,
            verts_count: 0,
        }
    }

}

impl Pass for TerrainPass {

    fn update(&mut self, queue: &Queue, game: &Game) {
        // general purpose triangles
        let mut i = 0;
        for tri in &game.get_tris_to_raster() {
            if i + 3 * Vertex::size_of() as u64 > Gpu::max_verts() { break; }
            let dat = tri.to_array();
            queue.write_buffer(&self.vertex_buffer_general, i, &dat);
            i += 3 * Vertex::size_of() as u64;
        }
        self.verts_count = i as u32;

        // world chunk triangles
        let (visible, updated) = game.world.get_chunks_to_write();
        for (key, mesh) in &visible {
            let v = self.vertex_buffer_pool.reserve(key, updated);
            match v {
                None => {continue;}
                Some(v) => {
                    queue.write_buffer(&self.vertex_buffer_world, v as u64, mesh.vertex_array());
                }
            }
            let i = self.index_buffer_pool.reserve(key, updated);
            match i {
                None => {continue;}
                Some(i) => {
                    let offset = (v.unwrap() / Vertex::size_of()) as u32;
                    queue.write_buffer(&self.index_buffer_world, i as u64, &mesh.index_array(offset));
                }
            }
        }
        // free chunks not visible
        if self.vertex_buffer_pool.len() < visible.len()
            || self.index_buffer_pool.len() < visible.len()
        {
            let removed = self.vertex_buffer_pool.keep_reserved(&visible);
            for i in removed {
                // queue.write_buffer(&self.vertex_buffer_world, i as u64, &[0; ChunkMesh::MAX_VERTS_MEM]);
            }
            let removed = self.index_buffer_pool.keep_reserved(&visible);
            for i in removed {
                queue.write_buffer(&self.index_buffer_world, i as u64, &[0; ChunkMesh::MAX_INDEX_MEM]);
            }
        }

        self.globals.update(queue, game);
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
        rpass.set_vertex_buffer(0, self.vertex_buffer_world.slice(..));
        rpass.set_index_buffer(self.index_buffer_world.slice(..), IndexFormat::Uint32);
        rpass.draw_indexed(0..Gpu::max_inds() as u32, 0, 0..1);
        Ok(())
    }
}

