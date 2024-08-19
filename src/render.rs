use std::collections::BinaryHeap;
use wgpu::*;
use glam::*;
use crate::{
    hasher::*,
    math::*,
    game::Game,
    octree::*,
    render::globals::*,
};

pub mod globals;
pub mod terrain;
pub mod sdf;

pub struct GameData<'a>
{
    pub general_triangles: Vec<Triangle>,
    pub visible_meshes: Vec<(SeaHashKey, &'a IndexedMesh)>,
    pub updated_mesh_keys: &'a SeaHashSet<SeaHashKey>,
    pub camera: CameraUniform,
    pub light: LightUniform,
}

pub trait Pass {
    fn update(&mut self, queue: &Queue, gamedata: &GameData) {}
    fn draw(&mut self, view: &TextureView, encoder: &mut CommandEncoder) -> Result<(), SurfaceError>;
}

#[derive(Clone, Copy)]
pub struct Pixel
{
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

//{{{ Vertex

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: [f32; 4],
    pub normal: [f32; 4],
    pub color: [f32; 4],
}

impl Default for Vertex{
    fn default() -> Self {
        Self {
            position: [0.0; 4],
            normal: [0.0; 4],
            color: [1.0; 4],
        }
    }
}

impl Vertex {
    pub const fn size_of() -> usize { std::mem::size_of::<Self>() }
}

//}}}

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
        let position = self.position;
        for tri in self.tris.iter_mut() {
            tri.calc_normal(self.normal_dir);
        }
    }

    pub fn preprocess_mesh(&self, tris_to_raster: &mut Vec<Triangle>)
    {
        let position = self.position;
        let rotation = mat_rotation(self.rotation);
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

//{{{ IndexedMesh

pub struct IndexedMesh {
    // pub key: SeaHashKey,
    pub inds:  [u32; Self::MAX_INDEX as usize],
    pub verts: [Vertex; Self::MAX_VERTS as usize],
    pub vert_index: SeaHashMap<SeaHashKey, usize>,
    pub next_ind: usize,
    pub next_vert: usize,
}

impl Default for IndexedMesh {
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

impl IndexedMesh {
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
                let (ckey, vkey) = (coord2key(c), coord2key(c + v));
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

//{{{ Light

pub struct Light
{
    pub ambient_color: DVec3,
    pub ambient_strength: f64,
    pub diffuse_color: DVec3,
    pub diffuse_strength: f64,
    pub specular_color: DVec3,
    pub specular_strength: f64,
    pub direction: DVec3,
}

impl Light
{
    pub fn new(acol: DVec3, astr: f64, dcol: DVec3, dstr: f64, scol: DVec3, sstr: f64) -> Self
    {
        Self
        {
            ambient_color: acol,
            ambient_strength: astr,
            diffuse_color: dcol,
            diffuse_strength: dstr,
            specular_color: scol,
            specular_strength: sstr,
            direction: dvec3(0.0, 0.0, 1.0), //arbitrary default
        }
    }

    pub fn to_light_uniform(&self) -> LightUniform {
        LightUniform {
            position: [1.0, 1.0, 1.0, 1.0],
            color: [1.0, 1.0, 1.0, 1.0],
            ambient_color_strength: self.ambient_color.extend(self.ambient_strength).as_vec4().to_array(),
            diffuse_color_strength: self.diffuse_color.extend(self.diffuse_strength).as_vec4().to_array(),
            specular_color_strength: self.specular_color.extend(self.specular_strength).as_vec4().to_array(),
            direction: self.direction.as_vec3().extend(1.0).to_array(),
        }
    }

    pub fn color2pixel(color: DVec3) -> Pixel
    {
        Pixel
        {
            r:(color.x * 255.0) as u8,
            g:(color.y * 255.0) as u8,
            b:(color.z * 255.0) as u8,
            a:0xFF
        }
    }

    pub fn get_pixel_illum (&self, illum: f64) -> Pixel
    {
        let plum = (self.ambient_color * self.ambient_strength)
            + (self.diffuse_color * self.diffuse_strength * illum);
        Self::color2pixel(plum)
    }

}

// }}}

//{{{ BucketPool

// virtual bufferpool
// does not actually track memory/buffers, only offsets and buffer number

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BucketCoord {
    buffer: u16,
    offset: u16,
}

type MinBinaryHeap<T> = BinaryHeap<std::cmp::Reverse<T>>;

pub struct BucketPool {
    // total number of pools (buffers)
    pub dims: u16,
    // total number of buckets
    pub size: u16,
    // tracks current number of dims loaded into pool (size)
    pub cur_dim: u16,
    pub pool: MinBinaryHeap<BucketCoord>,
    pub reserved: SeaHashMap<SeaHashKey, BucketCoord>,
}

impl BucketPool {

    #[inline]
    pub fn len(&self) -> usize {self.reserved.keys().len()}

    #[inline]
    pub fn is_expandable(&self) -> bool {self.dims > self.cur_dim}

    pub fn new(dims: u16, size: u16) -> Self {
        let mut s = Self {
            dims,
            size,
            cur_dim: 0,
            pool: MinBinaryHeap::with_capacity((dims * size) as usize),
            reserved: SeaHashMap::new(),
        };
        s.expand();
        s
    }

    pub fn expand(&mut self) -> bool {
        if ! self.is_expandable() {return false;}
        self.pool.append(
            &mut (0..self.size)
                .map(|x| std::cmp::Reverse(BucketCoord{buffer: self.cur_dim, offset: x}))
                .collect::<MinBinaryHeap<BucketCoord>>()
        );
        self.cur_dim += 1;
        true
    }

    pub fn reserve(&mut self, key: &SeaHashKey, force: &SeaHashSet<SeaHashKey>) -> Option<BucketCoord> {
        if self.reserved.contains_key(key) {
            if ! force.contains(key) {None}
            else {Some(*self.reserved.get(key).unwrap())}
        }
        else {
            if self.pool.is_empty() && ! self.expand() {None}
            else {
                let i = self.pool.pop().unwrap().0;
                self.reserved.insert(*key, i);
                Some(i)
            }
        }
    }

    // return removed
    pub fn keep_reserved(&mut self, keep: &Vec<(SeaHashKey, &IndexedMesh)>) -> Vec<BucketCoord> {
        let mut keep_reserved : SeaHashMap<SeaHashKey, BucketCoord> = SeaHashMap::new();
        let mut removed = vec![];
        for (k, v) in &self.reserved {
            if ! keep.iter().any(|x| x.0 == *k) {
                self.pool.push(std::cmp::Reverse(*v));
                removed.push(*v);
            } else {
                keep_reserved.insert(*k, *v);
            }
        }
        self.reserved = keep_reserved;
        removed
    }

}

//}}}

//{{{

pub struct IndexedBufferManager
{
    pub num_buffers: usize,
    pub vertex_buffer_size: usize,
    pub vertex_bucket_size: usize,
    pub index_buffer_size: usize,
    pub index_bucket_size: usize,
    pub num_buckets: usize,
    pub buckets: BucketPool,
    pub vertex_buffers: Vec<Buffer>,
    pub index_buffers: Vec<Buffer>,
}

impl IndexedBufferManager
{

    pub fn new(device: &Device, num_buffers: usize) -> Self {
        let vertex_bucket_size = IndexedMesh::MAX_VERTS_MEM;
        let vertex_buffer_size = Limits::downlevel_defaults().max_buffer_size as usize;
        let num_buckets = vertex_buffer_size / vertex_bucket_size;
        let index_bucket_size = IndexedMesh::MAX_INDEX_MEM;
        let index_buffer_size = index_bucket_size * num_buckets;

        let v_desc = &BufferDescriptor {
                size: vertex_buffer_size as u64,
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
                label: Some("Managed Vertex Buffer"),
        };

        let i_desc = &BufferDescriptor {
                size: index_buffer_size as u64,
                usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
                mapped_at_creation: false,
                label: Some("Managed Index Buffer"),
        };

        let mut vertex_buffers = vec![];
        let mut index_buffers = vec![];
        for i in 0..num_buffers {
            vertex_buffers.push(device.create_buffer(v_desc));
            index_buffers.push(device.create_buffer(i_desc));
        }

        Self {
            num_buffers,
            vertex_buffer_size,
            vertex_bucket_size,
            index_buffer_size,
            index_bucket_size,
            num_buckets,
            buckets: BucketPool::new(num_buffers as u16, num_buckets as u16),
            vertex_buffers,
            index_buffers,
        }
    }

    pub fn update(&mut self, queue: &Queue, gamedata: &GameData) {
        // world chunk triangles
        // index and vertex buffers correlated
        let (visible, updated) = (&gamedata.visible_meshes, &gamedata.updated_mesh_keys);
        for (key, mesh) in visible {
            let c = self.buckets.reserve(key, updated);
            match c {
                None => {continue;}
                Some(c) => {
                    let v_buffer = &self.vertex_buffers[c.buffer as usize];
                    let i_buffer = &self.index_buffers[c.buffer as usize];
                    let vmem_offset = c.offset as u64 * self.vertex_bucket_size as u64;
                    let imem_offset = c.offset as u64 * self.index_bucket_size as u64;
                    let vcount_offset = c.offset as u32 * IndexedMesh::MAX_VERTS as u32;
                    queue.write_buffer(v_buffer, vmem_offset, mesh.vertex_array());
                    queue.write_buffer(i_buffer, imem_offset, &mesh.index_array(vcount_offset as u32));
                }
            }
        }
        // free chunks not visible
        if self.buckets.len() > visible.len()
        {
            let removed = self.buckets.keep_reserved(visible);
            for c in removed {
                let buffer = &self.index_buffers[c.buffer as usize];
                let offset = c.offset as u64 * self.index_bucket_size as u64;
                queue.write_buffer(buffer, offset, &[0; IndexedMesh::MAX_INDEX_MEM]);
            }
        }
    }

}

//}}}

