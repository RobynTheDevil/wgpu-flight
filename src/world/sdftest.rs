use crate::{
    math::*,
    player::Player,
};
use super::{*, chunk::*};

pub struct SdfWorld
{
    pub chunks: ChunkManager,
    pub coord_cur: IVec3,
    pub coord_last: IVec3,
}

impl World for SdfWorld {
    fn new() -> Self where Self: Sized {
        Self {
            chunks: ChunkManager::new(),
            coord_cur: ivec3(0, 0, 0),
            coord_last: ivec3(-1, 0, 0),
        }
    }

    fn initialize(&mut self) {
        self.chunks.generate_chunks(ivec3(0, 0, 0))
    }

    fn update(&mut self, player: &Player) {
        self.coord_cur = pos2chunk(player.get_position(), self.chunks.chunk_size);
        if self.coord_cur != self.coord_last {
            println!("chunk {} {} {}", self.coord_cur.x, self.coord_cur.y, self.coord_cur.z);
        }
        self.chunks.generate_chunks(self.coord_cur);
        self.coord_last = self.coord_cur;
    }

    fn get_meshes(&self) -> (Vec<(SeaHashKey, &IndexedMesh)>, &SeaHashSet<SeaHashKey>) {
        let visible = self.chunks.visible_meshes(self.coord_cur);
        let updated = &self.chunks.chunk_updated;
        (visible, updated)
    }

    fn get_data(&self) -> Vec<u8> {
        Vec::<u8>::new()
    }

}

