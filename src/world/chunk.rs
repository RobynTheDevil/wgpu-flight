#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unreachable_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_must_use)]

use crate::{
    math::*,
    direction::*,
    world::*,
    octree::*,
    generator::DistanceField,
    render::*,
};
use glam::*;

//{{{ WorldChunk

pub struct WorldChunk {
    pub coord: IVec3,
    pub degree: u8,
    pub midpoint: IVec3,
    pub scale: f64,
    pub sample_scale: f64,
    pub sdftree: SDFOctree,
}

impl WorldChunk {

    const MAX_RESOLUTION: u8 = 1; // max relative degree to sample sdf

    // absolute maximum degree is 21 as per octree depth limit (1 + 63 bit loc code)
    // center root midpoint at origin for world coord calculation
    // loccode -> IVec3 21 bits per axis chunk space -> (v - node midpoint) * scale + offset = worldspace
    pub fn new(chunk_coord: IVec3, scale: f64, sample_scale: f64, degree: u8, df: &DistanceField) -> Self {
        let mut sdftree = SDFOctree::new(degree);
        let mp_ax = if degree > 0 {1 << (degree - 1)} else {0};
        let midpoint = ivec3(mp_ax, mp_ax, mp_ax);
        let mut ret = Self {
            coord: chunk_coord,
            degree,
            midpoint,
            scale,
            sample_scale,
            sdftree,
        };
        ret.sample_df(0b1, df);
        ret
    }

    // magic number morton encoding/decoding{{{

    pub const _FF : u64 = 0x7FFF_FFFF_FFFF_FFFF;
    pub const _L2C_X : u64 = 0x1249_2492_4924_9249; // 0b0_001_001_...
    pub const _L2C_Y : u64 = 0x2492_4924_9249_2492; // 0b0_010_010_...
    pub const _L2C_Z : u64 = 0x4924_9249_2492_4924; // 0b0_100_100_...
    pub const _L2C_B : [u64; 6] = [
        0x0000_0000_001F_FFFF, // first 21 bits
        0x001F_0000_0000_FFFF, // 0b00000000_00011111_00000000_00000000_00000000_00000000_11111111_11111111
        0x001F_0000_FF00_00FF, // 0b00000000_00011111_00000000_00000000_11111111_00000000_00000000_11111111
        0x100F_00F0_0F00_F00F, // 0b00010000_00001111_00000000_11110000_00001111_00000000_11110000_00000000
        0x10C3_0C30_C30C_30C3, // 0b00010000_11000011_00001100_00110000_11000011_00001100_00110001_00000000
        0x1249_2492_4924_9249  // 0b00010010_00101001_00100100_10010010_01001001_00100100_10010010_01001001
    ];

    pub fn splitby3(a: u32) -> u64 {
        let mut a = (a as u64) & Self::_L2C_B[0];
        a = (a | a << 32) & Self::_L2C_B[1];
        a = (a | a << 16) & Self::_L2C_B[2];
        a = (a | a << 8)  & Self::_L2C_B[3];
        a = (a | a << 4)  & Self::_L2C_B[4];
        a = (a | a << 2)  & Self::_L2C_B[5];
        a
    }

    // https://graphics.stanford.edu/~seander/bithacks.html#InterleaveBMN
    //chunkspace coord (assume all > 0), assume self.degree for return
    pub fn coord2loc(&self, coord: IVec3) -> u64 {
        let (mut x, mut y, mut z) = (
            coord.x as u32, coord.y as u32, coord.z as u32
        );
        let (x, y, z) = (
            Self::splitby3(coord.x as u32),
            Self::splitby3(coord.y as u32),
            Self::splitby3(coord.z as u32)
        );
        let r = x | (y << 1) | (z << 2);
        //flag bit denotes chunk degree
        let zeros = r.leading_zeros() as u8;
        let m = std::cmp::max(self.degree * 3, 63 + (zeros % 3) - zeros);
        r | (1 << m)
    }

    pub fn thirdbits(m: u64) -> u64 {
        let mut m = m & Self::_L2C_B[5];
        m = (m ^ (m >> 2))  & Self::_L2C_B[4];
        m = (m ^ (m >> 4))  & Self::_L2C_B[3];
        m = (m ^ (m >> 8))  & Self::_L2C_B[2];
        m = (m ^ (m >> 16)) & Self::_L2C_B[1];
        m = (m ^ (m >> 32)) & Self::_L2C_B[0];
        m
    }

    //chunkspace coord with origin at BDL
    // assume degree <= self.degree
    pub fn loc2coord(&self, loc: u64) -> IVec3 {
        let zeros = loc.leading_zeros();
        let degree = (21 - (zeros + 1) / 3) as u8;
        let rel_degree = self.degree - degree; // assume positive or zero
        let degree_mask = Self::_FF >> zeros;
        let oloc = loc;
        let loc = loc & degree_mask;
        let (x, y, z) = (
            loc & Self::_L2C_X,
            (loc & Self::_L2C_Y) >> 1,
            (loc & Self::_L2C_Z) >> 2,
        );
        let x = Self::thirdbits(x) << rel_degree;
        let y = Self::thirdbits(y) << rel_degree;
        let z = Self::thirdbits(z) << rel_degree;
        ivec3(x as i32, y as i32, z as i32)
    }

//}}}

    // given relative coord (BDL at origin)
    // uses BDL corner as voxel/node point (except for root)
    pub fn coord2pos(&self, coord: IVec3) -> DVec3 {
        let offset = self.coord * (1 << self.degree);
        let pos = (coord - self.midpoint + offset).as_dvec3();
        pos
    }

    pub fn sample_df_value(&self, loc: u64, df: &DistanceField) -> u8 {
        // chunk relative coord unit 1 at degree = self.degree
        let coord = self.loc2coord(loc);
        // to worldspace
        let pos = self.coord2pos(coord);
        let v = df.gen(pos * self.sample_scale);
        // println!("{} {:016b} {}", coord, loc, v);
        v
    }

    pub fn _sample_df(&mut self, loc: u64, degree: u8, df: &DistanceField) -> SDFNode {
        let mut mask = 0b0;
        let value = self.sample_df_value(loc, df);
        if self.degree - degree >= Self::MAX_RESOLUTION {
            let loc = loc << 3;
            for d in 0 .. 8 {
                let loc = loc | d;
                let child = self._sample_df(loc, degree + 1, df);
                if child.mask != 0 || child.value != value {
                    self.sdftree.insert(loc, child);
                    mask |= 1 << d;
                }
            }
        }
        SDFNode{
            mask,
            value,
        }
    }

    pub fn sample_df(&mut self, loc: u64, df: &DistanceField) {
        if self.sdftree.contains_key(&loc) {return;}
        let degree = (21 - (loc.leading_zeros() + 1) / 3) as u8;
        let node = self._sample_df(loc, degree, df);
        self.sdftree.insert(loc, node);
    }

    // relative coord (BDL at origin)
    pub fn get_voxel_by_coord(&self, coord: IVec3) -> u8 {
        let loc = self.coord2loc(coord);
        // println!("get {} {:064b}", coord, loc);
        let node = self.sdftree.get_node_or_parent(loc);
        node.value
    }

    // (index to dirs and by extension neighbors in caller, coord relative, coord orig)
    // assume dirs are ordered properly (ie positive dirs)
    pub fn neighbor_coords(&self, coord: IVec3, dirs: &[IVec3]) -> Vec<(usize, IVec3)>
    {
        let mut ret = vec![(0, ivec3(0, 0, 0)); dirs.len() as usize];
        let chunk_size = 1 << self.degree;
        for i in 0 .. dirs.len()
        {
            let dir = dirs[i];
            let mut coord_cur = coord + dir;
            let mut dir_ind = 0b0 as usize;
            if coord_cur.x >= chunk_size || coord_cur.x < 0
            {
                dir_ind |= 0b001;
                coord_cur.x = (coord_cur.x + chunk_size) % chunk_size;
            }
            if coord_cur.y >= chunk_size || coord_cur.y < 0
            {
                dir_ind |= 0b010;
                coord_cur.y = (coord_cur.y + chunk_size) % chunk_size;
            }
            if coord_cur.z >= chunk_size || coord_cur.z < 0
            {
                dir_ind |= 0b100;
                coord_cur.z = (coord_cur.z + chunk_size) % chunk_size;
            }
            //zyx bits to pos dirs ordering
            dir_ind = IDirection::BITWISE_TO_DIRS[dir_ind];
            ret[i] = (dir_ind, coord_cur);
        }
        ret
    }

    // pass in neighbor chunks, same length as dirs (ind zero/self always null)
    pub fn neighbor_dist(&self, coord: IVec3, dirs: &[IVec3], neighbors: &Vec<Option<&WorldChunk>>) -> Vec<f64>
    {
        let mut dists = vec![0.0; dirs.len()];
        let n_coords = self.neighbor_coords(coord, dirs);
        for i in 0 .. n_coords.len()
        {
            let dir_ind = n_coords[i].0;
            let chunk = neighbors[dir_ind];
            let coord = n_coords[i].1;
            dists[i] = (
                match chunk {
                    None => self.get_voxel_by_coord(coord),
                    Some(chunk) => chunk.get_voxel_by_coord(coord)
                } as i32 - 128
            ) as f64;
        }
        dists
    }

}

//}}}

// ChunkManager
// is essentially a copy of bobbinsworld
// decoupled from player
// TODO: generalize queue actions and hashmap objects to one object

pub struct ChunkManager
{
    pub chunk_size: i32,
    pub chunk_degree: u8,
    pub chunk_sample_scale: f64,
    pub chunk_scale: f64,
    pub chunks: SeaHashMap<SeaHashKey, WorldChunk>,
    pub surface_maps: SeaHashMap<SeaHashKey, SurfaceOctree>,
    pub meshes: SeaHashMap<SeaHashKey, IndexedMesh>,
    pub view_dist: i32,
    pub gen_dist: i32,
    pub operations_per_frame: i32,
    pub queue_chunk: Vec<IVec3>,
    pub queue_sfp: Vec<IVec3>,
    pub queue_mesh: Vec<IVec3>,
    pub operation_pending: SeaHashSet<SeaHashKey>,
    pub chunk_updated: SeaHashSet<SeaHashKey>,
    pub distance_field: DistanceField,
}

impl ChunkManager
{
    pub fn new() -> Self
    {
        let chunk_degree = 3;
        Self
        {
            chunk_size: 1 << chunk_degree,
            chunk_degree: chunk_degree as u8,
            chunk_sample_scale: 0.1,
            chunk_scale: 1.0,
            chunks: SeaHashMap::new(),
            surface_maps: SeaHashMap::new(),
            meshes: SeaHashMap::new(),
            view_dist: 10,
            gen_dist: 10,
            operations_per_frame: 20,
            queue_chunk: Vec::<IVec3>::with_capacity(100),
            queue_sfp: Vec::<IVec3>::with_capacity(100),
            queue_mesh: Vec::<IVec3>::with_capacity(100),
            operation_pending: SeaHashSet::new(),
            chunk_updated: SeaHashSet::new(),
            distance_field: DistanceField::new(),
        }
    }

    #[inline]
    pub fn chunk_coord2key(&self, coord: IVec3) -> SeaHashKey {
        let coord = coord * self.chunk_size;
        coord2key(coord)
    }

    pub fn create_chunk(&mut self, chunk_coord: IVec3)
    {
        let key = self.chunk_coord2key(chunk_coord);
        // if self.chunks.contains_key(&key) {return}
        let chunk = WorldChunk::new(chunk_coord, self.chunk_scale, self.chunk_sample_scale, self.chunk_degree, &self.distance_field);
        self.chunks.insert(key, chunk);
    }

    pub fn get_neighbor_chunks(&self, chunk_coord: IVec3, dirs: &[IVec3]) -> Vec<Option<&WorldChunk>> {
        let mut ret = vec![None; dirs.len()];
        for i in 0 .. dirs.len() {
            let chunk_key = self.chunk_coord2key(chunk_coord + dirs[i]);
            ret[i] = self.chunks.get(&chunk_key);
        }
        ret
    }

    pub fn generate_chunks(&mut self, cur_chunk: IVec3)
    {
        let mut do_generation = false;
        // check for non-visible chunks
        let coords = Self::nearby_coords(cur_chunk, self.view_dist);
        for c in coords.iter() {
            let key = self.chunk_coord2key(*c);
            if ! self.operation_pending.contains(&key)
                && ! self.chunks.contains_key(&key)
            {
                do_generation = true;
                break;
            }
        }
        if do_generation {
            let coords = Self::nearby_coords(cur_chunk, self.gen_dist);
            for c in coords.iter() {
                let key = self.chunk_coord2key(*c);
                if ! self.operation_pending.contains(&key)
                    && ! self.chunks.contains_key(&key)
                {
                    self.operation_pending.insert(key);
                    self.queue_chunk.push(*c);
                }
            }
        }

        self.chunk_updated.clear();
        let mut chunk_updated_list = vec![];
        for i in 0 .. self.operations_per_frame {
            if ! self.queue_chunk.is_empty()
            {
                let c = self.queue_chunk.pop().unwrap();
                self.create_chunk(c);
                // regen surrounding sfp+mesh
                for dir in IDirection::NEGATIVE_DIRS
                {
                    let cc = c + *dir;
                    let key = self.chunk_coord2key(cc);
                    if self.chunks.contains_key(&key)
                        && ! self.operation_pending.contains(&key)
                    {
                        self.operation_pending.insert(key);
                        self.queue_sfp.push(cc);
                    }
                }
                // c has op pending
                self.queue_sfp.push(c);
            }
            else if ! self.queue_sfp.is_empty()
            {
                let c = self.queue_sfp.pop().unwrap();
                self.create_surface_map(c);
                // regen surrounding mesh
                for dir in IDirection::POSITIVE_DIRS
                {
                    let cc = c + *dir;
                    let key = self.chunk_coord2key(cc);
                    if self.surface_maps.contains_key(&key)
                        && ! self.operation_pending.contains(&key)
                    {
                        self.operation_pending.insert(key);
                        self.queue_mesh.push(cc);
                    }
                }
                // c has op pending
                self.queue_mesh.push(c);
            }
            else if ! self.queue_mesh.is_empty()
            {
                let c = self.queue_mesh.pop().unwrap();
                self.create_mesh(c);
                let key = self.chunk_coord2key(c);
                self.operation_pending.remove(&key);
                self.chunk_updated.insert(key);
                chunk_updated_list.push(key);
            }
        }
    }

    // assume chunk exists
    pub fn create_surface_map(&mut self, chunk_coord: IVec3)
    {
        let chunk_key = self.chunk_coord2key(chunk_coord);
        let neighbors = &self.get_neighbor_chunks(chunk_coord, IDirection::POSITIVE_DIRS);
        let chunk = self.chunks.get(&chunk_key).unwrap();
        let chunk_size = 1 << chunk.degree;
        let mut dd = [(0.0, 0.0); 12];
        let mut signs = [false; 12];
        let mut sptree = SurfaceOctree::new(chunk.degree);
        for k in 0 .. chunk_size
        {
            for j in 0 .. chunk_size
            {
                for i in 0 .. chunk_size
                {
                    let coord = ivec3(i, j, k);
                    let dists = chunk.neighbor_dist(coord, IDirection::POSITIVE_DIRS, neighbors);
                    let mut acc = 0;
                    for d in 0 .. 12 {
                        let ei = IDirection::EDGE_INDS[d];
                        let (d0, d1) = (dists[ei.0], dists[ei.1]);
                        dd[d] = (d0, d1);
                        signs[d] = is_intersection(d0, d1);
                        if signs[d] {acc += 1;}
                    }
                    if acc > 0 {
                        let mut r = dvec3(0.0, 0.0, 0.0);
                        for s in 0 .. 12 {
                            let ratio = dd[s].0 / (dd[s].0 - dd[s].1);
                            if signs[s] {
                                r += DDirection::EDGE_PAIRS[s].0
                                   + DDirection::EDGE_PAIRS[s].1 * ratio;
                            }
                        }
                        // generate normal with grad decent
                        let (x, y, z) = (
                            IDirection::EDGE_INDS_X,
                            IDirection::EDGE_INDS_Y,
                            IDirection::EDGE_INDS_Z,
                        );
                        let normal = dvec3(
                            dists[x[0].0] - dists[x[0].1]
                          + dists[x[1].0] - dists[x[1].1]
                          + dists[x[2].0] - dists[x[2].1]
                          + dists[x[3].0] - dists[x[3].1],
                            dists[y[0].0] - dists[y[0].1]
                          + dists[y[1].0] - dists[y[1].1]
                          + dists[y[2].0] - dists[y[2].1]
                          + dists[y[3].0] - dists[y[3].1],
                            dists[z[0].0] - dists[z[0].1]
                          + dists[z[1].0] - dists[z[1].1]
                          + dists[z[2].0] - dists[z[2].1]
                          + dists[z[3].0] - dists[z[3].1],
                        );
                        let sp = SurfacePoint{
                            position: r / acc as f64,
                            normal: normal.normalize(),
                        };
                        let loc = chunk.coord2loc(coord);
                        sptree.insert_value(loc, sp);
                    }
                }
            }
        }
        self.surface_maps.insert(chunk_key, sptree);
    }

    // assume chunk exists
    pub fn has_surface_map(&self, chunk_coord: IVec3) -> bool {
        let chunk_key = self.chunk_coord2key(chunk_coord);
        self.surface_maps.contains_key(&chunk_key)
    }

    // assume exists
    pub fn get_surface_map(&self, chunk_coord: IVec3) -> &SurfaceOctree {
        let chunk_key = self.chunk_coord2key(chunk_coord);
        self.surface_maps.get(&chunk_key).as_ref().unwrap()
    }

    // // relative coord (BDL at origin)
    pub fn get_sfp_by_coord(&self, chunk: &WorldChunk, coord: IVec3) -> Option<SurfacePoint> {
        if ! self.has_surface_map(chunk.coord) {return None;}
        let map = self.get_surface_map(chunk.coord);
        let loc = chunk.coord2loc(coord);
        let node = map.get_node_option(loc);
        match node {
            None => None,
            Some(n) => Some(n.value)
        }
    }

    // // (chunk coord, coord, surface point)
    pub fn get_neighbor_sfp(&self, chunk: &WorldChunk, coord: IVec3, dirs: &[IVec3], neighbors: &Vec<Option<&WorldChunk>>) -> Vec<Option<(IVec3, IVec3, SurfacePoint)>>
    {
        let mut sfps = vec![None; dirs.len() as usize];
        let n_coords = chunk.neighbor_coords(coord, dirs);
        for i in 0 .. n_coords.len()
        {
            let dir_ind = n_coords[i].0;
            let n_coord = n_coords[i].1;
            let n_chunk = neighbors[dir_ind];
            if ! n_chunk.is_none() {
                let n_chunk = n_chunk.unwrap();
                if self.has_surface_map(n_chunk.coord) {
                    let v = self.get_sfp_by_coord(&n_chunk, n_coord);
                    match v {
                        None => {continue;}
                        Some(v) => {
                            sfps[i] = Some((
                                n_chunk.coord * (1 << n_chunk.degree),
                                n_coord,
                                SurfacePoint{
                                    position: v.position + to_dvec3(coord + dirs[i]),
                                    normal:   v.normal,
                                }
                            ));
                        }
                    }
                }
            }
        }
        sfps
    }


    // (chunk coord, coord, surface point)
    pub fn create_mesh(&mut self, chunk_coord: IVec3) {
        let chunk_key = self.chunk_coord2key(chunk_coord);
        if ! self.surface_maps.contains_key(&chunk_key) {return}
        let chunk = self.chunks.get(&chunk_key).unwrap();
        let neighbors = &self.get_neighbor_chunks(chunk_coord, IDirection::UNIT_DIRS);
        let n_neighbors = &self.get_neighbor_chunks(chunk_coord, IDirection::NEGATIVE_DIRS);
        let mut mesh = IndexedMesh::new();
        let mesh_position = to_dvec3(chunk.coord * (1 << chunk.degree));
        for loc in self.get_surface_map(chunk_coord).keys()
        {
            let coord = chunk.loc2coord(*loc);
            let dists = chunk.neighbor_dist(coord, IDirection::UNIT_DIRS, neighbors);
            let sfps = self.get_neighbor_sfp(&chunk, coord, IDirection::NEGATIVE_DIRS, n_neighbors);
            // create mesh positions
            for s in 0 .. 3 {
                let (i, j, k) = IDirection::SFP_INDS[s];
                let (s0, s1, s2, s3) = (sfps[0], sfps[i], sfps[j], sfps[k]);
                if is_intersection(dists[0], dists[s+1])
                    && !s1.is_none()
                    && !s2.is_none()
                    && !s3.is_none()
                {
                    let (mut z, mut a, mut b, mut c) = (s0.unwrap(), s1.unwrap(), s2.unwrap(), s3.unwrap());
                    z.2.position += mesh_position;
                    z.2.position *= self.chunk_scale;
                    a.2.position += mesh_position;
                    a.2.position *= self.chunk_scale;
                    b.2.position += mesh_position;
                    b.2.position *= self.chunk_scale;
                    c.2.position += mesh_position;
                    c.2.position *= self.chunk_scale;
                    if dists[s+1] > dists[0] {
                        mesh.add_positions(&[z, a, b, z, b, c]);
                    }
                    else {
                        mesh.add_positions(&[z, b, a, z, c, b]);
                    }
                }
            }
        }
        self.meshes.insert(chunk_key, mesh);
    }

    pub fn nearby_coords(orig: IVec3, dist: i32) -> Vec<IVec3>
    {
        let s = (2 * dist + 1) * (2 * dist + 1) * (2 * dist + 1);
        let mut coords: Vec<IVec3> = Vec::with_capacity(s as usize);
        // for k in -dist ..= dist {
        //     for j in -dist ..= dist {
        //         for i in -dist ..= dist {
        //             coords.push(orig + ivec3(i, j, k));
        //         }
        //     }
        // }

        // radial loading
        for k in 0 ..= dist { // phi length
            //forward and back planes
            for j in 0 ..= k { // theta length
                for i in -j ..= j { //horizontal strip
                    coords.push(orig + ivec3(i, j, k));
                    coords.push(orig + ivec3(i, -j, k));
                    coords.push(orig + ivec3(i, j, -k));
                    coords.push(orig + ivec3(i, -j, -k));
                }
                if j != 0 {
                    let j = j - 1;
                    for i in -j ..= j { //vertical strip
                        coords.push(orig + ivec3(j, i, k));
                        coords.push(orig + ivec3(-j, i, k));
                        coords.push(orig + ivec3(j, i, -k));
                        coords.push(orig + ivec3(-j, i, -k));
                    }
                }
            }
            //peripheral cylinder
            if k != 0 {
                let k2 = k - 1;
                for j in -k2 ..= k2 {// cylinder h coord
                    for i in -k ..= k { // horizontal strip
                        coords.push(orig + ivec3(i, k, j));
                        coords.push(orig + ivec3(i, -k, j));
                    }
                    for i in -k2 ..= k2 { // vertical strip
                        coords.push(orig + ivec3(k2, i, j));
                        coords.push(orig + ivec3(-k2, i, j));
                    }
                }
            }
        }

        coords
    }

    pub fn visible_meshes(&self, cur_chunk: IVec3) -> Vec<(SeaHashKey, &IndexedMesh)>
    {
        let coords = Self::nearby_coords(cur_chunk, self.view_dist);
        let mut ret = Vec::with_capacity(10 * self.view_dist as usize);
        for c in coords.iter()
        {
            let key = &self.chunk_coord2key(*c);
            if self.meshes.contains_key(key)
            {
                ret.push((*key, self.meshes.get(key).unwrap()))
            }
        }
        ret
    }

    pub fn chunk_borders(&mut self, cur_chunk: IVec3) -> Vec<Mesh>
    {
        let coords = Self::nearby_coords(cur_chunk, self.view_dist);
        let mut ret = Vec::with_capacity(10 * (self.view_dist + 1) as usize);
        for c in coords.iter()
        {
            let key = self.chunk_coord2key(cur_chunk + *c);
            if self.chunks.contains_key(&key)
            {
                let mut tris = Vec::with_capacity(12);
                for e in IDirection::EDGE_PAIRS
                {
                    let p0 = to_dvec3((e.0) * self.chunk_size) * self.chunk_scale;
                    let p1 = to_dvec3((e.0 + e.1) * self.chunk_size) * self.chunk_scale;
                    let mut tri = Triangle::from_dvec3(p0, p1, p0);
                    tri.is_line = true;
                    tris.push(tri);
                }
                let mut mesh = Mesh::new(tris);
                mesh.position = to_dvec3(*c * self.chunk_size) * self.chunk_scale;
                ret.push(mesh);
            }
        }
        ret
    }

}

