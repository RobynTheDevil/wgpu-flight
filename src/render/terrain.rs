#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unreachable_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_must_use)]

use std::collections::HashMap;
use wgpu::*;
use glam::*;
use crate::{
    gpu::Gpu,
    hasher::*,
    world::*,
    math::*,
};
use super::*;

// Triangle --------------------------{{{

#[derive(Clone, Copy)]
pub struct Triangle {
    pub verts: DMat4,
    pub tex: DMat3,
    pub normal: DVec3,
    pub color: Pixel,
    pub is_line: bool
}

impl Default for Triangle
{
    fn default() -> Self {
        Self {
            verts: dmat4(
                dvec4(0.0, 0.0, 0.0, 0.0),
                dvec4(0.0, 0.0, 0.0, 0.0),
                dvec4(0.0, 0.0, 0.0, 0.0),
                dvec4(0.0, 0.0, 0.0, 0.0),
            ),
            tex: dmat3(
                dvec3(0.0, 0.0, 0.0),
                dvec3(0.0, 0.0, 0.0),
                dvec3(0.0, 0.0, 0.0),
            ),
            normal: dvec3(0.0, 0.0, 0.0),
            color: Pixel{r:255, g:255, b:255, a: 0xFF},
            is_line: false,
        }
    }
}

impl Triangle
{
    pub fn new(verts: DMat4, color: Pixel) -> Self
    {
        Self {verts: verts, color: color, ..Default::default()}
    }

    // COPIES
    pub fn to_vertex(&self) -> [Vertex; 3] {
        [
            Vertex{
                position: self.verts.col(0).as_vec4().to_array(),
                normal: self.normal.as_vec3().extend(1.0).to_array(),
                color: [self.color.r as f32 / 255.0, self.color.g as f32 / 255.0, self.color.b as f32 / 255.0, 1.0],
            },
            Vertex{
                position: self.verts.col(1).as_vec4().to_array(),
                normal: self.normal.as_vec3().extend(1.0).to_array(),
                color: [self.color.r as f32 / 255.0, self.color.g as f32 / 255.0, self.color.b as f32 / 255.0, 1.0],
            },
            Vertex{
                position: self.verts.col(2).as_vec4().to_array(),
                normal: self.normal.as_vec3().extend(1.0).to_array(),
                color: [self.color.r as f32 / 255.0, self.color.g as f32 / 255.0, self.color.b as f32 / 255.0, 1.0],
            }
        ]
    }

    // COPIES
    pub fn to_array(&self) -> [u8; 3 * Vertex::size_of()] {
        let arr = unsafe { std::mem::transmute::<[Vertex; 3], [u8; 3 * Vertex::size_of()]>(self.to_vertex()) };
        arr
    }

    pub fn from_dvec3(a: DVec3, b: DVec3, c: DVec3) -> Self
    {
        Triangle
        {
            verts: dmat4
            (
                a.extend(1.0),
                b.extend(1.0),
                c.extend(1.0),
                dvec4(0.0, 0.0, 0.0, 0.0),
            ),
            ..Default::default()
        }
    }

    pub fn rotate(&mut self, angles: DVec3) { self.verts = mat_rotation(angles) * self.verts; }
    pub fn translate(&mut self, trans: DVec3) { self.verts = mat_translation(trans) * self.verts; }
    pub fn translate2d(&mut self, trans:DVec2) {
        for i in 0 .. 3 {
            self.verts.col(i).x += trans.x;
            self.verts.col(i).y += trans.y;
        }
    }
    pub fn transform(&mut self, trans: DMat4) { self.verts = trans * self.verts; }

    pub fn calc_normal(&mut self, normal_dir: f64)
    {
        let p0 = (self.verts.col(1) - self.verts.col(0)).truncate();
        let p1 = (self.verts.col(2) - self.verts.col(0)).truncate();
        self.normal = (p0.cross(p1)).normalize() * normal_dir;
    }

}

//}}}

// Mesh --------------------------{{{

pub struct Mesh {
    pub tris: Vec<Triangle>,
    pub normal_dir: f64,
    pub has_texture: bool,
    pub position: DVec3, // local offset
    pub rotation: DVec3, // local rotation
    pub preprocessed: bool,
}

impl Default for Mesh
{
    fn default() -> Self
    {
        Self
        {
            tris:vec![],
            normal_dir: 1.0,
            has_texture: false,
            position: dvec3(0.0, 0.0, 0.0),
            rotation: dvec3(0.0, 0.0, 0.0),
            preprocessed: false,
        }
    }
}

impl WorldObject for Mesh
{
    fn get_position(&self) -> DVec3 {self.position}
    fn get_rotation(&self) -> DMat4 {mat_rotation(self.rotation)}
}

impl Mesh
{

    // moderate extreme case of rows of infinite planes with chunk size = 16
    // 768kb, Expected 170 buckets (max buffer size is 128mb currently)
    // gives max 5 view distance with one buffer
    // const MAX_VERTS : u64 = 16 * 16 * 16 * 6;

    // target view distance 10, gives 128kb per chunk, 1024 buckets
    // given moderate extreme case above max chunk size = 8
    // we can align to 128kb and give an extra 2 verticies per voxel 6->8
    // const MAX_VERTS : u64 = 8 * 8 * 8 * 8; // 4096
    // const MAX_MEM_SIZE : usize = Self::MAX_VERTS as usize * Vertex::size_of();

    // added normals to vertex will count as x1.5 larger and return to 6 verts/vox
    // gives 144kb per chunk for ~910 buckets, view dist of 9
    pub const MAX_VERTS : u64 = 8 * 8 * 8 * 6; // 3072
    pub const MAX_MEM_SIZE : usize = Self::MAX_VERTS as usize * Vertex::size_of();

    // TODO: vertex inds will make above obsolete

    pub fn new(tris: Vec<Triangle>) -> Self
    {
        Self {tris: tris, ..Default::default()}
    }

    // COPIES TWICE
    pub fn to_array(&self) -> [u8; Self::MAX_MEM_SIZE] {
        let mut ret = [0; Self::MAX_MEM_SIZE];
        let mut i = 0;
        let step = 3 * Vertex::size_of();
        for tri in self.tris.iter(){
            if i + step >= Self::MAX_MEM_SIZE {break;}
            let verts = tri.to_array(); // TODO reduce copies
            ret[i..i + step].copy_from_slice(&verts);
            i += step;
        }
        ret
    }

    pub fn load_from_object_file(&mut self, filename: String)
    {
        let (models, _materials) =
            tobj::load_obj(
                &filename,
                &tobj::LoadOptions::default()
            )
            .expect("Failed to load OBJ file");

        let mesh = &models[0].mesh;
        let mut tris: Vec<Triangle> = Vec::with_capacity(100);
        for (n, idx) in mesh.indices.iter().enumerate().step_by(3)
        {
            let i0 = *idx as usize;
            let i1 = *(&mesh.indices[n+1]) as usize;
            let i2 = *(&mesh.indices[n+2]) as usize;
            let verts = dmat4(
                dvec4(
                    mesh.positions[3 * i0] as f64,
                    mesh.positions[3 * i0 + 1] as f64,
                    mesh.positions[3 * i0 + 2] as f64,
                    1.0,
                ),
                dvec4(
                    mesh.positions[3 * i1] as f64,
                    mesh.positions[3 * i1 + 1] as f64,
                    mesh.positions[3 * i1 + 2] as f64,
                    1.0,
                ),
                dvec4(
                    mesh.positions[3 * i2] as f64,
                    mesh.positions[3 * i2 + 1] as f64,
                    mesh.positions[3 * i2 + 2] as f64,
                    1.0,
                ),
                dvec4(0.0, 0.0, 0.0, 0.0),
            );
            if n < mesh.texcoord_indices.len()
            {
                let i0 = *(&mesh.texcoord_indices[n]) as usize;
                let i1 = *(&mesh.texcoord_indices[n+1]) as usize;
                let i2 = *(&mesh.texcoord_indices[n+2]) as usize;
                let tex = dmat3(
                    dvec3(
                        mesh.texcoords[2 * i0] as f64,
                        mesh.texcoords[2 * i0 + 1] as f64,
                        1.0,
                    ),
                    dvec3(
                        mesh.texcoords[2 * i1] as f64,
                        mesh.texcoords[2 * i1 + 1] as f64,
                        1.0,
                    ),
                    dvec3(
                        mesh.texcoords[2 * i2] as f64,
                        mesh.texcoords[2 * i2 + 1] as f64,
                        1.0,
                    ),
                );
                tris.push(Triangle{verts:verts, tex:tex, ..Default::default()});
            }
            else
            {
                tris.push(Triangle{verts:verts, ..Default::default()});
            }
        }

        self.tris = tris;
    }

    pub fn load_texture(&mut self, filename: String)
    {
        // TODO texturing
        // self.texture = olc::Sprite::from_image(&filename)
        //     .expect("Failed to load Texture file");
        self.has_texture = true;
    }

    // unlike general meshes, only preproc once bc attr will not change
    // skip rotation and translation
    pub fn preprocess_chunk_mesh(&mut self) {
        if self.preprocessed { return; }
        self.preprocessed = true;
        let position = self.get_position();
        for tri in self.tris.iter_mut() {
            tri.calc_normal(self.normal_dir);
        }
    }

    pub fn preprocess_mesh(&self, tris_to_raster: &mut Vec<Triangle>)
    {
        let position = self.get_position();
        let rotation = self.get_rotation();
        for tri in self.tris.iter()
        {
            // modelspace
            let mut tri = *tri; // copy

            // model rotation and translation to worldspace
            tri.transform(rotation);
            tri.translate(position);

            // get tri normal
            tri.calc_normal(self.normal_dir);
            tris_to_raster.push(tri);
        }
    }

}

//}}}

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
            vert_index: SeaHash::map(),
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
    pub global_uniform_buffer: Buffer,
    pub global_bind_group_layout: BindGroupLayout,
    pub global_bind_group: BindGroup,
    pub local_bind_group_layout: BindGroupLayout,
    pub local_bind_groups: HashMap<usize, BindGroup>,
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
            label: Some("Terrain Global Layout"),
        });
        let global_bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &global_bind_group_layout,
            entries: &[
                BindGroupEntry{
                    binding: 0,
                    resource: global_uniform_buffer.as_entire_binding(),
                },
            ],
            label: Some("Terrain Globals"),
        });

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

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            bind_group_layouts: &[
                &global_bind_group_layout,
                &local_bind_group_layout
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
            local_bind_group_layout,
            local_bind_groups: Default::default(),
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

    pub fn draw<'a: 'b, 'b>(&'a self, mut rpass: RenderPass<'b>)
    {
        rpass.set_pipeline(&self.render_pipeline);
        rpass.set_bind_group(0, &self.global_bind_group, &[]);
        rpass.set_vertex_buffer(0, self.vertex_buffer_world.slice(..));
        rpass.set_index_buffer(self.index_buffer_world.slice(..), IndexFormat::Uint32);
        rpass.draw_indexed(0..Gpu::max_inds() as u32, 0, 0..1);
    }

}

impl Pass for TerrainPass {
    fn render(&mut self, view: &TextureView, encoder: &mut CommandEncoder) -> Result<(), SurfaceError>
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
                     load: LoadOp::Clear(1.0),
                     store: true,
                 }),
                 stencil_ops: None,
             }),
            label: None,
        });
        self.draw(rpass);
        Ok(())
    }
}


