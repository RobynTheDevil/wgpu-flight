use nohash_hasher::IntMap;
use glam::*;
use crate::hasher::*;

// Octree -- linearly hashed octree with locational codes

enum OctPos {
    BDL = 0b000,
    BDR = 0b001,
    BUL = 0b010,
    BUR = 0b011,
    FDL = 0b100,
    FDR = 0b101,
    FUL = 0b110,
    FUR = 0b111,

}

pub struct OctreeNode<T> {
    // location: u64,
    pub mask: u8,
    pub value: T,
}

pub struct Octree<T> {
    pub depth: u8, // max 21
    pub values: IntMap<u64, OctreeNode<T>>,
}

impl<T> Octree<T> {

    pub fn new(depth: u8) -> Self {
        let values = Default::default();
        Self {
            depth,
            values,
        }
    }

    pub fn get(&self, loc: &u64) -> Option<&OctreeNode<T>> { self.values.get(loc) }

    pub fn get_mut(&mut self, loc: &u64) -> Option<&mut OctreeNode<T>> { self.values.get_mut(loc) }

    // assume exists (child mask of parent is checked before calling)
    pub fn get_node(&self, loc: &u64) -> &OctreeNode<T> {
        self.get(loc).unwrap()
    }

    // assume exists (value of 0b1, root node, is checked before callling)
    pub fn get_parent(&self, loc: u64) -> &OctreeNode<T> {
        let l = loc >> 3;
        self.get(&l).unwrap()
    }

    // assume exists (value of 0b1, root node, is checked before callling)
    pub fn get_parent_mut(&mut self, loc: u64) -> &mut OctreeNode<T> {
        let l = loc >> 3;
        self.get_mut(&l).unwrap()
    }

    pub fn insert(&mut self, loc: u64, node: OctreeNode<T>) {
        self.values.insert(loc, node);
    }

    pub fn insert_value(&mut self, loc: u64, value: T) {
        self.insert(loc, OctreeNode::<T>{mask: 0, value});
    }

    pub fn get_node_or_parent(&self, loc: u64) -> &OctreeNode<T> {
        let n = self.values.get(&loc);
        match n {
            Some(n) => n,
            None => {
                // println!("p {:016b}", loc);
                if loc == 0 {panic!()}
                self.get_node_or_parent(loc >> 3)
            }
        }
    }

    pub fn get_node_option(&self, loc: u64) -> Option<&OctreeNode<T>> {
        self.values.get(&loc)
    }

    // include current node as first element
    // assume exists (child mask of parent is checked before calling)
    pub fn get_children(&self, loc: u64) -> Vec<&OctreeNode<T>> {
        let cur_node = self.get_node(&loc);
        let mut ret = Vec::with_capacity(9);
        ret.push(cur_node);
        if cur_node.mask > 0 {
            for i in 0 .. 8 {
                if cur_node.mask & (1 << i) > 0 {
                    let loc = (loc << 3) | i;
                    ret.push(
                        self.get_node(&loc)
                    );
                }
            }
        }
        ret
    }

    pub fn contains_key(&self, loc: &u64) -> bool {self.values.contains_key(loc)}

    pub fn is_empty(&self) -> bool {self.values.is_empty()}

    pub fn keys(&self) -> std::collections::hash_map::Keys<'_, u64, OctreeNode<T>> {self.values.keys()}

}

pub type SDFNode = OctreeNode<u8>;
pub type SDFOctree = Octree<u8>;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SurfacePoint{
    pub position: DVec3,
    pub normal: DVec3
}

pub type SurfaceMap = SeaHashMap<SeaHashKey, SurfacePoint>;

pub type SurfaceNode = OctreeNode<SurfacePoint>;
pub type SurfaceOctree = Octree<SurfacePoint>;


