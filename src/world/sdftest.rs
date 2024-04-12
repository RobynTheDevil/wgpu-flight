use super::{*};

pub struct SdfWorld
{
}

impl World for SdfWorld {
    fn new() -> Self where Self: Sized {
        Self {}
    }

    fn initialize(&mut self) {
    }

    fn update(&mut self, player: &Player) {
    }

    fn get_data(&self) -> Vec<u8> {
        Vec::<u8>::new()
    }

}

